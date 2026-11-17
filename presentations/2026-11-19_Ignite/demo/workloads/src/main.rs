//! The AI service: the application (gateway, retries, telemetry), its simulated external
//! systems (three model deployments, their health monitor, the change-approval tool and the
//! gateway configuration API), and the demo driver (traffic, controls, reset). It reads no
//! Drasi results: every write is a simulation, a demo control, or a call it received.
mod content;
#[cfg(test)]
mod db_tests;
mod external;
mod model;
mod scenario;
mod telemetry;
mod traffic;

use anyhow::{Context, Result};
use axum::{
    extract::{FromRequest, Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use model::port;
use opentelemetry::{
    trace::{Span, Status},
    KeyValue,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

macro_rules! ensure {
    ($condition:expr, $message:literal) => {
        if !$condition {
            return Err(anyhow::anyhow!($message).into());
        }
    };
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis() as i64
}
fn identity() -> String {
    Uuid::new_v4().to_string()
}
async fn db() -> Result<Client> {
    let (client, connection) =
        tokio_postgres::connect(&std::env::var("DATABASE_URL")?, NoTls).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("PostgreSQL connection failed: {error}");
        }
    });
    Ok(client)
}
#[derive(Clone)]
struct App {
    http: reqwest::Client,
    telemetry: Arc<telemetry::Telemetry>,
    traffic: Arc<tokio::sync::Mutex<traffic::Traffic>>,
}

/// An API error: 400 malformed (permanent), 404 unknown, 409 rejected demo control,
/// 503 database unavailable (callers such as Drasi's HTTP sink retry it).
struct Error {
    status: StatusCode,
    error: anyhow::Error,
}
impl Error {
    fn with(status: StatusCode, message: &str) -> Self {
        Self {
            status,
            error: anyhow::anyhow!(message.to_owned()),
        }
    }
    fn bad_request(message: &str) -> Self {
        Self::with(StatusCode::BAD_REQUEST, message)
    }
    fn not_found(message: &str) -> Self {
        Self::with(StatusCode::NOT_FOUND, message)
    }
    fn conflict(message: &str) -> Self {
        Self::with(StatusCode::CONFLICT, message)
    }
    fn bad(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error,
        }
    }
}
/// A database that cannot be reached (as opposed to one that refused a statement).
fn unavailable(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<tokio_postgres::Error>()
            .is_some_and(|e| match e.as_db_error() {
                None => {
                    e.is_closed()
                        || std::error::Error::source(e).is_some_and(|s| s.is::<std::io::Error>())
                }
                Some(db) => ["08", "57P"].iter().any(|c| db.code().code().starts_with(c)),
            })
    })
}
impl<E: Into<anyhow::Error>> From<E> for Error {
    fn from(e: E) -> Self {
        let error = e.into();
        Self {
            status: if unavailable(&error) {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::CONFLICT
            },
            error,
        }
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        eprintln!("Request failed ({}): {:#}", self.status, self.error);
        (self.status, Json(json!({"error":self.error.to_string()}))).into_response()
    }
}
type Api = std::result::Result<Json<Value>, Error>;

