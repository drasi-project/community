//! The company's configuration (workloads, deployments, routes, policy destinations), the
//! simulated deployments' fault modes, and the demo controls that change them.
use crate::{db, now, Api, App, Error, Payload};
use anyhow::{Context, Result};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use tokio_postgres::GenericClient;

/// The deployments the AI service simulates. The scenario holds those provisioned
/// (`model_deployments`): any subset; the demo starts without US-C.
pub const DEPLOYMENTS: [&str; 3] = ["EU-A", "EU-B", "US-C"];
pub const WORKLOADS: [&str; 2] = ["support", "batch"];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Workload {
    pub id: String,
    pub data_class: String,
    pub region_requirement: String,
    pub model: String,
    pub demand_tpm: i64,
    /// The current route (`workload_routes`).
    pub deployment: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Deployment {
    pub id: String,
    pub region: String,
    pub model: String,
    pub capacity_tpm: i64,
    /// Simulated fault mode (`simulated_deployment_faults`), never published to Drasi.
    pub mode: String,
    pub reporting: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Scenario {
    pub run_epoch: String,
    pub revision: i64,
    pub workloads: Vec<Workload>,
    pub deployments: Vec<Deployment>,
    pub destinations: BTreeMap<String, Vec<String>>,
}

impl Scenario {
    pub fn route(&self, workload: &str) -> Result<&str> {
        Ok(&self
            .workloads
            .iter()
            .find(|w| w.id == workload)
            .context("unknown workload")?
            .deployment)
    }

    pub fn deployment(&self, id: &str) -> Result<&Deployment> {
        self.deployments
            .iter()
            .find(|d| d.id == id)
            .context("unknown deployment")
    }
}

/// The current run and scenario revision.
pub async fn current(client: &impl GenericClient) -> Result<(String, i64)> {
    let row = client
        .query_one(
            "SELECT run_epoch, revision FROM routing_config WHERE singleton",
            &[],
        )
        .await?;
    Ok((row.get(0), row.get(1)))
}

pub async fn load(client: &impl GenericClient) -> Result<Scenario> {
    let (run_epoch, revision) = current(client).await?;
    let workloads = client
        .query(
            "SELECT w.id, w.data_class, w.region_requirement, w.model, w.demand_tpm, r.deployment
             FROM ai_workloads w JOIN workload_routes r ON r.workload = w.id AND r.run_epoch = w.run_epoch
             WHERE w.run_epoch = $1 ORDER BY w.id",
            &[&run_epoch],
        )
        .await?
        .iter()
        .map(|r| Workload {
            id: r.get(0),
            data_class: r.get(1),
            region_requirement: r.get(2),
            model: r.get(3),
            demand_tpm: r.get(4),
            deployment: r.get(5),
        })
        .collect::<Vec<_>>();
    let deployments = client
        .query(
            "SELECT d.id, d.region, d.model, d.capacity_tpm, s.mode, s.reporting
             FROM model_deployments d JOIN simulated_deployment_faults s ON s.deployment = d.id
             WHERE d.run_epoch = $1 ORDER BY d.id",
            &[&run_epoch],
        )
        .await?
        .iter()
        .map(|r| Deployment {
            id: r.get(0),
            region: r.get(1),
            model: r.get(2),
            capacity_tpm: r.get(3),
            mode: r.get(4),
            reporting: r.get(5),
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        workloads.len() == WORKLOADS.len() && !deployments.is_empty(),
        "scenario incomplete; run ops/native-runtime.py reset"
    );
    let mut destinations: BTreeMap<String, Vec<String>> = WORKLOADS
        .iter()
        .map(|w| (w.to_string(), Vec::new()))
        .collect();
    for r in client
        .query(
            "SELECT workload, deployment FROM data_residency_policy WHERE run_epoch = $1
             ORDER BY workload, deployment",
            &[&run_epoch],
        )
        .await?
    {
        destinations
            .entry(r.get(0))
            .or_default()
            .push(r.get(1));
    }
    Ok(Scenario {
        run_epoch,
        revision,
        workloads,
        deployments,
        destinations,
    })
}

pub async fn health(State(app): State<App>) -> Api {
    let client = db().await?;
    let s = load(&client).await?;
    let pending: i64 = client
        .query_one(
            "SELECT count(*) FROM ai_requests WHERE run_epoch=$1 AND completed_ms IS NULL AND cancelled_ms IS NULL",
            &[&s.run_epoch],
        )
        .await?
        .get(0);
    let traffic = app.traffic.lock().await.snapshot();
    Ok(Json(json!({
        "run_epoch": s.run_epoch, "revision": s.revision, "pending": pending,
        "task_target_seconds": 15, "report_expiry_seconds": 8, "traffic": traffic,
        "deployments": s.deployments, "workloads": s.workloads, "destinations": s.destinations,
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    run_epoch: String,
    deployment: String,
    action: String,
}

/// Simulated fault modes for one deployment: not scenario state, so the revision is unchanged.
pub fn apply_control(mode: &mut String, reporting: &mut bool, action: &str) -> Result<()> {
    match action {
        "fail" => *mode = "failed".into(),
        "throttle" => *mode = "throttled".into(),
        "pause-reports" => *reporting = false,
        "resume-reports" => *reporting = true,
        "restore" => {
            *mode = "ok".into();
            *reporting = true;
        }
        _ => anyhow::bail!("unknown control action"),
    }
    Ok(())
}

pub async fn control(Payload(input): Payload<Control>) -> Api {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    let (epoch, revision) = current(&tx).await?;
    if epoch != input.run_epoch {
        return Err(Error::conflict("stale control basis"));
    }
    let row = tx
        .query_opt(
            "SELECT mode, reporting FROM simulated_deployment_faults WHERE deployment=$1",
            &[&input.deployment],
        )
        .await?
        .ok_or_else(|| Error::bad_request("unknown deployment"))?;
    if input.action == "provision" {
        return provision(tx, &epoch, &input.deployment).await;
    }
    let (mut mode, mut reporting): (String, bool) = (row.get(0), row.get(1));
    apply_control(&mut mode, &mut reporting, &input.action).map_err(Error::bad)?;
    tx.execute(
        "UPDATE simulated_deployment_faults SET mode=$1, reporting=$2 WHERE deployment=$3",
        &[&mode, &reporting, &input.deployment],
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"run_epoch": epoch, "revision": revision})))
}

/// Adds a known but unprovisioned deployment to the scenario: healthy, 100k tokens per minute,
/// in its region. It is scenario state, so the revision rises.
async fn provision(tx: tokio_postgres::Transaction<'_>, epoch: &str, id: &str) -> Api {
    let region = match id {
        "US-C" => "US",
        "EU-A" | "EU-B" => "EU",
        _ => return Err(Error::bad_request("unknown deployment")),
    };
    let inserted = tx
        .execute(
            "INSERT INTO model_deployments(id, run_epoch, region, model, capacity_tpm)
             VALUES($1, $2, $3, 'support-model', 100000) ON CONFLICT(id) DO NOTHING",
            &[&id, &epoch, &region],
        )
        .await?;
    if inserted == 0 {
        return Err(Error::conflict("deployment already provisioned"));
    }
    tx.execute(
        "UPDATE simulated_deployment_faults SET mode='ok', reporting=true WHERE deployment=$1",
        &[&id],
    )
    .await?;
    let revision = bump(&tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"run_epoch": epoch, "revision": revision})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    run_epoch: String,
    revision: i64,
    #[serde(default)]
    capacity: BTreeMap<String, i64>,
    #[serde(default)]
    destinations: BTreeMap<String, Vec<String>>,
}

/// Checks a desk configuration change against the known workloads and deployments. This is
/// input validation of a demo control, not policy: any destination set is accepted.
pub fn validate_configuration(input: &Configuration, provisioned: &[String]) -> Result<()> {
    let known = |id: &str| provisioned.iter().any(|d| d == id);
    for (id, tpm) in &input.capacity {
        anyhow::ensure!(known(id), "unknown deployment {id}");
        anyhow::ensure!(
            (0..=1_000_000_000).contains(tpm),
            "capacity must be between 0 and 1000000000 tokens per minute"
        );
    }
    for (workload, list) in &input.destinations {
        anyhow::ensure!(
            WORKLOADS.contains(&workload.as_str()),
            "unknown workload {workload}"
        );
        anyhow::ensure!(
            list.iter().all(|d| known(d)),
            "{workload} names a deployment that isn't provisioned"
        );
        anyhow::ensure!(
            list.iter().collect::<BTreeSet<_>>().len() == list.len(),
            "duplicate destination for {workload}"
        );
    }
    Ok(())
}

pub async fn configuration(Payload(input): Payload<Configuration>) -> Api {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    let (epoch, revision) = current(&tx).await?;
    if epoch != input.run_epoch || revision != input.revision {
        return Err(Error::conflict("stale configuration basis"));
    }
    let provisioned: Vec<String> = tx
        .query("SELECT id FROM model_deployments WHERE run_epoch=$1", &[&epoch])
        .await?
        .iter()
        .map(|r| r.get(0))
        .collect();
    validate_configuration(&input, &provisioned).map_err(Error::bad)?;
    let mut changed = 0;
    for (id, tpm) in &input.capacity {
        changed += tx
            .execute(
                "UPDATE model_deployments SET capacity_tpm=$1 WHERE id=$2 AND run_epoch=$3 AND capacity_tpm<>$1",
                &[tpm, id, &epoch],
            )
            .await?;
    }
    for (workload, list) in &input.destinations {
        changed += tx
            .execute(
                "DELETE FROM data_residency_policy WHERE workload=$1 AND NOT deployment = ANY($2)",
                &[workload, list],
            )
            .await?;
        for deployment in list {
            changed += tx
                .execute(
                    "INSERT INTO data_residency_policy(id, run_epoch, workload, deployment)
                     VALUES($1 || '/' || $2, $3, $1, $2) ON CONFLICT(id) DO NOTHING",
                    &[workload, deployment, &epoch],
                )
                .await?;
        }
    }
    let revision = if changed > 0 {
        bump(&tx).await?
    } else {
        revision
    };
    tx.commit().await?;
    Ok(Json(json!({"run_epoch": epoch, "revision": revision})))
}

/// Raises the scenario revision by one; call once per transaction that changes the scenario.
pub async fn bump(client: &impl GenericClient) -> Result<i64> {
    Ok(client
        .query_one(
            "UPDATE routing_config SET revision = revision + 1 WHERE singleton RETURNING revision",
            &[],
        )
        .await?
        .get(0))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reset {
    run_epoch: String,
    revision: i64,
}

pub async fn reset(State(app): State<App>, Payload(input): Payload<Reset>) -> Api {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    let (epoch, revision) = current(&tx).await?;
    if epoch != input.run_epoch || revision != input.revision {
        return Err(Error::conflict("stale reset basis"));
    }
    tx.batch_execute(&reset_sql()).await?;
    let (epoch, revision) = current(&tx).await?;
    tx.commit().await?;
    let cleared = now();
    let channel = app.clone();
    tokio::spawn(async move {
        // The incident channel starts empty too (best effort; Mattermost may not be running).
        if let Err(error) = crate::mattermost::clear_channel(&channel).await {
            eprintln!("#incident-ai-support not cleared: {error:#}");
        }
    });
    tokio::spawn(async move {
        for attempt in 0..5 {
            match clear_annotations(&app, cleared).await {
                Ok(()) => return,
                Err(error) if attempt == 4 => {
                    eprintln!("Previous run's Grafana annotations not cleared: {error:#}")
                }
                Err(_) => tokio::time::sleep(std::time::Duration::from_secs(5)).await,
            }
        }
    });
    Ok(Json(json!({"run_epoch": epoch, "revision": revision})))
}

/// `database/reset.sql` without its psql directives and transaction, so it runs inside ours.
pub fn reset_sql() -> String {
    include_str!("../../database/reset.sql")
        .lines()
        .filter(|line| !line.starts_with('\\') && *line != "BEGIN;" && *line != "COMMIT;")
        .collect::<Vec<_>>()
        .join("\n")
}

/// Demo driver: start the decision-events list clean by deleting the previous run's annotations
/// (tag `drasi-decision`, written by Drasi's annotation reaction).
async fn clear_annotations(app: &App, before: i64) -> Result<()> {
    let Ok(password) = std::env::var("GRAFANA_ADMIN_PASSWORD") else {
        return Ok(());
    };
    let grafana = ignite_settings::get().url("GRAFANA")?;
    let existing: Value = app
        .http
        .get(format!("{grafana}/api/annotations"))
        .query(&[("tags", "drasi-decision"), ("limit", "5000")])
        .basic_auth("admin", Some(&password))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    for annotation in existing.as_array().context("Grafana annotation list")? {
        if annotation["time"].as_i64().is_some_and(|t| t <= before) {
            let id = annotation["id"].as_i64().context("annotation id")?;
            app.http
                .delete(format!("{grafana}/api/annotations/{id}"))
                .basic_auth("admin", Some(&password))
                .send()
                .await?
                .error_for_status()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_change_only_the_simulated_fault_mode() {
        let (mut mode, mut reporting) = ("ok".to_string(), true);
        apply_control(&mut mode, &mut reporting, "fail").unwrap();
        assert_eq!((mode.as_str(), reporting), ("failed", true));
        apply_control(&mut mode, &mut reporting, "pause-reports").unwrap();
        assert_eq!((mode.as_str(), reporting), ("failed", false));
        apply_control(&mut mode, &mut reporting, "throttle").unwrap();
        assert_eq!(mode, "throttled");
        apply_control(&mut mode, &mut reporting, "restore").unwrap();
        assert_eq!((mode.as_str(), reporting), ("ok", true));
        assert!(apply_control(&mut mode, &mut reporting, "explode").is_err());
    }

    fn configuration(value: Value) -> Configuration {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn configuration_accepts_any_destination_set_but_only_known_ids() {
        let base = json!({"run_epoch":"e","revision":1});
        let ok = |extra: Value| {
            let mut v = base.clone();
            v.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            let provisioned = ["EU-A", "EU-B", "US-C"].map(String::from);
            validate_configuration(&configuration(v), &provisioned)
        };
        assert!(ok(json!({"capacity":{"EU-A":50000}})).is_ok());
        assert!(ok(json!({"destinations":{"batch":["EU-A","EU-B"]}})).is_ok());
        // Not a policy check: even an empty set or one the Rego rules deny is a valid edit.
        assert!(ok(json!({"destinations":{"support":["US-C"]}})).is_ok());
        assert!(ok(json!({"destinations":{"support":[]}})).is_ok());
        assert!(ok(json!({"capacity":{"EU-Z":1}})).is_err());
        assert!(ok(json!({"capacity":{"EU-A":-1}})).is_err());
        assert!(ok(json!({"destinations":{"batch":["EU-A","EU-A"]}})).is_err());
        assert!(ok(json!({"destinations":{"other":["EU-A"]}})).is_err());
        let europe = ["EU-A", "EU-B"].map(String::from);
        let us = configuration(json!({"run_epoch":"e","revision":1,"destinations":{"batch":["EU-A","US-C"]}}));
        assert!(validate_configuration(&us, &europe).is_err(), "US-C not provisioned");
        assert!(serde_json::from_value::<Configuration>(
            json!({"run_epoch":"e","revision":1,"routes":{"support":"EU-B"}})
        )
        .is_err());
    }

    #[test]
    fn reset_sql_runs_inside_the_callers_transaction() {
        let sql = reset_sql();
        assert!(!sql.contains("BEGIN;") && !sql.contains("COMMIT;") && !sql.contains("\\set"));
        assert!(sql.contains("workload_routes") && sql.contains("simulated_deployment_faults"));
    }
}
