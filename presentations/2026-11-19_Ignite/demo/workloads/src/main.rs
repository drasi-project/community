mod dashboard;
mod decisions;
mod model;
mod observer;
mod telemetry;
mod traffic;

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use model::{port, Model};
use opentelemetry::{
    trace::{Span, Status},
    KeyValue,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
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
    context_ready: Arc<AtomicU64>,
    context_rows: Arc<tokio::sync::RwLock<BTreeMap<String, BTreeMap<String, Value>>>>,
    traffic: Arc<tokio::sync::Mutex<traffic::Traffic>>,
}
struct Error(anyhow::Error);
impl<E: Into<anyhow::Error>> From<E> for Error {
    fn from(e: E) -> Self {
        Self(e.into())
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        eprintln!("Request failed: {:#}", self.0);
        (
            StatusCode::CONFLICT,
            Json(json!({"error":self.0.to_string()})),
        )
            .into_response()
    }
}
type Api = std::result::Result<Json<Value>, Error>;

async fn model(client: &impl tokio_postgres::GenericClient) -> Result<(String, i64, Model)> {
    let row = client
        .query_one(
            "SELECT run_epoch, revision, payload FROM ignite_model WHERE id='active'",
            &[],
        )
        .await?;
    let model: Model = serde_json::from_value(row.get(2))?;
    model.validate()?;
    Ok((row.get(0), row.get(1), model))
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

async fn submit(State(app): State<App>, Json(input): Json<Submit>) -> Api {
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
    let (epoch, _, m) = model(&tx).await?;
    ensure!(input.run_epoch == epoch, "stale submission epoch");
    ensure!(
        m.workloads.iter().any(|w| w.id == input.workload),
        "unknown workload"
    );
    if let Some(destination) = &input.destination {
        m.dispatch(&input.workload, destination)?;
        ensure!(
            m.workloads
                .iter()
                .any(|w| w.id == input.workload && &w.destination == destination),
            "destination is not the committed route"
        );
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
    span.set_attribute(KeyValue::new("run.epoch", epoch));
    span.end();
    Ok(Json(
        json!({"id":input.id,"trace_id":trace,"root_span_id":root_span,"deadline_ms":deadline,
        "trace_url":format!("http://localhost:13001/project/ignite-probe/traces/{trace}")}),
    ))
}

async fn health(State(app): State<App>) -> Api {
    let client = db().await?;
    let (epoch, revision, m) = model(&client).await?;
    let pending: i64 = client
        .query_one(
            "SELECT count(*) FROM ignite_tasks WHERE run_epoch=$1 AND completed_ms IS NULL",
            &[&epoch],
        )
        .await?
        .get(0);
    let rows = app.context_rows.read().await;
    let coherent = coherent_context(&rows, &epoch, revision)
        && app.context_ready.load(Ordering::Acquire) == decisions::STREAM_MASK;
    drop(rows);
    let traffic = app.traffic.lock().await.snapshot();
    Ok(Json(
        json!({"run_epoch":epoch,"revision":revision,"model":m,"pending":pending,
        "continuous_context":if coherent {"ready"} else {"unknown"},
        "streams_attached":app.context_ready.load(Ordering::Acquire)==decisions::STREAM_MASK,
        "task_target_seconds":15,"report_expiry_seconds":8,"traffic":traffic}),
    ))
}

fn coherent_context(
    rows: &BTreeMap<String, BTreeMap<String, Value>>,
    epoch: &str,
    revision: i64,
) -> bool {
    let Some(model) = rows.get("model-context").and_then(|r| r.get("active")) else {
        return false;
    };
    if model["run_epoch"] != epoch || model["revision"] != revision {
        return false;
    }
    let Some(reports) = rows.get("endpoint-freshness") else {
        return false;
    };
    reports.len() == 3
        && ["EU-A", "EU-B", "US-C"].iter().all(|id| {
            reports.get(*id).is_some_and(|r| {
                r["run_epoch"] == epoch
                    && r["status"] != "unknown"
                    && r["expires_ms"]
                        .as_i64()
                        .is_some_and(|expires| expires > now())
            })
        })
}

async fn context(State(app): State<App>) -> Json<Value> {
    Json(
        json!({"streams_attached":app.context_ready.load(Ordering::Acquire)==decisions::STREAM_MASK,
        "queries":*app.context_rows.read().await}),
    )
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
            "trace_url":format!("http://localhost:13001/project/ignite-probe/traces/{}",row.get::<_,Value>(0)["trace_id"].as_str().context("trace")?)}),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    run_epoch: String,
    revision: i64,
    endpoint: String,
    action: String,
}
async fn control(Json(input): Json<Control>) -> Api {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    let (epoch, revision, mut m) = model(&tx).await?;
    let before = serde_json::to_value(&m)?;
    ensure!(
        epoch == input.run_epoch && revision == input.revision,
        "stale control basis"
    );
    let e = m
        .endpoints
        .iter_mut()
        .find(|e| e.id == input.endpoint)
        .context("unknown endpoint")?;
    match input.action.as_str() {
        "fail" => e.mode = "failed".into(),
        "throttle" => e.mode = "throttled".into(),
        "pause-reports" => e.reporting = false,
        "resume-reports" => e.reporting = true,
        "restore" => {
            e.mode = "ok".into();
            e.reporting = true;
        }
        _ => return Err(anyhow::anyhow!("unknown control action").into()),
    }
    m.validate()?;
    let payload = serde_json::to_value(&m)?;
    if payload == before {
        return Ok(Json(
            json!({"run_epoch":epoch,"revision":revision,"model":m}),
        ));
    }
    tx.execute(
        "UPDATE ignite_model SET payload=$1,revision=revision+1 WHERE id='active'",
        &[&payload],
    )
    .await?;
    tx.execute(
        "UPDATE ignite_endpoints SET available=$1,revision=revision+1 WHERE id=$2",
        &[
            &(m.endpoints
                .iter()
                .find(|e| e.id == input.endpoint)
                .context("endpoint")?
                .mode
                == "ok"),
            &input.endpoint,
        ],
    )
    .await?;
    decisions::audit(&tx,None,"control",&json!({"run_epoch":epoch,"revision":revision+1,"endpoint":input.endpoint,"action":input.action})).await?;
    tx.commit().await?;
    Ok(Json(
        json!({"run_epoch":epoch,"revision":revision+1,"model":m}),
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

async fn endpoint_health(State(state): State<EndpointApp>) -> Api {
    let client = db().await?;
    let (epoch, _, m) = model(&client).await?;
    let e = m
        .endpoints
        .iter()
        .find(|e| e.id == state.id)
        .context("endpoint")?;
    Ok(Json(
        json!({"run_epoch":epoch,"id":e.id,"status":e.mode,"reporting":e.reporting}),
    ))
}

async fn infer(
    State(state): State<EndpointApp>,
    Json(input): Json<Inference>,
) -> std::result::Result<Response, Error> {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    ensure!(
        tx.query_one("SELECT pg_try_advisory_xact_lock_shared(190619)", &[])
            .await?
            .get::<_, bool>(0),
        "control or reset pending; retry admission"
    );
    let (epoch, revision, m) = model(&tx).await?;
    ensure!(
        epoch == input.run_epoch && revision == input.model_revision,
        "stale dispatch basis"
    );
    let e = m.dispatch(&input.workload, &state.id)?;
    ensure!(
        m.workloads
            .iter()
            .any(|w| w.id == input.workload && w.destination == state.id),
        "not the committed destination"
    );
    let row = tx
        .query_one(
            "SELECT content,workload,trace_id,root_span_id FROM ignite_tasks WHERE id=$1 AND run_epoch=$2",
            &[&input.task_id, &epoch],
        )
        .await?;
    ensure!(
        row.get::<_, String>(0) == input.content
            && row.get::<_, String>(1) == input.workload
            && row.get::<_, String>(2) == input.trace_id
            && row.get::<_, String>(3) == input.root_span_id,
        "task identity mismatch"
    );
    tx.query_one(
        "SELECT pg_advisory_xact_lock($1)",
        &[&(190700i64 + i64::from(port(&state.id)?))],
    )
    .await?;
    let mut span = state.app.telemetry.span(
        "model.inference",
        &input.trace_id,
        &input.root_span_id,
        &input.task_id,
        &input.workload,
    )?;
    span.set_attribute(KeyValue::new("endpoint.id", state.id.clone()));
    if let Some(row) = tx
        .query_opt(
            "SELECT response,tokens FROM ignite_receipts WHERE id=$1",
            &[&input.task_id],
        )
        .await?
    {
        return Ok(Json(json!({"response":row.get::<_,String>(0),"tokens":row.get::<_,i64>(1),"duplicate":true})).into_response());
    }
    let response = if input.workload == "support" {
        format!("Synthetic support answer: {} words reviewed; follow the documented account recovery steps.", input.content.split_whitespace().count())
    } else {
        format!(
            "Public document summary: {} words indexed for search.",
            input.content.split_whitespace().count()
        )
    };
    // Whitespace token units and fixed responses are explicitly simulated, never GPU measurements.
    let tokens =
        (input.content.split_whitespace().count() + response.split_whitespace().count()) as i64;
    let used: i64 = tx.query_one("SELECT COALESCE(sum(tokens),0)::bigint FROM ignite_receipts WHERE endpoint=$1 AND run_epoch=$2 AND received_ms>$3",
        &[&state.id,&epoch,&(now()-60_000)]).await?.get(0);
    let status = if e.mode == "failed" {
        503
    } else if e.mode == "throttled" || used + tokens > e.budget_tpm {
        429
    } else {
        200
    };
    span.set_attribute(KeyValue::new("http.response.status_code", status as i64));
    if status != 200 {
        span.set_status(Status::error(
            "Injected endpoint failure or actual modeled budget exhaustion",
        ));
        return Ok((
            StatusCode::from_u16(status)?,
            Json(json!({"error":e.mode,"endpoint":e.id})),
        )
            .into_response());
    }
    tx.execute("INSERT INTO ignite_receipts(id,task_id,run_epoch,endpoint,workload,content,response,tokens,received_ms,model_revision) VALUES($1,$1,$2,$3,$4,$5,$6,$7,$8,$9)",
        &[&input.task_id,&epoch,&state.id,&input.workload,&input.content,&response,&tokens,&now(),&revision]).await?;
    tx.commit().await?;
    span.set_attribute(KeyValue::new(
        "gen_ai.usage.input_tokens",
        input.content.split_whitespace().count() as i64,
    ));
    span.set_attribute(KeyValue::new(
        "gen_ai.usage.output_tokens",
        response.split_whitespace().count() as i64,
    ));
    Ok(Json(json!({"response":response,"tokens":tokens})).into_response())
}

async fn dispatch_once(app: &App, workload: &str) -> Result<()> {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock_shared(190619)")
        .await?;
    let (epoch, revision, m) = model(&tx).await?;
    let pending: i64 = tx.query_one("SELECT count(*) FROM ignite_tasks WHERE run_epoch=$1 AND workload=$2 AND completed_ms IS NULL", &[&epoch,&workload]).await?.get(0);
    app.telemetry.queue.record(
        pending as u64,
        &[KeyValue::new("workload", workload.to_owned())],
    );
    let Some(row) = tx.query_opt("SELECT id,content,trace_id,root_span_id FROM ignite_tasks WHERE run_epoch=$1 AND workload=$2 AND completed_ms IS NULL AND cancelled_ms IS NULL ORDER BY created_ms,id LIMIT 1", &[&epoch,&workload]).await? else { return Ok(()) };
    let id: String = row.get(0);
    let trace: String = row.get(2);
    let root_span: String = row.get(3);
    // Durable receipts reconcile an HTTP success whose gateway acknowledgement was lost.
    if let Some(receipt) = tx
        .query_opt(
            "SELECT endpoint,response,received_ms,model_revision FROM ignite_receipts WHERE task_id=$1",
            &[&id],
        )
        .await?
    {
        tx.execute("UPDATE ignite_tasks SET completed_ms=$1,endpoint=$2,outcome=$3,revision=revision+1,completed_revision=$5 WHERE id=$4 AND completed_ms IS NULL",
            &[&receipt.get::<_,i64>(2),&receipt.get::<_,String>(0),&receipt.get::<_,String>(1),&id,&receipt.get::<_,Option<i64>>(3)]).await?;
        decisions::score(&tx,&format!("{id}-outcome"),&trace,"drasi_task_completed",1.,
            &format!("Durable receipt at {}; actual task completion, not a recovery assertion",receipt.get::<_,String>(0))).await?;
        tx.commit().await?;
        app.telemetry
            .completed
            .add(1, &[KeyValue::new("workload", workload.to_owned())]);
        app.telemetry
            .span("task.completed", &trace, &root_span, &id, workload)?
            .end();
        return Ok(());
    }
    let w = m
        .workloads
        .iter()
        .find(|w| w.id == workload)
        .context("workload")?;
    let e = match m.dispatch(workload, &w.destination) {
        Ok(e) => e,
        Err(error) => {
            let detail = error.to_string();
            tx.execute("INSERT INTO ignite_attempts(id,task_id,run_epoch,workload,endpoint,model_revision,started_ms,finished_ms,status,detail,trace_id) VALUES($1,$2,$3,$4,$5,$6,$7,$7,403,$8,$9)",
                &[&identity(),&id,&epoch,&workload,&w.destination,&revision,&now(),&detail,&trace]).await?;
            decisions::score(
                &tx,
                &format!("{id}-policy-{revision}"),
                &trace,
                "drasi_policy_allowed",
                0.,
                &detail,
            )
            .await?;
            tx.commit().await?;
            let mut span =
                app.telemetry
                    .span("gateway.policy_rejected", &trace, &root_span, &id, workload)?;
            span.set_status(Status::error(detail));
            span.end();
            return Ok(());
        }
    };
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
        &[&attempt,&id,&epoch,&workload,&e.id,&revision,&started,&trace]).await?;
    tx.commit().await?;
    // The shared reset/control lock is held from the final basis check through HTTP completion.
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock_shared(190619)")
        .await?;
    let (current_epoch, current_revision, current) = model(&tx).await?;
    if current_epoch != epoch || current_revision != revision {
        tx.execute("UPDATE ignite_attempts SET finished_ms=$1,status=409,detail='basis changed before transmission' WHERE id=$2", &[&now(),&attempt]).await?;
        tx.commit().await?;
        return Ok(());
    }
    current.dispatch(workload, &e.id)?;
    decisions::score(
        &tx,
        &format!("{id}-policy-{revision}"),
        &trace,
        "drasi_policy_allowed",
        1.,
        &format!(
            "Current Rego policy revision {} checked before transmission to {}",
            current.policy.revision, e.id
        ),
    )
    .await?;
    let mut span = app
        .telemetry
        .span("gateway.attempt", &trace, &root_span, &id, workload)?;
    span.set_attributes([
        KeyValue::new("endpoint.id", e.id.clone()),
        KeyValue::new("attempt.id", attempt.clone()),
    ]);
    let response = app
        .http
        .post(format!("http://127.0.0.1:{}/infer", port(&e.id)?))
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
        KeyValue::new("endpoint", e.id.clone()),
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
        span.set_status(Status::error(detail));
    }
    Ok(())
}

async fn report_once(app: &App) -> Result<()> {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock_shared(190619)")
        .await?;
    let (epoch, revision, m) = model(&tx).await?;
    decisions::metrics(app, &epoch, revision, &m).await?;
    for e in &m.endpoints {
        app.telemetry.capacity.record(
            e.budget_tpm as u64,
            &[KeyValue::new("endpoint", e.id.clone())],
        );
        if !e.reporting {
            continue;
        }
        let value: Value = app
            .http
            .get(format!("http://127.0.0.1:{}/health", port(&e.id)?))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        ensure!(
            value["run_epoch"] == epoch && value["id"] == e.id,
            "mismatched report identity"
        );
        let status = value["status"]
            .as_str()
            .context("missing endpoint status")?;
        let observed = now();
        tx.execute("UPDATE ignite_reports SET observed_ms=$1,expires_ms=$2,status=$3,sequence=sequence+1 WHERE id=$4 AND run_epoch=$5",
            &[&observed,&(observed+8_000),&status,&e.id,&epoch]).await?;
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
        .get("http://127.0.0.1:18500/health")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let epoch = state["run_epoch"].as_str().context("epoch")?;
    for _ in 0..count {
        let value: Value = http
            .post("http://127.0.0.1:18500/tasks")
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
    model(&owner).await?;
    let app = App {
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(4))
            .build()?,
        telemetry: Arc::new(telemetry::Telemetry::new()?),
        context_ready: Arc::new(AtomicU64::new(0)),
        context_rows: Arc::new(tokio::sync::RwLock::new(BTreeMap::new())),
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
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18500").await?;
    let router = Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
        .route("/tasks", post(submit))
        .route("/traffic", post(traffic::control))
        .route("/tasks/{id}", get(task))
        .route("/control", post(control))
        .route("/decisions", get(decisions::get))
        .route("/approvals", post(decisions::approval))
        .route("/configuration", post(decisions::configuration))
        .route("/reset", post(decisions::reset))
        .route("/alerts/grafana", post(observer::alert))
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
            loop {
                dispatch_once(&app, workload).await?;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }
    let reports = app.clone();
    jobs.spawn(async move {
        loop {
            report_once(&reports).await?;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    let observer = app.clone();
    jobs.spawn(async move { observer::watch(observer).await });
    let adoption = app.clone();
    jobs.spawn(async move {
        loop {
            decisions::apply_once(&adoption).await?;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
    let publications = app.clone();
    jobs.spawn(async move {
        loop {
            decisions::publish_once(&publications).await?;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    println!("Ignite workload gateway 127.0.0.1:18500; model simulators 18531/18532/18533");
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let result = tokio::select! {
        signal = tokio::signal::ctrl_c() => { signal?; Ok(()) },
        _ = terminate.recv() => Ok(()),
        task = jobs.join_next() => { task.context("all workers exited")?? }
    };
    jobs.abort_all();
    while jobs.join_next().await.is_some() {}
    app.telemetry.shutdown()?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_stale_or_mixed_epoch_context_is_not_ready() {
        let mut rows = BTreeMap::new();
        assert!(!coherent_context(&rows, "epoch", 1));
        rows.insert(
            "model-context".into(),
            BTreeMap::from([("active".into(), json!({"run_epoch":"epoch","revision":1}))]),
        );
        rows.insert(
            "endpoint-freshness".into(),
            ["EU-A", "EU-B", "US-C"]
                .into_iter()
                .map(|id| {
                    (
                        id.into(),
                        json!({"run_epoch":"epoch","status":"ok","expires_ms":now()+8000}),
                    )
                })
                .collect(),
        );
        assert!(coherent_context(&rows, "epoch", 1));
        assert!(!coherent_context(&rows, "epoch", 2));
        assert!(!coherent_context(&rows, "new-epoch", 1));
        rows.get_mut("endpoint-freshness")
            .unwrap()
            .get_mut("EU-A")
            .unwrap()["status"] = json!("unknown");
        assert!(!coherent_context(&rows, "epoch", 1));
        rows.get_mut("endpoint-freshness").unwrap().remove("EU-A");
        assert!(!coherent_context(&rows, "epoch", 1));
    }
}
