use crate::{db, model, now, Api, App};
use anyhow::{ensure, Context, Result};
use axum::{extract::State, Json};
use ignite_domain::decision::{fresh, Decision, Model};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tokio_postgres::GenericClient;

pub const STREAM_MASK: u64 = 32767;

pub async fn current(app: &App) -> Result<Decision> {
    ensure!(
        app.context_ready.load(Ordering::Acquire) == STREAM_MASK,
        "continuous evidence disconnected/initializing"
    );
    let rows = app.context_rows.read().await;
    let row = rows
        .get("allocation-results")
        .and_then(|r| r.get("active"))
        .context("candidate unavailable")?;
    let d: Decision = serde_json::from_str(row["payload"].as_str().context("candidate payload")?)?;
    ensure!(
        d.valid && fresh(&d, now()),
        "candidate invalid, expired or unknown"
    );
    Ok(d)
}

async fn verify(client: &impl GenericClient, d: &Decision) -> Result<()> {
    let (epoch, revision, m) = model(client).await?;
    ensure!(
        epoch == d.run_epoch
            && revision == d.revision
            && m.policy.revision == d.policy_revision
            && serde_json::to_value(&m)? == serde_json::to_value(&d.model)?,
        "stale decision basis/policy"
    );
    ensure!(
        fresh(d, now()) && d.valid,
        "decision evidence expired/unknown"
    );
    let reports = client
        .query(
            "SELECT id,run_epoch,status,expires_ms FROM ignite_reports",
            &[],
        )
        .await?;
    ensure!(reports.len() == 3, "missing observations");
    for r in reports {
        let id: String = r.get(0);
        let e = d.evidence.get(&id).context("missing decision evidence")?;
        ensure!(
            r.get::<_, String>(1) == epoch
                && r.get::<_, String>(2) == e.status
                && r.get::<_, i64>(3) > now(),
            "observation changed or expired"
        );
    }
    let mut proposed = m;
    ensure!(
        d.assignments.len() == proposed.workloads.len(),
        "incomplete assignment"
    );
    for w in &mut proposed.workloads {
        w.destination = d
            .assignments
            .get(&w.id)
            .context("missing commitment")?
            .clone();
    }
    for w in &proposed.workloads {
        proposed.dispatch(&w.id, &w.destination)?;
        ensure!(
            d.evidence[&w.destination].status == "ok",
            "destination not observed available"
        );
    }
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    command_id: String,
    candidate_id: String,
    basis: String,
}

async fn approve(app: &App, input: &Approval) -> Result<Value> {
    ensure!(
        !input.command_id.is_empty() && input.command_id.len() <= 100,
        "invalid command identity"
    );
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    if let Some(row) = tx
        .query_opt(
            "SELECT candidate_id,basis,to_jsonb(c) FROM ignite_commands c WHERE id=$1",
            &[&input.command_id],
        )
        .await?
    {
        ensure!(
            row.get::<_, String>(0) == input.candidate_id && row.get::<_, String>(1) == input.basis,
            "conflicting command identity reuse"
        );
        return Ok(json!({"duplicate":true,"command":row.get::<_,Value>(2)}));
    }
    let d = current(app).await?;
    ensure!(
        d.basis == input.basis && d.candidate_id == input.candidate_id,
        "stale candidate; refresh and approve explicitly"
    );
    verify(&tx, &d).await?;
    ensure!(
        d.model
            .workloads
            .iter()
            .any(|w| d.assignments[&w.id] != w.destination),
        "candidate needs no route change"
    );
    ensure!(tx.query_opt("SELECT id FROM ignite_commands WHERE run_epoch=$1 AND (candidate_id=$2 OR status='committed')",
        &[&d.run_epoch,&d.candidate_id]).await?.is_none(), "candidate already approved or another command awaiting application");
    tx.execute("INSERT INTO ignite_commands(id,run_epoch,candidate_id,basis,input_revision,policy_revision,decision,status,committed_ms,support_endpoint) VALUES($1,$2,$3,$4,$5,$6,$7,'committed',$8,$9)",
        &[&input.command_id,&d.run_epoch,&d.candidate_id,&d.basis,&d.revision,&d.policy_revision,
          &serde_json::to_value(&d)?,&now(),&d.assignments["support"]]).await?;
    audit(
        &tx,
        Some(&input.command_id),
        "approved",
        &json!({"basis":d.basis,"candidate_id":d.candidate_id}),
    )
    .await?;
    annotation(
        &tx,
        &format!("{}-committed", input.command_id),
        &format!(
            "Recovery approved: support to {}, summaries to {}. Awaiting gateway adoption.",
            d.assignments["support"], d.assignments["batch"]
        ),
    )
    .await?;
    let command: Value = tx
        .query_one(
            "SELECT to_jsonb(c) FROM ignite_commands c WHERE id=$1",
            &[&input.command_id],
        )
        .await?
        .get(0);
    tx.commit().await?;
    Ok(json!({"duplicate":false,"command":command}))
}
pub async fn approval(State(app): State<App>, Json(input): Json<Approval>) -> Api {
    match approve(&app, &input).await {
        Ok(value) => Ok(Json(value)),
        Err(error) => {
            let client = db().await?;
            audit(&client, Some(&input.command_id), "approval_rejected",
                &json!({"error":error.to_string(),"candidate_id":input.candidate_id,"basis":input.basis})).await?;
            let event_id = ignite_domain::decision::hash(&json!({
                "command":input.command_id,"basis":input.basis,"error":error.to_string()
            }))?;
            annotation(
                &client,
                &format!("approval-rejected-{event_id}"),
                &format!("Approval rejected: {error}"),
            )
            .await?;
            Err(error.into())
        }
    }
}

