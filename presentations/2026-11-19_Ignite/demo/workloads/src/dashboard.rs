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
        push(
            view,
            "",
            "",
            match decision.map(|d| d.state.as_str()) {
                Some("feasible") => 1,
                Some("infeasible") => 0,
                _ => -1,
            },
        );
    }
    push("context", "", "", if coherent { 1 } else { -1 });
    push(
        "approval",
        "",
        "",
        allocation.map_or(-1, |d| {
            if !d.valid {
                2
            } else {
                i64::from(
                    model
                        .workloads
                        .iter()
                        .any(|w| d.assignments.get(&w.id) != Some(&w.destination)),
                )
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
            push(
                "configured",
                &workload.id,
                &endpoint.id,
                if workload.destination == endpoint.id {
                    workload.reservation_tpm
                } else {
                    0
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

#[cfg(test)]
mod tests {
    use super::*;
    use ignite_domain::decision::{allocate, policy};
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
        let r = allocate(p.clone(), Some("EU-A")).unwrap();
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
