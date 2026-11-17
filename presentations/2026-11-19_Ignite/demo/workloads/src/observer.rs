use crate::{db, now, Api, App};
use anyhow::{ensure, Context, Result};
use axum::Json;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::{sync::atomic::Ordering, time::Duration};

async fn persist(row: &Value) -> Result<()> {
    let id = row["id"].as_str().context("breach task identity")?;
    let epoch = row["run_epoch"].as_str().context("breach epoch")?;
    let deadline = row["deadline_ms"].as_i64().context("breach deadline")?;
    let trace = row["trace_id"].as_str().context("breach trace")?;
    let client = db().await?;
    // History comes from a real maintained query result, including bootstrap reconstruction.
    client.execute("INSERT INTO ignite_breaches(id,run_epoch,deadline_ms,observed_ms,trace_id) SELECT id,run_epoch,deadline_ms,$1,trace_id FROM ignite_tasks WHERE id=$2 AND run_epoch=$3 AND deadline_ms=$4 AND trace_id=$5 ON CONFLICT(id) DO NOTHING",
        &[&now(),&id,&epoch,&deadline,&trace]).await?;
    Ok(())
}

async fn attach(app: &App, name: &str, bit: u64) -> Result<()> {
    let root = format!("{}/api/v1/instances/ignite", ignite_settings::get().url("DRASI_API")?);
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .build()?;
    let response = client
        .get(format!("{root}/queries/{name}/attach"))
        .send()
        .await?
        .error_for_status()?;
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();
    let mut snapshot = false;
    // Server sends SSE keepalives every 15 seconds, even for an unchanged query.
    while let Some(chunk) = tokio::time::timeout(Duration::from_secs(35), stream.next()).await? {
        buffer.extend_from_slice(&chunk?);
        ensure!(
            buffer.len() < 4_000_000,
            "SSE frame exceeds bounded observer buffer"
        );
        while let Some(end) = buffer.windows(2).position(|bytes| bytes == b"\n\n") {
            let frame = std::str::from_utf8(&buffer[..end])?.to_owned();
            buffer.drain(..end + 2);
            let kind = frame
                .lines()
                .find_map(|line| line.strip_prefix("event:"))
                .map(str::trim);
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .map(str::trim)
                .collect::<Vec<_>>()
                .join("\n");
            if data.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&data)?;
            if kind == Some("snapshot") {
                ensure!(!snapshot, "duplicate initial snapshot");
                let mut replacement = std::collections::BTreeMap::new();
                for row in value.as_array().context("snapshot rows")? {
                    if name == "deadline-history" {
                        persist(row).await?;
                    }
                    if name == "recovery-results" {
                        crate::decisions::recovered(row).await?;
                    }
                    crate::decisions::maintain(name, row).await?;
                    replacement.insert(
                        row["id"].as_str().context("snapshot identity")?.to_owned(),
                        row.clone(),
                    );
                }
                app.context_rows
                    .write()
                    .await
                    .insert(name.to_owned(), replacement);
                snapshot = true;
                app.context_ready.fetch_or(bit, Ordering::Release);
            } else {
                ensure!(snapshot, "live result before snapshot");
                for change in value["results"].as_array().context("result differences")? {
                    let row = change.get("after").unwrap_or(&change["data"]);
                    let id = row["id"].as_str().context("result identity")?;
                    match change["type"].as_str() {
                        Some("ADD" | "UPDATE") => {
                            if name == "deadline-history" {
                                persist(row).await?;
                            }
                            if name == "recovery-results" {
                                crate::decisions::recovered(row).await?;
                            }
                            crate::decisions::maintain(name, row).await?;
                            app.context_rows
                                .write()
                                .await
                                .entry(name.to_owned())
                                .or_default()
                                .insert(id.to_owned(), row.clone());
                        }
                        Some("DELETE") => {
                            app.context_rows
                                .write()
                                .await
                                .entry(name.to_owned())
                                .or_default()
                                .remove(id);
                        }
                        _ => anyhow::bail!("unknown query operation"),
                    }
                }
            }
        }
    }
    anyhow::bail!("query attachment disconnected")
}

pub async fn watch(app: App) -> Result<()> {
    let names = [
        "model-context",
        "task-context",
        "overdue-tasks",
        "deadline-history",
        "endpoint-freshness",
        "attempt-context",
        "breach-history",
        "alert-context",
        "decision-context",
        "policy-results",
        "allocation-results",
        "resilience-results",
        "recovery-results",
        "recovery-task-context",
        "recovery-observation-context",
    ];
    futures_util::future::join_all(names.into_iter().enumerate().map(|(index, name)| {
        let app = app.clone();
        async move {
            loop {
                let bit = 1 << index;
                let result = attach(&app, name, bit).await;
                app.context_ready.fetch_and(!bit, Ordering::Release);
                app.context_rows.write().await.remove(name);
                eprintln!(
                    "Continuous context UNKNOWN ({name}); reconnecting snapshot-first: {result:?}"
                );
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }))
    .await;
    anyhow::bail!("all context observers exited")
}

pub async fn alert(Json(payload): Json<Value>) -> Api {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    let alerts = payload["alerts"]
        .as_array()
        .context("native Grafana alerts array")?;
    if alerts.is_empty() {
        return Err(anyhow::anyhow!("empty Grafana alert group").into());
    }
    for alert in alerts {
        let id = alert["fingerprint"]
            .as_str()
            .context("Grafana fingerprint")?;
        let status = alert["status"].as_str().context("Grafana status")?;
        if !["firing", "resolved"].contains(&status) {
            return Err(anyhow::anyhow!("invalid Grafana status").into());
        }
        let event = alert[if status == "resolved" {
            "endsAt"
        } else {
            "startsAt"
        }]
        .as_str()
        .context("event timestamp")?;
        let event_ms = chrono::DateTime::parse_from_rfc3339(event)?.timestamp_millis();
        let title = payload["title"].as_str().context("Grafana title")?;
        let message = payload["message"].as_str().context("Grafana message")?;
        tx.execute("INSERT INTO ignite_alerts(id,status,title,message,event_ms,received_ms,payload) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO UPDATE SET status=excluded.status,title=excluded.title,message=excluded.message,event_ms=excluded.event_ms,received_ms=excluded.received_ms,payload=excluded.payload WHERE excluded.event_ms>ignite_alerts.event_ms",
            &[&id,&status,&title,&message,&event_ms,&now(),&payload]).await?;
    }
    tx.commit().await?;
    Ok(Json(json!({"accepted":true,"durable":true})))
}