pub async fn audit(
    client: &impl GenericClient,
    command: Option<&str>,
    action: &str,
    detail: &Value,
) -> Result<()> {
    client
        .execute(
            "INSERT INTO ignite_audit(command_id,recorded_ms,action,detail,run_epoch) VALUES($1,$2,$3,$4,(SELECT run_epoch FROM ignite_model WHERE id='active'))",
            &[&command, &now(), &action, &detail],
        )
        .await?;
    Ok(())
}
pub async fn publication(
    client: &impl GenericClient,
    id: &str,
    target: &str,
    payload: &Value,
) -> Result<()> {
    client.execute("INSERT INTO ignite_publications(id,target,payload,created_ms) VALUES($1,$2,$3,$4) ON CONFLICT(id) DO NOTHING",
        &[&id,&target,&payload,&now()]).await?;
    Ok(())
}
pub async fn annotation(client: &impl GenericClient, id: &str, text: &str) -> Result<()> {
    publication(
        client,
        id,
        "grafana",
        &json!({"time":now(),"tags":["ignite-decision",id],"text":text}),
    )
    .await
}
pub async fn score(
    client: &impl GenericClient,
    key: &str,
    trace: &str,
    name: &str,
    value: f64,
    comment: &str,
) -> Result<()> {
    let hash = ignite_domain::decision::hash(&key)?;
    publication(client,key,"langfuse",&json!({"id":format!("{}-{}-{}-{}-{}",&hash[..8],&hash[8..12],&hash[12..16],&hash[16..20],&hash[20..32]),
        "traceId":trace,"name":name,"value":value,"dataType":"NUMERIC","comment":comment})).await
}

