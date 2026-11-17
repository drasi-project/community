use crate::model::Model;
use anyhow::{ensure, Context, Result};
use ignite_domain::decision::{fresh, Decision};
use serde_json::Value;
use std::collections::BTreeMap;

pub struct Sample {
    pub view: &'static str,
    pub workload: String,
    pub endpoint: String,
    pub value: i64,
    pub valid_until_ms: i64,
}

/// Project maintained results for Grafana; policy and allocation stay in Drasi.
pub fn samples(
    rows: &BTreeMap<String, BTreeMap<String, Value>>,
    epoch: &str,
    revision: i64,
    model: &Model,
    attached: bool,
    now: i64,
) -> Result<Vec<Sample>> {
    let coherent = attached
        && rows
            .get("model-context")
            .and_then(|r| r.get("active"))
            .is_some_and(|r| r["run_epoch"] == epoch && r["revision"] == revision);
    let read = |name: &str| -> Result<Option<Decision>> {
        rows.get(name)
            .and_then(|r| r.get("active"))
            .map(|row| {
                let d: Decision =
                    serde_json::from_str(row["payload"].as_str().context("decision payload")?)?;
                ensure!(
                    row["run_epoch"] == d.run_epoch && row["revision"] == d.revision,
                    "decision envelope and payload disagree"
                );
                Ok(d)
            })
            .transpose()
    };
    let policy = read("policy-results")?;
    let allocation = read("allocation-results")?;
    let resilience = read("resilience-results")?;
    let current =
        |d: &&Decision| coherent && d.run_epoch == epoch && d.revision == revision && fresh(d, now);
    let policy = policy.as_ref().filter(current);
    let allocation = allocation.as_ref().filter(current);
    let resilience = resilience.as_ref().filter(current);
    let expiry = |decision: Option<&Decision>| {
        decision
            .and_then(|d| d.evidence.values().map(|e| e.expires_ms).min())
            .unwrap_or(now + 8000)
    };
    let report_expiry = rows
        .get("endpoint-freshness")
        .into_iter()
        .flat_map(|r| r.values())
        .filter(|r| r["run_epoch"] == epoch)
        .filter_map(|r| r["expires_ms"].as_i64())
        .min()
        .unwrap_or(now + 8000);
    let recovery_expiry = rows
        .get("recovery-results")
        .into_iter()
        .flat_map(|r| r.values())
        .filter(|r| {
            r["run_epoch"] == epoch
                && r["applied_revision"] == revision
                && r["state"] == "confirmed"
        })
        .filter_map(|r| r["expires_ms"].as_i64())
        .filter(|t| *t > now)
        .min()
        .unwrap_or(now + 8000);
    let validity = BTreeMap::from([
        ("policy", expiry(policy)),
        ("candidate", expiry(allocation)),
        ("proposed", expiry(allocation)),
        ("approval", expiry(allocation)),
        ("resilience", expiry(resilience)),
        ("endpoint", report_expiry),
        ("recovery", recovery_expiry),
    ]);
    let mut result = Vec::new();
    let mut push = |view, workload: &str, endpoint: &str, value| {
        result.push(Sample {
            view,
            workload: workload.into(),
            endpoint: endpoint.into(),
            value,
            valid_until_ms: validity.get(view).copied().unwrap_or(now + 8000),
        });
    };
    for (view, decision) in [("candidate", allocation), ("resilience", resilience)] {
        let state = decision.map(|d| d.state.as_str());
        // An infeasible resilience result names the deployment whose loss leaves no plan.
        let failure = decision
            .filter(|d| view == "resilience" && d.state == "infeasible")
            .and_then(|d| d.failure.as_deref())
            .unwrap_or("");
        push(
            view,
            "",
            failure,
            match state {
                Some("feasible") => 1,
                Some("infeasible") => 0,
                _ => -1,
            },
        );
    }
    push("context", "", "", if coherent { 1 } else { -1 });
    // -1 unknown, 0 nothing to approve, 1 approve the proposed plan, 2 no valid plan,
    // 3 at risk: current routes are the plan, but losing one deployment would leave no plan.
    push(
        "approval",
        "",
        "",
        allocation.map_or(-1, |d| {
            if !d.valid {
                2
            } else if model
                .workloads
                .iter()
                .any(|w| d.assignments.get(&w.id) != Some(&w.destination))
            {
                1
            } else if resilience.is_some_and(|r| r.state == "infeasible") {
                3
            } else {
                0
            }
        }),
    );
    for workload in &model.workloads {
        let overdue = rows.get("overdue-tasks").filter(|_| coherent);
        push(
            "overdue",
            &workload.id,
            "",
            overdue.map_or(-1, |r| {
                r.values()
                    .filter(|r| r["run_epoch"] == epoch && r["workload"] == workload.id)
                    .count() as i64
            }),
        );
        for endpoint in &model.endpoints {
            push(
                "policy",
                &workload.id,
                &endpoint.id,
                policy
                    .and_then(|d| d.eligibility.get(&workload.id))
                    .and_then(|destinations| destinations.get(&endpoint.id))
                    .and_then(|value| value["allowed"].as_bool())
                    .map_or(-1, i64::from),
            );
            // Negative: the route is held because the current policy denies it.
            let denied = policy
                .and_then(|d| d.eligibility.get(&workload.id))
                .and_then(|destinations| destinations.get(&endpoint.id))
                .is_some_and(|value| value["allowed"] == false);
            push(
                "configured",
                &workload.id,
                &endpoint.id,
                if workload.destination != endpoint.id {
                    0
                } else if denied {
                    -workload.reservation_tpm
                } else {
                    workload.reservation_tpm
                },
            );
            push(
                "proposed",
                &workload.id,
                &endpoint.id,
                allocation.filter(|d| d.valid).map_or(-1, |d| {
                    if d.assignments.get(&workload.id) == Some(&endpoint.id) {
                        workload.reservation_tpm
                    } else {
                        0
                    }
                }),
            );
        }
    }
    for endpoint in &model.endpoints {
        let report = rows
            .get("endpoint-freshness")
            .and_then(|r| r.get(&endpoint.id))
            .filter(|r| {
                coherent
                    && r["run_epoch"] == epoch
                    && r["expires_ms"].as_i64().is_some_and(|t| t > now)
                    && r["observed_ms"].as_i64().is_some_and(|t| t <= now)
            });
        push(
            "endpoint",
            "",
            &endpoint.id,
            match report.and_then(|r| r["status"].as_str()) {
                Some("ok") => 0,
                Some("throttled") => 1,
                Some("failed") => 2,
                _ => -1,
            },
        );
    }
    push(
        "recovery",
        "",
        "",
        // 2: no recovery applied at this revision; 0: applied, awaiting evidence; 1: verified.
        if coherent {
            let applied: Vec<_> = rows
                .get("recovery-results")
                .into_iter()
                .flat_map(|r| r.values())
                .filter(|r| r["run_epoch"] == epoch && r["applied_revision"] == revision)
                .collect();
            if applied.is_empty() {
                2
            } else {
                i64::from(applied.iter().any(|r| {
                    r["state"] == "confirmed" && r["expires_ms"].as_i64().is_some_and(|t| t > now)
                }))
            }
        } else {
            -1
        },
    );
    Ok(result)
}