/// A JSON body; anything malformed is a permanent 400, so retrying callers stop.
struct Payload<T>(T);
impl<S: Send + Sync, T: DeserializeOwned> FromRequest<S> for Payload<T> {
    type Rejection = Error;
    async fn from_request(request: Request, state: &S) -> std::result::Result<Self, Error> {
        match Json::<T>::from_request(request, state).await {
            Ok(Json(value)) => Ok(Payload(value)),
            Err(rejection) => Err(Error::bad_request(&rejection.body_text())),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Submit {
    id: String,
    run_epoch: String,
    workload: String,
    content: String,
    #[serde(default)]
    destination: Option<String>,
}

async fn submit(State(app): State<App>, Payload(input): Payload<Submit>) -> Api {
    ensure!(
        !input.id.is_empty()
            && input.id.len() <= 100
            && !input.content.is_empty()
            && input.content.len() <= 16000,
        "invalid task content or identity"
    );
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock_shared(190619)")
        .await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190621)")
        .await?;
    let scenario = scenario::load(&tx).await?;
    let epoch = scenario.run_epoch.clone();
    ensure!(input.run_epoch == epoch, "stale submission epoch");
    let route = scenario.route(&input.workload)?;
    if let Some(destination) = &input.destination {
        ensure!(destination == route, "destination is not the configured route");
    }
    if let Some(row) = tx
        .query_opt(
            "SELECT workload, content, trace_id, run_epoch, root_span_id FROM ignite_tasks WHERE id=$1",
            &[&input.id],
        )
        .await?
    {
        ensure!(
            row.get::<_, String>(0) == input.workload
                && row.get::<_, String>(1) == input.content
                && row.get::<_, String>(3) == epoch,
            "idempotency key reused for different content or epoch"
        );
        return Ok(Json(
            json!({"id":input.id,"trace_id":row.get::<_,String>(2),"root_span_id":row.get::<_,String>(4),"duplicate":true}),
        ));
    }
    let pending: i64 = tx.query_one(
        "SELECT count(*) FROM ignite_tasks WHERE run_epoch=$1 AND workload=$2 AND completed_ms IS NULL AND cancelled_ms IS NULL",
        &[&epoch,&input.workload],
    ).await?.get(0);
    ensure!(pending < 200, "workload queue admission limit reached");
    let mut span = app.telemetry.root(&input.id, &input.workload);
    let trace = span.span_context().trace_id().to_string();
    let root_span = span.span_context().span_id().to_string();
    let created = now();
    let deadline = created + 15_000;
    tx.execute("INSERT INTO ignite_tasks(id,run_epoch,workload,content,trace_id,created_ms,deadline_ms,root_span_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
        &[&input.id,&epoch,&input.workload,&input.content,&trace,&created,&deadline,&root_span]).await?;
    tx.commit().await?;
    span.set_attributes([
        KeyValue::new("run.epoch", epoch),
        KeyValue::new("langfuse.observation.input", input.content.clone()),
        KeyValue::new("langfuse.trace.input", input.content.clone()),
    ]);
    span.end();
    Ok(Json(
        json!({"id":input.id,"trace_id":trace,"root_span_id":root_span,"deadline_ms":deadline,
        "trace_url":format!("http://localhost:{}/project/ignite-probe/traces/{trace}", ignite_settings::get().port("LANGFUSE")?)}),
    ))
}

async fn task(Path(id): Path<String>) -> Api {
    let client = db().await?;
    let row = client
        .query_one("SELECT to_jsonb(t) FROM ignite_tasks t WHERE id=$1", &[&id])
        .await?;
    let attempts: Vec<Value> = client
        .query(
            "SELECT to_jsonb(a) FROM ignite_attempts a WHERE task_id=$1 ORDER BY started_ms",
            &[&id],
        )
        .await?
        .iter()
        .map(|r| r.get(0))
        .collect();
    Ok(Json(
        json!({"task":row.get::<_,Value>(0),"attempts":attempts,
            "trace_url":format!("http://localhost:{}/project/ignite-probe/traces/{}",ignite_settings::get().port("LANGFUSE")?,row.get::<_,Value>(0)["trace_id"].as_str().context("trace")?)}),
    ))
}

#[derive(Deserialize, Serialize)]
struct Inference {
    task_id: String,
    run_epoch: String,
    workload: String,
    content: String,
    trace_id: String,
    root_span_id: String,
    model_revision: i64,
}
#[derive(Clone)]
struct EndpointApp {
    app: App,
    id: String,
}

/// A simulated deployment's own health endpoint, polled by the simulated health monitor.
async fn endpoint_health(State(state): State<EndpointApp>) -> Api {
    let client = db().await?;
    let (epoch, _) = scenario::current(&client).await?;
    let row = client
        .query_one(
            "SELECT mode, reporting FROM ignite_simulation WHERE deployment=$1",
            &[&state.id],
        )
        .await?;
    Ok(Json(
        json!({"run_epoch":epoch,"id":state.id,"status":row.get::<_,String>(0),"reporting":row.get::<_,bool>(1)}),
    ))
}

/// A simulated model deployment: answers, or fails (503) or throttles (429) per its fault mode
/// and token quota. It knows nothing of routes, policy or plans.
async fn infer(
    State(state): State<EndpointApp>,
    Payload(input): Payload<Inference>,
) -> std::result::Result<Response, Error> {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    ensure!(
        tx.query_one("SELECT pg_try_advisory_xact_lock_shared(190619)", &[])
            .await?
            .get::<_, bool>(0),
        "control or reset pending; retry admission"
    );
    let scenario = scenario::load(&tx).await?;
    let epoch = scenario.run_epoch.clone();
    ensure!(epoch == input.run_epoch, "stale run epoch");
    let e = scenario.deployment(&state.id)?;
    tx.query_one(
        "SELECT pg_advisory_xact_lock($1)",
        &[&(190700i64 + i64::from(port(&state.id)?))],
    )
    .await?;
    let mut span = state.app.telemetry.span(
        format!("model · {}", state.id),
        &input.trace_id,
        &input.root_span_id,
        &input.task_id,
        &input.workload,
    )?;
    span.set_attributes([
        KeyValue::new("endpoint.id", state.id.clone()),
        KeyValue::new("langfuse.observation.type", "generation"),
        KeyValue::new("gen_ai.request.model", e.model.clone()),
        KeyValue::new("langfuse.observation.input", input.content.clone()),
        KeyValue::new("langfuse.observation.metadata.deployment", state.id.clone()),
        KeyValue::new("langfuse.observation.metadata.region", e.region.clone()),
    ]);
    if let Some(row) = tx
        .query_opt(
            "SELECT response,tokens FROM ignite_receipts WHERE id=$1",
            &[&input.task_id],
        )
        .await?
    {
        return Ok(Json(json!({"response":row.get::<_,String>(0),"tokens":row.get::<_,i64>(1),"duplicate":true})).into_response());
    }
    let response = content::answer(&input.workload, &input.content);
    // Whitespace token units and fixed responses are explicitly simulated, never GPU measurements.
    let tokens =
        (input.content.split_whitespace().count() + response.split_whitespace().count()) as i64;
    let used: i64 = tx.query_one("SELECT COALESCE(sum(tokens),0)::bigint FROM ignite_receipts WHERE endpoint=$1 AND run_epoch=$2 AND received_ms>$3",
        &[&state.id,&epoch,&(now()-60_000)]).await?.get(0);
    let status = if e.mode == "failed" {
        503
    } else if e.mode == "throttled" || used + tokens > e.capacity_tpm {
        429
    } else {
        200
    };
    span.set_attribute(KeyValue::new("http.response.status_code", status as i64));
    if status != 200 {
        let reason = if status == 503 {
            format!("Deployment {} failed (HTTP 503)", state.id)
        } else {
            format!("Deployment {} is over its token quota (HTTP 429)", state.id)
        };
        span.set_attributes([
            KeyValue::new("langfuse.observation.level", "ERROR"),
            KeyValue::new("langfuse.observation.status_message", reason.clone()),
            KeyValue::new("langfuse.observation.output", reason.clone()),
        ]);
        span.set_status(Status::error(reason));
        return Ok((
            StatusCode::from_u16(status)?,
            Json(json!({"error":e.mode,"endpoint":e.id})),
        )
            .into_response());
    }
    tx.execute("INSERT INTO ignite_receipts(id,task_id,run_epoch,endpoint,workload,content,response,tokens,received_ms,model_revision) VALUES($1,$1,$2,$3,$4,$5,$6,$7,$8,$9)",
        &[&input.task_id,&epoch,&state.id,&input.workload,&input.content,&response,&tokens,&now(),&input.model_revision]).await?;
    tx.commit().await?;
    span.set_attribute(KeyValue::new(
        "gen_ai.usage.input_tokens",
        input.content.split_whitespace().count() as i64,
    ));
    span.set_attribute(KeyValue::new(
        "gen_ai.usage.output_tokens",
        response.split_whitespace().count() as i64,
    ));
    span.set_attribute(KeyValue::new(
        "langfuse.observation.output",
        response.clone(),
    ));
    Ok(Json(json!({"response":response,"tokens":tokens})).into_response())
}

async fn dispatch_once(app: &App, workload: &str) -> Result<()> {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock_shared(190619)")
        .await?;
    let scenario = scenario::load(&tx).await?;
    let (epoch, revision) = (scenario.run_epoch.clone(), scenario.revision);
    let pending: i64 = tx.query_one("SELECT count(*) FROM ignite_tasks WHERE run_epoch=$1 AND workload=$2 AND completed_ms IS NULL", &[&epoch,&workload]).await?.get(0);
    app.telemetry.queue.record(
        pending as u64,
        &[KeyValue::new("workload", workload.to_owned())],
    );
    // Untried requests first, then the least recently tried, so a failing deployment doesn't
    // leave every request behind the oldest one with no attempt in its trace.
    let Some(row) = tx.query_opt("SELECT t.id,t.content,t.trace_id,t.root_span_id FROM ignite_tasks t WHERE t.run_epoch=$1 AND t.workload=$2 AND t.completed_ms IS NULL AND t.cancelled_ms IS NULL ORDER BY (SELECT max(a.started_ms) FROM ignite_attempts a WHERE a.task_id=t.id) NULLS FIRST,t.created_ms,t.id LIMIT 1", &[&epoch,&workload]).await? else { return Ok(()) };
    let id: String = row.get(0);
    let trace: String = row.get(2);
    let root_span: String = row.get(3);
    // Durable receipts reconcile an HTTP success whose gateway acknowledgement was lost.
    if let Some(receipt) = tx
        .query_opt(
            "SELECT r.endpoint,r.response,r.received_ms,r.model_revision,t.created_ms FROM ignite_receipts r JOIN ignite_tasks t ON t.id=r.task_id WHERE r.task_id=$1",
            &[&id],
        )
        .await?
    {
        tx.execute("UPDATE ignite_tasks SET completed_ms=$1,endpoint=$2,outcome=$3,revision=revision+1,completed_revision=$5 WHERE id=$4 AND completed_ms IS NULL",
            &[&receipt.get::<_,i64>(2),&receipt.get::<_,String>(0),&receipt.get::<_,String>(1),&id,&receipt.get::<_,Option<i64>>(3)]).await?;
        tx.commit().await?;
        app.telemetry
            .completed
            .add(1, &[KeyValue::new("workload", workload.to_owned())]);
        app.telemetry.answer.record(
            (receipt.get::<_, i64>(2) - receipt.get::<_, i64>(4)).max(0) as f64 / 1000.,
            &[KeyValue::new("workload", workload.to_owned())],
        );
        let mut done = app
            .telemetry
            .span("task.completed", &trace, &root_span, &id, workload)?;
        let answer: String = receipt.get(1);
        let deployment: String = receipt.get(0);
        done.set_attributes([
            KeyValue::new("langfuse.observation.output", answer.clone()),
            KeyValue::new("langfuse.trace.output", answer),
            KeyValue::new("langfuse.observation.metadata.answered_by", deployment),
        ]);
        done.end();
        return Ok(());
    }
    // The gateway sends to the configured route; Drasi decides routes, the gateway only follows them.
    let deployment = scenario.route(workload)?.to_owned();
    let attempt = identity();
    let started = now();
    let count: i64 = tx
        .query_one(
            "SELECT count(*) FROM ignite_attempts WHERE task_id=$1",
            &[&id],
        )
        .await?
        .get(0);
    tx.execute("INSERT INTO ignite_attempts(id,task_id,run_epoch,workload,endpoint,model_revision,started_ms,trace_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
        &[&attempt,&id,&epoch,&workload,&deployment,&revision,&started,&trace]).await?;
    tx.commit().await?;
    // The shared reset/control lock is held from the final basis check through HTTP completion.
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock_shared(190619)")
        .await?;
    let (current_epoch, current_revision) = scenario::current(&tx).await?;
    if current_epoch != epoch || current_revision != revision {
        tx.execute("UPDATE ignite_attempts SET finished_ms=$1,status=409,detail='route configuration changed before transmission' WHERE id=$2", &[&now(),&attempt]).await?;
        tx.commit().await?;
        return Ok(());
    }
    let mut span = app.telemetry.span(
        format!("attempt · {}", deployment),
        &trace,
        &root_span,
        &id,
        workload,
    )?;
    span.set_attributes([
        KeyValue::new(
            "langfuse.observation.input",
            format!("Send to {}, attempt {}", deployment, count + 1),
        ),
        KeyValue::new("endpoint.id", deployment.clone()),
        KeyValue::new("attempt.id", attempt.clone()),
        KeyValue::new("langfuse.observation.metadata.deployment", deployment.clone()),
        KeyValue::new(
            "langfuse.observation.metadata.attempt",
            (count + 1).to_string(),
        ),
    ]);
    let response = app
        .http
        .post(format!("http://127.0.0.1:{}/infer", port(&deployment)?))
        .json(&Inference {
            task_id: id.clone(),
            run_epoch: epoch,
            workload: workload.into(),
            content: row.get(1),
            trace_id: trace,
            root_span_id: root_span,
            model_revision: revision,
        })
        .send()
        .await;
    let (status, detail, tokens) = match response {
        Ok(response) => {
            let status = response.status().as_u16() as i32;
            let value: Value = response.json().await?;
            (
                status,
                value.to_string(),
                value.get("tokens").and_then(Value::as_i64).unwrap_or(0),
            )
        }
        Err(error) => (0, format!("transport outcome uncertain: {error}"), 0),
    };
    tx.execute(
        "UPDATE ignite_attempts SET finished_ms=$1,status=$2,detail=$3,tokens=$4 WHERE id=$5",
        &[&now(), &status, &detail, &tokens, &attempt],
    )
    .await?;
    tx.commit().await?;
    let labels = [
        KeyValue::new("workload", workload.to_owned()),
        KeyValue::new("endpoint", deployment.clone()),
    ];
    let mut status_labels = labels.to_vec();
    status_labels.push(KeyValue::new("status", status.to_string()));
    app.telemetry.requests.add(1, &status_labels);
    if count > 0 {
        app.telemetry.retries.add(1, &labels);
    }
    app.telemetry
        .duration
        .record((now() - started) as f64 / 1000., &labels);
    app.telemetry
        .failure
        .record(u64::from(status != 200), &labels);
    app.telemetry.tokens.add(tokens as u64, &labels);
    span.set_attribute(KeyValue::new("http.response.status_code", status as i64));
    if status != 200 {
        let message = match status {
            503 => format!("Deployment {} failed (HTTP 503); will retry", deployment),
            429 => format!(
                "Deployment {} is over its token quota (HTTP 429); will retry",
                deployment
            ),
            0 => format!("No response from deployment {}; will retry", deployment),
            other => format!("Deployment {} returned HTTP {other}", deployment),
        };
        span.set_attributes([
            KeyValue::new("langfuse.observation.level", "ERROR"),
            KeyValue::new("langfuse.observation.status_message", message.clone()),
            KeyValue::new("langfuse.observation.output", message),
        ]);
        span.set_status(Status::error(detail));
    } else {
        span.set_attribute(KeyValue::new("langfuse.observation.output", "Answered"));
    }
    Ok(())
}

/// The simulated health monitor: checks each deployment every second and records what it saw.
/// A deployment whose reporting is paused stays silent, so its last report expires.
async fn report_once(app: &App) -> Result<()> {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock_shared(190619)")
        .await?;
    let (epoch, _) = scenario::current(&tx).await?;
    for id in scenario::DEPLOYMENTS {
        let value: Value = app
            .http
            .get(format!("http://127.0.0.1:{}/health", port(id)?))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        ensure!(
            value["run_epoch"] == epoch && value["id"] == id,
            "mismatched report identity"
        );
        if value["reporting"] != true {
            continue;
        }
        let status = value["status"]
            .as_str()
            .context("missing deployment status")?;
        let observed = now();
        tx.execute("UPDATE ignite_reports SET observed_ms=$1,expires_ms=$2,status=$3,sequence=sequence+1 WHERE id=$4 AND run_epoch=$5",
            &[&observed,&(observed+8_000),&status,&id,&epoch]).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn client_command(workload: &str, count: usize) -> Result<()> {
    ensure!(
        ["support", "batch"].contains(&workload),
        "client must be support or batch"
    );
    let http = reqwest::Client::new();
    let state: Value = http
        .get(format!("{}/health", ignite_settings::get().url("WORKLOAD")?))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let epoch = state["run_epoch"].as_str().context("epoch")?;
    for _ in 0..count {
        let value: Value = http
            .post(format!("{}/tasks", ignite_settings::get().url("WORKLOAD")?))
            .json(&Submit {
                id: format!("{epoch}-{workload}-{}", identity()),
                run_epoch: epoch.into(),
                workload: workload.into(),
                destination: None,
                content: if workload == "support" {
                    "SYNTHETIC RESTRICTED EU customer needs account recovery assistance."
                } else {
                    "PUBLIC DOCUMENT Rust provides memory safety without garbage collection."
                }
                .into(),
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        println!("{value}");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("client") {
        return client_command(
            args.get(2).context("client support|batch [count]")?,
            args.get(3).map(|v| v.parse()).transpose()?.unwrap_or(1),
        )
        .await;
    }
    ensure!(
        args.get(1).map(String::as_str) == Some("serve"),
        "usage: ignite-workloads serve|client support|batch [count]"
    );
    let (owner, connection) =
        tokio_postgres::connect(&std::env::var("DATABASE_URL")?, NoTls).await?;
    let owner_connection = tokio::spawn(connection);
    ensure!(
        owner
            .query_one("SELECT pg_try_advisory_lock(190620)", &[])
            .await?
            .get::<_, bool>(0),
        "workload service already running"
    );
    scenario::load(&owner).await?;
    let app = App {
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(4))
            .build()?,
        telemetry: Arc::new(telemetry::Telemetry::new()?),
        traffic: Arc::new(tokio::sync::Mutex::new(traffic::Traffic::default())),
    };
    let mut jobs = tokio::task::JoinSet::<Result<()>>::new();
    jobs.spawn(async move {
        owner_connection.await??;
        anyhow::bail!("workload ownership connection ended")
    });
    for id in ["EU-A", "EU-B", "US-C"] {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port(id)?)).await?;
        let router = Router::new()
            .route("/health", get(endpoint_health))
            .route("/infer", post(infer))
            .with_state(EndpointApp {
                app: app.clone(),
                id: id.into(),
            });
        jobs.spawn(async move {
            axum::serve(listener, router).await?;
            Ok(())
        });
    }
    let settings = ignite_settings::get();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", settings.port("WORKLOAD")?)).await?;
    let router = Router::new()
        // Demo driver and the application.
        .route("/health", get(scenario::health))
        .route("/tasks", post(submit))
        .route("/tasks/{id}", get(task))
        .route("/traffic", post(traffic::control))
        .route("/control", post(scenario::control))
        .route("/configuration", post(scenario::configuration))
        .route("/reset", post(scenario::reset))
        // Change-approval tool and gateway configuration API (stand-ins for external systems).
        .route("/approvals", post(external::approve))
        .route("/routes/apply", post(external::apply))
        .route("/approvals/{id}/reject", post(external::reject))
        .route("/approvals/{id}/verified", post(external::verified))
        .with_state(app.clone());
    jobs.spawn(async move {
        axum::serve(listener, router).await?;
        Ok(())
    });
    let generator = app.clone();
    jobs.spawn(async move {
        loop {
            traffic::tick(&generator).await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    for workload in ["support", "batch"] {
        let app = app.clone();
        jobs.spawn(async move {
            // A transient error (database or simulator briefly unavailable) is retried on the
            // next pass instead of stopping the service.
            loop {
                if let Err(error) = dispatch_once(&app, workload).await {
                    eprintln!("{workload} dispatch failed, retrying: {error:#}");
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }
    let reports = app.clone();
    jobs.spawn(async move {
        // A missed report just ages; Drasi turns it Unknown if reports stay missing.
        loop {
            if let Err(error) = report_once(&reports).await {
                eprintln!("Health report failed, retrying: {error:#}");
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    println!(
        "Ignite workload gateway 127.0.0.1:{}; model simulators {}/{}/{}",
        settings.port("WORKLOAD")?,
        settings.deployment_port("EU-A")?,
        settings.deployment_port("EU-B")?,
        settings.deployment_port("US-C")?
    );
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let result = tokio::select! {
        signal = tokio::signal::ctrl_c() => { signal?; Ok(()) },
        _ = terminate.recv() => Ok(()),
        task = jobs.join_next() => { task.context("all workers exited")?? }
    };
    jobs.abort_all();
    while jobs.join_next().await.is_some() {}
    // A collector that is down must not hide why the service stopped.
    if let Err(error) = app.telemetry.shutdown() {
        eprintln!("Telemetry not flushed at shutdown: {error:#}");
    }
    if let Err(error) = &result {
        eprintln!("Workload service stopped: {error:#}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_unreachable_database_is_retriable() {
        let refused = Error::from(anyhow::anyhow!("stale reset basis"));
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(Error::bad_request("x").status, StatusCode::BAD_REQUEST);
        assert!(!unavailable(&anyhow::anyhow!("no database involved")));
    }

    #[tokio::test]
    async fn database_outage_answers_503() {
        let error = tokio_postgres::connect("postgresql://ignite@127.0.0.1:1/none", NoTls)
            .await
            .err()
            .expect("nothing listens on port 1");
        assert_eq!(Error::from(error).status, StatusCode::SERVICE_UNAVAILABLE);
    }
}