pub async fn apply_once(app: &App) -> Result<()> {
    let mut client = db().await?;
    let pending: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM ignite_commands WHERE status='committed')",
            &[],
        )
        .await?
        .get(0);
    if !pending {
        return Ok(());
    }
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    let Some(row)=tx.query_opt("SELECT id,decision FROM ignite_commands WHERE status='committed' ORDER BY committed_ms LIMIT 1",&[]).await?
        else { return Ok(()) };
    let id: String = row.get(0);
    let saved: Decision = serde_json::from_value(row.get(1))?;
    let (epoch, revision, m) = model(&tx).await?;
    if epoch != saved.run_epoch
        || revision != saved.revision
        || m.policy.revision != saved.policy_revision
    {
        tx.execute("UPDATE ignite_commands SET status='rejected',error='approved model/policy changed before adoption' WHERE id=$1",&[&id]).await?;
        audit(
            &tx,
            Some(&id),
            "application_rejected",
            &json!({"error":"approved model/policy changed before adoption"}),
        )
        .await?;
        tx.commit().await?;
        return Ok(());
    }
    // Unknown/disconnected evidence is retriable, never implicit approval or rejection.
    let current = match current(app).await {
        Ok(d) => d,
        Err(error) => {
            tx.execute(
                "UPDATE ignite_commands SET error=$1 WHERE id=$2",
                &[&format!("waiting for current evidence: {error}"), &id],
            )
            .await?;
            tx.commit().await?;
            return Ok(());
        }
    };
    let result = async {
        ensure!(
            current.basis == saved.basis && current.candidate_id == saved.candidate_id,
            "approved basis changed before adoption"
        );
        verify(&tx, &current).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    if let Err(error) = result {
        tx.execute(
            "UPDATE ignite_commands SET status='rejected',error=$1 WHERE id=$2",
            &[&error.to_string(), &id],
        )
        .await?;
        audit(
            &tx,
            Some(&id),
            "application_rejected",
            &json!({"error":error.to_string()}),
        )
        .await?;
    } else {
        let mut m = current.model;
        for w in &mut m.workloads {
            w.destination = current.assignments[&w.id].clone();
        }
        let applied_revision = current.revision + 1;
        tx.execute(
            "UPDATE ignite_model SET payload=$1,revision=$2 WHERE id='active'",
            &[&serde_json::to_value(&m)?, &applied_revision],
        )
        .await?;
        tx.execute("UPDATE ignite_commands SET status='gateway_applied',applied_ms=$1,applied_revision=$2,error=NULL WHERE id=$3",
            &[&now(),&applied_revision,&id]).await?;
        audit(
            &tx,
            Some(&id),
            "gateway_applied",
            &json!({"revision":applied_revision,"assignments":current.assignments}),
        )
        .await?;
        annotation(
            &tx,
            &format!("{id}-applied"),
            &format!("Gateway adopted routes: support to {}, summaries to {}. Recovery not yet verified.",
                current.assignments["support"], current.assignments["batch"]),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn recovered(row: &Value) -> Result<()> {
    if row["state"] != "confirmed" {
        return Ok(());
    }
    let id = row["id"].as_str().context("recovery id")?;
    let revision = row["applied_revision"]
        .as_i64()
        .context("applied revision")?;
    let epoch = row["run_epoch"].as_str().context("recovery epoch")?;
    let expires = row["expires_ms"].as_i64().context("recovery expiry")?;
    let mut client = db().await?;
    let pending: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM ignite_commands WHERE id=$1 AND status='gateway_applied')",
            &[&id],
        )
        .await?
        .get(0);
    if !pending {
        return Ok(());
    }
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    let (current_epoch, current_revision, _) = model(&tx).await?;
    if current_epoch != epoch || current_revision != revision || expires <= now() {
        return Ok(());
    }
    let evidence:bool=tx.query_one("SELECT NOT EXISTS(SELECT 1 FROM ignite_tasks WHERE run_epoch=$1 AND workload='support' AND completed_ms IS NULL AND cancelled_ms IS NULL) AND EXISTS(SELECT 1 FROM ignite_tasks WHERE run_epoch=$1 AND workload='support' AND completed_revision=$2 AND completed_ms>$3)",
        &[&epoch,&revision,&(now()-15000)]).await?.get(0);
    if !evidence {
        return Ok(());
    }
    let observed: bool = tx.query_one(
        "SELECT EXISTS(SELECT 1 FROM ignite_commands c JOIN ignite_reports r ON r.id=c.support_endpoint AND r.run_epoch=c.run_epoch WHERE c.id=$1 AND c.applied_ms+5000<=$2 AND r.status='ok' AND r.expires_ms>$2)",
        &[&id,&now()]).await?.get(0);
    if !observed {
        return Ok(());
    }
    let changed=tx.execute("UPDATE ignite_commands SET status='recovery_confirmed',recovered_ms=$1 WHERE id=$2 AND status='gateway_applied' AND applied_revision=$3",
        &[&now(),&id,&revision]).await?;
    if changed > 0 {
        audit(&tx, Some(id), "recovery_confirmed", row).await?;
        annotation(&tx,&format!("{id}-recovered"),"Recovery verified: successful support requests and no outstanding support tasks after the observation window.").await?;
        for task in tx.query("SELECT id,trace_id FROM ignite_tasks WHERE run_epoch=$1 AND workload='support' AND completed_revision=$2",&[&epoch,&revision]).await? {
            score(&tx,&format!("{}-recovery-{id}",task.get::<_,String>(0)),&task.get::<_,String>(1),"drasi_recovery",1.,&row.to_string()).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    run_epoch: String,
    revision: i64,
    model: Model,
}
pub async fn configuration(Json(mut input): Json<Configuration>) -> Api {
    Ok(Json(configure(&mut input).await?))
}
async fn configure(input: &mut Configuration) -> Result<Value> {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    let (epoch, revision, old) = model(&tx).await?;
    ensure!(
        epoch == input.run_epoch && revision == input.revision,
        "stale configuration basis"
    );
    ensure!(
        input.model.policy.revision == old.policy.revision,
        "policy revision is server-managed; submit current revision"
    );
    if input.model.policy.destinations != old.policy.destinations {
        input.model.policy.revision += 1;
    }
    input.model.validate()?;
    let payload = serde_json::to_value(&input.model)?;
    if payload == serde_json::to_value(&old)? {
        return Ok(json!({"run_epoch":epoch,"revision":revision,"model":old,"unchanged":true}));
    }
    tx.execute(
        "UPDATE ignite_model SET payload=$1,revision=revision+1 WHERE id='active'",
        &[&payload],
    )
    .await?;
    audit(
        &tx,
        None,
        "configuration",
        &json!({"run_epoch":epoch,"revision":revision+1,"model":payload}),
    )
    .await?;
    tx.commit().await?;
    Ok(json!({"run_epoch":epoch,"revision":revision+1,"model":input.model}))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reset {
    run_epoch: String,
    revision: i64,
}
pub async fn reset(Json(input): Json<Reset>) -> Api {
    Ok(Json(reset_model(input).await?))
}
async fn reset_model(input: Reset) -> Result<Value> {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    let (epoch, revision, _) = model(&tx).await?;
    ensure!(
        epoch == input.run_epoch && revision == input.revision,
        "stale reset basis"
    );
    let sql = include_str!("../../database/reset.sql")
        .lines()
        .filter(|line| !line.starts_with('\\') && *line != "BEGIN;" && *line != "COMMIT;")
        .collect::<Vec<_>>()
        .join("\n");
    tx.batch_execute(&sql).await?;
    audit(
        &tx,
        None,
        "reset",
        &json!({"previous_epoch":epoch,"previous_revision":revision}),
    )
    .await?;
    let (epoch, revision, m) = model(&tx).await?;
    tx.commit().await?;
    Ok(json!({"run_epoch":epoch,"revision":revision,"model":m}))
}
pub async fn get(State(app): State<App>) -> Api {
    let client = db().await?;
    let (epoch, revision, _) = model(&client).await?;
    let rows = app.context_rows.read().await;
    let attached = app.context_ready.load(Ordering::Acquire) == STREAM_MASK;
    let mut result = json!({"run_epoch":epoch,"revision":revision,"streams_attached":attached,
        "observation_window_ms":5000,"success_freshness_ms":15000,"approval":"unavailable"});
    for (field, name) in [
        ("policy", "policy-results"),
        ("allocation", "allocation-results"),
        ("resilience", "resilience-results"),
    ] {
        result[field] = match rows.get(name).and_then(|r| r.get("active")) {
            Some(row) => {
                serde_json::from_str::<Value>(row["payload"].as_str().context("decision payload")?)?
            }
            None => Value::Null,
        };
    }
    let d = result["allocation"].clone();
    if !d.is_null() {
        let d: Decision = serde_json::from_value(d)?;
        result["candidate_current"] =
            json!(attached && d.run_epoch == epoch && d.revision == revision && fresh(&d, now()));
        if result["candidate_current"] == true
            && d.valid
            && d.model
                .workloads
                .iter()
                .any(|w| d.assignments[&w.id] != w.destination)
        {
            result["approval"] = json!("awaiting_approval");
        }
    }
    result["recovery"] = json!(rows.get("recovery-results"));
    drop(rows);
    for (field,sql) in [
        ("commands","SELECT to_jsonb(c) FROM ignite_commands c WHERE run_epoch=$1 ORDER BY committed_ms DESC LIMIT 20"),
        ("audit","SELECT to_jsonb(a) FROM ignite_audit a WHERE run_epoch=$1 OR command_id IN (SELECT id FROM ignite_commands WHERE run_epoch=$1) ORDER BY id DESC LIMIT 50"),
    ] {
        result[field]=json!(client.query(sql,&[&epoch]).await?.iter().map(|r|r.get::<_,Value>(0)).collect::<Vec<_>>());
    }
    result["publications"]=json!(client.query("SELECT jsonb_build_object('id',id,'target',target,'attempts',attempts,'last_error',last_error,'next_attempt_ms',next_attempt_ms) FROM ignite_publications WHERE delivered_ms IS NULL ORDER BY created_ms LIMIT 30",&[]).await?.iter().map(|r|r.get::<_,Value>(0)).collect::<Vec<_>>());
    Ok(Json(result))
}

pub async fn publish_once(app: &App) -> Result<()> {
    let client = db().await?;
    let rows=client.query("SELECT id,target,payload FROM ignite_publications WHERE delivered_ms IS NULL AND next_attempt_ms<=$1 ORDER BY created_ms LIMIT 10",&[&now()]).await?;
    for row in rows {
        let id: String = row.get(0);
        let target: String = row.get(1);
        let payload: Value = row.get(2);
        let outcome = async {
            if target == "grafana" {
                let password = std::env::var("GRAFANA_ADMIN_PASSWORD")?;
                let existing: Value = app
                    .http
                    .get("http://127.0.0.1:13000/api/annotations")
                    .query(&[("tags", &id)])
                    .basic_auth("admin", Some(&password))
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                if existing
                    .as_array()
                    .context("Grafana annotation list")?
                    .is_empty()
                {
                    app.http
                        .post("http://127.0.0.1:13000/api/annotations")
                        .basic_auth("admin", Some(password))
                        .json(&payload)
                        .send()
                        .await?
                        .error_for_status()?;
                }
            } else {
                app.http
                    .post("http://127.0.0.1:13001/api/public/scores")
                    .header(
                        "Authorization",
                        format!("Basic {}", std::env::var("LANGFUSE_AUTH_BASE64")?),
                    )
                    .json(&payload)
                    .send()
                    .await?
                    .error_for_status()?;
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;
        match outcome {
            Ok(()) => {
                client.execute("UPDATE ignite_publications SET delivered_ms=$1,attempts=attempts+1,last_error=NULL WHERE id=$2",&[&now(),&id]).await?;
            }
            Err(error) => {
                eprintln!("Retriable {target} publication failure ({id}): {error}");
                client.execute("UPDATE ignite_publications SET attempts=attempts+1,last_error=$1,next_attempt_ms=$2 WHERE id=$3",
                    &[&error.to_string(),&(now()+5000),&id]).await?;
            }
        }
    }
    Ok(())
}

pub async fn maintain(name: &str, row: &Value) -> Result<()> {
    if !["allocation-results", "resilience-results"].contains(&name) {
        return Ok(());
    }
    let d: Decision = serde_json::from_str(row["payload"].as_str().context("decision payload")?)?;
    let id = format!("{name}-{}-{}", d.basis, d.state);
    annotation(
        &db().await?,
        &id,
        &format!(
            "{}: {}. {} (model {}, policy {})",
            if name == "allocation-results" {
                "Recovery plan"
            } else {
                "Resilience after EU-A loss"
            },
            d.state,
            d.reason,
            d.revision,
            d.policy_revision
        ),
    )
    .await
}

pub async fn metrics(app: &App, epoch: &str, revision: i64, model: &Model) -> Result<()> {
    use opentelemetry::KeyValue;
    let rows = app.context_rows.read().await;
    let attached = app.context_ready.load(Ordering::Acquire) == STREAM_MASK;
    let at = now();
    for sample in crate::dashboard::samples(&rows, epoch, revision, model, attached, at)? {
        app.telemetry.dashboard_valid_until.record(
            sample.valid_until_ms / 1000,
            &[KeyValue::new("view", sample.view)],
        );
        app.telemetry.dashboard.record(
            sample.value,
            &[
                KeyValue::new("view", sample.view),
                KeyValue::new("workload", sample.workload),
                KeyValue::new("endpoint", sample.endpoint),
            ],
        );
    }
    app.telemetry.dashboard_timestamp.record(at / 1000, &[]);
    for (kind, name) in [
        ("policy", "policy-results"),
        ("allocation", "allocation-results"),
        ("resilience", "resilience-results"),
    ] {
        let decision = rows
            .get(name)
            .and_then(|r| r.get("active"))
            .map(|row| -> Result<Decision> {
                Ok(serde_json::from_str(
                    row["payload"].as_str().context("decision payload")?,
                )?)
            })
            .transpose()?;
        let state = decision
            .as_ref()
            .filter(|d| {
                attached && d.run_epoch == epoch && d.revision == revision && fresh(d, now())
            })
            .map(|d| d.state.as_str())
            .unwrap_or("unknown");
        for label in ["fresh", "feasible", "infeasible", "unknown"] {
            app.telemetry.assessment.record(
                u64::from(state == label),
                &[KeyValue::new("kind", kind), KeyValue::new("state", label)],
            );
        }
    }
    let recovered = attached
        && rows.get("recovery-results").is_some_and(|rows| {
            rows.values().any(|r| {
                r["run_epoch"] == epoch
                    && r["applied_revision"] == revision
                    && r["state"] == "confirmed"
                    && r["expires_ms"]
                        .as_i64()
                        .is_some_and(|expires| expires > now())
            })
        });
    app.telemetry.assessment.record(
        u64::from(recovered),
        &[
            KeyValue::new("kind", "recovery"),
            KeyValue::new("state", "confirmed"),
        ],
    );
    Ok(())
}