/// The latest approval in this scenario, from the command and audit records.
pub struct Approval<'a> {
    pub id: &'a str,
    pub status: &'a str,
    pub committed_ms: i64,
}

/// 0 none, 1 approved and applying, 2 applied awaiting evidence, 3 verified (once), 4 rejected.
pub fn last_approval(
    rows: &BTreeMap<String, BTreeMap<String, Value>>,
    latest: Option<Approval>,
    rejected_ms: Option<i64>,
    epoch: &str,
    now: i64,
) -> Sample {
    let verified = latest.as_ref().and_then(|c| {
        rows.get("recovery-results")
            .and_then(|r| r.get(c.id))
            .filter(|r| r["run_epoch"] == epoch && r["state"] == "confirmed")
            .and_then(|r| r["expires_ms"].as_i64())
            .filter(|t| *t > now)
    });
    let value = match &latest {
        _ if rejected_ms.is_some_and(|t| latest.as_ref().is_none_or(|c| t > c.committed_ms)) => 4,
        None => 0,
        Some(c) => match c.status {
            "committed" => 1,
            "rejected" => 4,
            // Once verified, the approval stays verified; Recovery shows whether it still holds.
            "recovery_confirmed" => 3,
            _ => 2,
        },
    };
    Sample {
        view: "last_approval",
        workload: String::new(),
        endpoint: String::new(),
        value,
        valid_until_ms: verified.map_or(now + 8000, |t| t.min(now + 8000)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ignite_domain::decision::{allocate, policy, resilience};
    use serde_json::json;

    fn fixture() -> (Model, BTreeMap<String, BTreeMap<String, Value>>) {
        let model: Model = serde_json::from_value(json!({
            "endpoints": [
                {"id":"EU-A","region":"EU","model":"test","budget_tpm":100000,"mode":"failed","reporting":true},
                {"id":"EU-B","region":"EU","model":"test","budget_tpm":100000,"mode":"ok","reporting":true},
                {"id":"US-C","region":"US","model":"test","budget_tpm":100000,"mode":"ok","reporting":true}
            ],
            "workloads": [
                {"id":"support","region":"EU","model":"test","reservation_tpm":60000,"destination":"EU-A"},
                {"id":"batch","region":"any","model":"test","reservation_tpm":80000,"destination":"EU-B"}
            ],
            "policy":{"revision":1,"destinations":{"support":["EU-A","EU-B"],"batch":["EU-A","EU-B","US-C"]}}
        })).unwrap();
        let reports: BTreeMap<_, _> = model
            .endpoints
            .iter()
            .map(|e| {
                (
                    e.id.clone(),
                    json!({
                        "id":e.id,"run_epoch":"test","revision":1,"payload":model,
                        "status":e.mode,"observed_ms":1000,"expires_ms":9000,"sequence":1
                    }),
                )
            })
            .collect();
        let p = policy(&reports).unwrap().unwrap();
        let a = allocate(p.clone(), None).unwrap();
        let r = resilience(p.clone()).unwrap();
        let mut rows = BTreeMap::from([
            (
                "model-context".into(),
                BTreeMap::from([("active".into(), json!({"run_epoch":"test","revision":1}))]),
            ),
            ("endpoint-freshness".into(), reports),
            (
                "overdue-tasks".into(),
                BTreeMap::from([(
                    "task".into(),
                    json!({"run_epoch":"test","workload":"support"}),
                )]),
            ),
        ]);
        for (name, d) in [
            ("policy-results", p),
            ("allocation-results", a),
            ("resilience-results", r),
        ] {
            rows.insert(
                name.into(),
                BTreeMap::from([(
                    "active".into(),
                    json!({
                        "run_epoch":"test","revision":1,"payload":serde_json::to_string(&d).unwrap()
                    }),
                )]),
            );
        }
        (model, rows)
    }

    fn replace(rows: &mut BTreeMap<String, BTreeMap<String, Value>>, name: &str, d: &Decision) {
        rows.insert(
            name.into(),
            BTreeMap::from([(
                "active".into(),
                json!({"run_epoch":"test","revision":1,"payload":serde_json::to_string(d).unwrap()}),
            )]),
        );
    }

    fn value(samples: &[Sample], view: &str, workload: &str, endpoint: &str) -> i64 {
        samples
            .iter()
            .find(|s| s.view == view && s.workload == workload && s.endpoint == endpoint)
            .unwrap()
            .value
    }

    #[test]
    fn projects_actual_policy_plan_and_impact_without_computing_them() {
        let (model, rows) = fixture();
        let s = samples(&rows, "test", 1, &model, true, 2000).unwrap();
        assert_eq!(value(&s, "policy", "support", "US-C"), 0);
        assert_eq!(value(&s, "policy", "batch", "US-C"), 1);
        assert_eq!(value(&s, "configured", "support", "EU-A"), 60000);
        assert_eq!(value(&s, "proposed", "support", "EU-B"), 60000);
        assert_eq!(value(&s, "proposed", "batch", "US-C"), 80000);
        assert_eq!(value(&s, "overdue", "support", ""), 1);
        assert_eq!(value(&s, "endpoint", "", "EU-A"), 2);
        assert_eq!(value(&s, "approval", "", ""), 1);
        assert_eq!(value(&s, "recovery", "", ""), 2);
        assert_eq!(value(&s, "resilience", "", "EU-B"), 0);
    }

    #[test]
    fn approval_flags_current_routes_at_risk_and_held_routes_are_negative() {
        let (mut model, mut rows) = fixture();
        model.workloads[0].destination = "EU-B".into();
        model.workloads[1].destination = "US-C".into();
        let p: Decision = serde_json::from_str(
            rows["policy-results"]["active"]["payload"].as_str().unwrap(),
        )
        .unwrap();
        let mut p = p;
        p.model = model.clone();
        replace(&mut rows, "allocation-results", &allocate(p.clone(), None).unwrap());
        replace(&mut rows, "resilience-results", &resilience(p.clone()).unwrap());
        let s = samples(&rows, "test", 1, &model, true, 2000).unwrap();
        assert_eq!(value(&s, "approval", "", ""), 3);
        assert_eq!(value(&s, "resilience", "", "EU-B"), 0);
        let mut safe = resilience(p.clone()).unwrap();
        safe.state = "feasible".into();
        safe.failure = None;
        replace(&mut rows, "resilience-results", &safe);
        let s = samples(&rows, "test", 1, &model, true, 2000).unwrap();
        assert_eq!(value(&s, "approval", "", ""), 0);
        assert_eq!(value(&s, "resilience", "", ""), 1);
        assert_eq!(value(&s, "configured", "support", "EU-B"), 60000);
        p.eligibility
            .get_mut("support")
            .unwrap()
            .insert("EU-B".into(), json!({"allowed":false}));
        replace(&mut rows, "policy-results", &p);
        let s = samples(&rows, "test", 1, &model, true, 2000).unwrap();
        assert_eq!(value(&s, "configured", "support", "EU-B"), -60000);
        assert_eq!(value(&s, "configured", "support", "EU-A"), 0);
    }

    #[test]
    fn last_approval_tracks_the_latest_command_and_rejection() {
        let (_, mut rows) = fixture();
        let command = |status| {
            Some(Approval {
                id: "cmd",
                status,
                committed_ms: 1000,
            })
        };
        let at = |latest, rejected, rows: &BTreeMap<_, _>| {
            last_approval(rows, latest, rejected, "test", 2000).value
        };
        assert_eq!(at(None, None, &rows), 0);
        assert_eq!(at(None, Some(500), &rows), 4);
        assert_eq!(at(command("committed"), None, &rows), 1);
        assert_eq!(at(command("committed"), Some(500), &rows), 1);
        assert_eq!(at(command("committed"), Some(1500), &rows), 4);
        assert_eq!(at(command("gateway_applied"), None, &rows), 2);
        assert_eq!(at(command("rejected"), None, &rows), 4);
        assert_eq!(at(command("recovery_confirmed"), None, &rows), 3);
        rows.insert(
            "recovery-results".into(),
            BTreeMap::from([(
                "cmd".into(),
                json!({"run_epoch":"test","applied_revision":2,"state":"confirmed","expires_ms":9000}),
            )]),
        );
        assert_eq!(at(command("recovery_confirmed"), None, &rows), 3);
        let sample = last_approval(&rows, command("recovery_confirmed"), None, "test", 2000);
        assert_eq!((sample.view, sample.valid_until_ms), ("last_approval", 9000));
        assert_eq!(at(command("recovery_confirmed"), None, &rows), 3);
        assert_eq!(
            last_approval(&rows, command("recovery_confirmed"), None, "test", 9500).value,
            3
        );
    }

    #[test]
    fn recovery_distinguishes_none_awaiting_and_verified() {
        let (model, mut rows) = fixture();
        let row = |state: &str| json!({"run_epoch":"test","applied_revision":1,"state":state,"expires_ms":9000});
        rows.insert(
            "recovery-results".into(),
            BTreeMap::from([("cmd".into(), row("unconfirmed"))]),
        );
        let s = samples(&rows, "test", 1, &model, true, 2000).unwrap();
        assert_eq!(value(&s, "recovery", "", ""), 0);
        rows.insert(
            "recovery-results".into(),
            BTreeMap::from([("cmd".into(), row("confirmed"))]),
        );
        let s = samples(&rows, "test", 1, &model, true, 2000).unwrap();
        assert_eq!(value(&s, "recovery", "", ""), 1);
        let s = samples(&rows, "test", 2, &model, true, 2000).unwrap();
        assert_eq!(value(&s, "recovery", "", ""), -1);
    }

    #[test]
    fn stale_disconnected_or_wrong_epoch_results_are_not_permission() {
        let (model, rows) = fixture();
        for (epoch, attached, now) in [
            ("test", false, 2000),
            ("other", true, 2000),
            ("test", true, 10000),
        ] {
            let s = samples(&rows, epoch, 1, &model, attached, now).unwrap();
            assert_eq!(value(&s, "policy", "support", "EU-A"), -1);
            assert_eq!(value(&s, "candidate", "", ""), -1);
            assert_eq!(value(&s, "endpoint", "", "EU-B"), -1);
            assert_eq!(value(&s, "proposed", "support", "EU-B"), -1);
        }
    }
}
