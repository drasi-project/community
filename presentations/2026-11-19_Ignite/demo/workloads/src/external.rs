//! Stand-ins for the systems Drasi talks to (plan/contracts.md section 10):
//! - the change-approval tool, which records what the presenter approved and nothing else;
//! - the gateway configuration API, called only by Drasi's `gateway-reaction`.
//! Neither re-evaluates policy, plans or approvals: apply is ordinary optimistic concurrency on
//! the scenario revision, and every call is idempotent on approval state and delivery ID.
use crate::{db, now, scenario, Api, Error, Payload};
use anyhow::Result;
use axum::{extract::Path, http::HeaderMap, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

fn bounded(value: &str, name: &str) -> std::result::Result<(), Error> {
    if value.len() > 200 {
        return Err(Error::bad_request(&format!("{name} is too long")));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    #[serde(default)]
    id: Option<String>,
    candidate_id: String,
    basis: String,
    revision: i64,
}

/// `POST /approvals`: insert a pending approval and make it the latest. No validation of the plan.
pub async fn approve(Payload(input): Payload<Approval>) -> Api {
    let id = input.id.unwrap_or_else(crate::identity);
    if id.is_empty() || id.len() > 100 {
        return Err(Error::bad_request("approval id must be 1 to 100 characters"));
    }
    bounded(&input.candidate_id, "candidate_id")?;
    bounded(&input.basis, "basis")?;
    let mut client = db().await?;
    let tx = client.transaction().await?;
    // Serialises approvals so exactly one is `latest`.
    tx.batch_execute("SELECT pg_advisory_xact_lock(190622)")
        .await?;
    if tx
        .query_opt("SELECT 1 FROM ignite_approvals WHERE id=$1", &[&id])
        .await?
        .is_some()
    {
        return Ok(Json(json!({"id": id})));
    }
    let (epoch, _) = scenario::current(&tx).await?;
    tx.execute(
        "UPDATE ignite_approvals SET latest=false WHERE latest AND run_epoch=$1",
        &[&epoch],
    )
    .await?;
    tx.execute(
        "INSERT INTO ignite_approvals(id, run_epoch, candidate_id, basis, revision, status, latest, requested_ms)
         VALUES($1, $2, $3, $4, $5, 'pending', true, $6)",
        &[&id, &epoch, &input.candidate_id, &input.basis, &input.revision, &now()],
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"id": id})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Apply {
    approval: String,
    candidate_id: String,
    basis: String,
    expected_revision: i64,
    assignments: BTreeMap<String, String>,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// The approval was already decided (or the call is a replay): nothing to do.
    Duplicate,
    /// The scenario moved on since the plan; the approval is rejected with this reason.
    Conflict(String),
    Apply,
}

/// Compare-and-set decision for one `POST /routes/apply`, given the approval's stored state.
pub fn decide(
    status: &str,
    approval_epoch: &str,
    current_epoch: &str,
    revision: i64,
    expected_revision: i64,
) -> Outcome {
    if status != "pending" {
        Outcome::Duplicate
    } else if approval_epoch != current_epoch {
        Outcome::Conflict("The demo was reset after this approval".into())
    } else if revision != expected_revision {
        Outcome::Conflict(format!(
            "Scenario changed since the plan (revision {revision}, plan revision {expected_revision})"
        ))
    } else {
        Outcome::Apply
    }
}

/// Structural check of a route set: one known deployment for every workload. Not a policy check.
pub fn check_assignments(
    assignments: &BTreeMap<String, String>,
    workloads: &BTreeSet<String>,
    deployments: &BTreeSet<String>,
) -> Result<()> {
    anyhow::ensure!(
        assignments.keys().cloned().collect::<BTreeSet<_>>() == *workloads,
        "assignments must name every workload exactly once"
    );
    anyhow::ensure!(
        assignments.values().all(|d| deployments.contains(d)),
        "assignments name an unknown deployment"
    );
    Ok(())
}

/// `POST /routes/apply` (gateway configuration API).
pub async fn apply(headers: HeaderMap, Payload(input): Payload<Apply>) -> Api {
    let delivery = headers
        .get("x-drasi-delivery-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && v.len() <= 300)
        .ok_or_else(|| Error::bad_request("missing x-drasi-delivery-id"))?
        .to_owned();
    let mut client = db().await?;
    let tx = client.transaction().await?;
    // Exclusive with dispatch, so no request is sent on a route that is being replaced.
    tx.batch_execute("SELECT pg_advisory_xact_lock(190619)")
        .await?;
    if let Some(row) = tx
        .query_opt("SELECT result FROM ignite_deliveries WHERE id=$1", &[&delivery])
        .await?
    {
        return Ok(Json(row.get(0)));
    }
    let row = tx
        .query_opt(
            "SELECT status, run_epoch, candidate_id, basis FROM ignite_approvals WHERE id=$1 FOR UPDATE",
            &[&input.approval],
        )
        .await?
        .ok_or_else(|| Error::not_found("unknown approval"))?;
    let (status, approval_epoch): (String, String) = (row.get(0), row.get(1));
    if row.get::<_, String>(2) != input.candidate_id || row.get::<_, String>(3) != input.basis {
        return Err(Error::bad_request(
            "candidate_id and basis do not match the approval",
        ));
    }
    let (epoch, revision) = scenario::current(&tx).await?;
    let result = match decide(
        &status,
        &approval_epoch,
        &epoch,
        revision,
        input.expected_revision,
    ) {
        Outcome::Duplicate => json!({"result": "duplicate", "status": status}),
        Outcome::Conflict(reason) => {
            tx.execute(
                "UPDATE ignite_approvals SET status='rejected', reason=$1, decided_ms=$2 WHERE id=$3",
                &[&reason, &now(), &input.approval],
            )
            .await?;
            json!({"result": "conflict", "reason": reason})
        }
        Outcome::Apply => {
            let workloads = ids(&tx, "ignite_workloads", &epoch).await?;
            let deployments = ids(&tx, "ignite_deployments", &epoch).await?;
            check_assignments(&input.assignments, &workloads, &deployments).map_err(Error::bad)?;
            let applied = now();
            for (workload, deployment) in &input.assignments {
                tx.execute(
                    "UPDATE ignite_routes SET deployment=$1, approval=$2, applied_ms=$3
                     WHERE workload=$4 AND run_epoch=$5 AND deployment<>$1",
                    &[deployment, &input.approval, &applied, workload, &epoch],
                )
                .await?;
            }
            let applied_revision = scenario::bump(&tx).await?;
            tx.execute(
                "UPDATE ignite_approvals SET status='applied', reason=NULL, decided_ms=$1, applied_revision=$2 WHERE id=$3",
                &[&applied, &applied_revision, &input.approval],
            )
            .await?;
            json!({"result": "applied", "revision": applied_revision})
        }
    };
    tx.execute(
        "INSERT INTO ignite_deliveries(id, received_ms, result) VALUES($1, $2, $3)",
        &[&delivery, &now(), &result],
    )
    .await?;
    tx.commit().await?;
    Ok(Json(result))
}

async fn ids(
    client: &impl tokio_postgres::GenericClient,
    table: &str,
    epoch: &str,
) -> Result<BTreeSet<String>> {
    Ok(client
        .query(
            &format!("SELECT id FROM {table} WHERE run_epoch=$1"),
            &[&epoch],
        )
        .await?
        .iter()
        .map(|r| r.get(0))
        .collect())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reject {
    reason: String,
}

/// `POST /approvals/{id}/reject`: `pending` → `rejected` with Drasi's reason; otherwise a no-op.
pub async fn reject(Path(id): Path<String>, Payload(input): Payload<Reject>) -> Api {
    bounded(&input.reason, "reason")?;
    transition(
        &id,
        "pending",
        "rejected",
        "UPDATE ignite_approvals SET status='rejected', reason=$1, decided_ms=$2 WHERE id=$3",
        &input.reason,
    )
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verified {
    verified_ms: i64,
}

/// `POST /approvals/{id}/verified`: `applied` → `verified`; otherwise a no-op.
pub async fn verified(Path(id): Path<String>, Payload(input): Payload<Verified>) -> Api {
    transition(
        &id,
        "applied",
        "verified",
        "UPDATE ignite_approvals SET status='verified', verified_ms=$1, decided_ms=$2 WHERE id=$3",
        &input.verified_ms,
    )
    .await
}

/// Moves an approval `from` → `to` with `update` ($1 = `value`, $2 = now, $3 = id).
async fn transition(
    id: &str,
    from: &str,
    to: &str,
    update: &str,
    value: &(dyn tokio_postgres::types::ToSql + Sync),
) -> Api {
    let mut client = db().await?;
    let tx = client.transaction().await?;
    let status: String = tx
        .query_opt(
            "SELECT status FROM ignite_approvals WHERE id=$1 FOR UPDATE",
            &[&id],
        )
        .await?
        .ok_or_else(|| Error::not_found("unknown approval"))?
        .get(0);
    let result: Value = if status == from {
        tx.execute(update, &[value, &now(), &id]).await?;
        json!({"result": to})
    } else {
        json!({"result": "no-op", "status": status})
    };
    tx.commit().await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_is_compare_and_set_on_revision_and_idempotent_on_state() {
        assert_eq!(decide("pending", "e", "e", 3, 3), Outcome::Apply);
        for status in ["applied", "rejected", "verified"] {
            assert_eq!(decide(status, "e", "e", 3, 3), Outcome::Duplicate);
        }
        assert_eq!(
            decide("pending", "e", "e", 4, 3),
            Outcome::Conflict(
                "Scenario changed since the plan (revision 4, plan revision 3)".into()
            )
        );
        assert!(matches!(
            decide("pending", "old", "new", 1, 1),
            Outcome::Conflict(_)
        ));
    }

    #[test]
    fn assignments_need_every_workload_and_known_deployments_only() {
        let set = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
        let map = |v: &[(&str, &str)]| {
            v.iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect::<BTreeMap<_, _>>()
        };
        let w = set(&["batch", "support"]);
        let d = set(&["EU-A", "EU-B", "US-C"]);
        // The API never checks policy: support on US-C is structurally fine.
        assert!(check_assignments(&map(&[("support", "US-C"), ("batch", "US-C")]), &w, &d).is_ok());
        assert!(check_assignments(&map(&[("support", "EU-B")]), &w, &d).is_err());
        assert!(check_assignments(
            &map(&[("support", "EU-B"), ("batch", "EU-Z")]),
            &w,
            &d
        )
        .is_err());
        assert!(check_assignments(
            &map(&[("support", "EU-B"), ("batch", "US-C"), ("other", "EU-A")]),
            &w,
            &d
        )
        .is_err());
    }

    #[test]
    fn requests_match_the_reaction_templates() {
        let apply: Apply = serde_json::from_value(json!({
            "approval":"a1","candidate_id":"c","basis":"b","expected_revision":3,
            "assignments":{"support":"EU-B","batch":"US-C"}})).unwrap();
        assert_eq!(apply.assignments["batch"], "US-C");
        assert!(serde_json::from_value::<Reject>(json!({"reason":"The plan changed"})).is_ok());
        assert!(serde_json::from_value::<Verified>(json!({"verified_ms":1})).is_ok());
        assert!(serde_json::from_value::<Approval>(
            json!({"candidate_id":"c","basis":"b","revision":2})
        )
        .is_ok());
    }
}
