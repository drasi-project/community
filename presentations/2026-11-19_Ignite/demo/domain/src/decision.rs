use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const POLICY: &str = include_str!("../../policies/placement.rego");

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub endpoints: Vec<Endpoint>,
    pub workloads: Vec<Workload>,
    pub policy: Policy,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub revision: i64,
    pub destinations: BTreeMap<String, Vec<String>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub id: String,
    pub region: String,
    pub model: String,
    pub budget_tpm: i64,
    pub mode: String,
    pub reporting: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Workload {
    pub id: String,
    pub region: String,
    pub model: String,
    pub reservation_tpm: i64,
    pub destination: String,
}
impl Model {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.endpoints.len() == 3 && self.workloads.len() == 2,
            "expected complete bounded model"
        );
        for id in ["EU-A", "EU-B", "US-C"] {
            ensure!(
                self.endpoints.iter().filter(|e| e.id == id).count() == 1,
                "duplicate/missing endpoint"
            );
        }
        for id in ["support", "batch"] {
            ensure!(
                self.workloads.iter().filter(|w| w.id == id).count() == 1,
                "duplicate/missing workload"
            );
        }
        ensure!(
            self.policy.revision > 0 && self.policy.destinations.len() == 2,
            "invalid versioned policy"
        );
        for e in &self.endpoints {
            ensure!(
                e.region == if e.id == "US-C" { "US" } else { "EU" },
                "immutable physical region"
            );
            ensure!(
                !e.model.is_empty() && (0..=1_000_000_000).contains(&e.budget_tpm),
                "invalid endpoint contract"
            );
            ensure!(
                ["ok", "failed", "throttled"].contains(&e.mode.as_str()),
                "invalid fault mode"
            );
        }
        for w in &self.workloads {
            ensure!(
                w.region == if w.id == "support" { "EU" } else { "any" },
                "immutable data classification"
            );
            ensure!(
                !w.model.is_empty() && (1..=1_000_000_000).contains(&w.reservation_tpm),
                "invalid commitment"
            );
            ensure!(
                self.endpoints.iter().any(|e| e.id == w.destination),
                "unknown destination"
            );
            let destinations = self
                .policy
                .destinations
                .get(&w.id)
                .context("missing workload policy")?;
            ensure!(
                destinations.iter().collect::<BTreeSet<_>>().len() == destinations.len(),
                "duplicate policy destination"
            );
            ensure!(
                destinations
                    .iter()
                    .all(|id| self.endpoints.iter().any(|e| &e.id == id)),
                "unknown policy destination"
            );
        }
        Ok(())
    }

    pub fn allowed(&self, workload: &str, destination: &str) -> Result<bool> {
        Ok(self.eligibility(workload, destination)?["allowed"] == true)
    }

    pub fn eligibility(&self, workload: &str, destination: &str) -> Result<Value> {
        let w = self
            .workloads
            .iter()
            .find(|w| w.id == workload)
            .context("unknown workload")?;
        let e = self
            .endpoints
            .iter()
            .find(|e| e.id == destination)
            .context("unknown endpoint")?;
        let mut engine = regorus::Engine::new();
        engine.add_policy("placement.rego".into(), POLICY.into())?;
        engine.set_input(regorus::Value::from_json_str(
            &json!({
                "workload":w,"endpoint":e,"policy":self.policy
            })
            .to_string(),
        )?);
        Ok(serde_json::from_str(
            &engine
                .eval_rule("data.ignite.assessment".into())?
                .to_json_str()?,
        )?)
    }

    pub fn dispatch(&self, workload: &str, destination: &str) -> Result<&Endpoint> {
        ensure!(
            self.allowed(workload, destination)?,
            "current Rego policy prohibits destination"
        );
        let w = self
            .workloads
            .iter()
            .find(|w| w.id == workload)
            .context("unknown workload")?;
        let e = self
            .endpoints
            .iter()
            .find(|e| e.id == destination)
            .context("unknown endpoint")?;
        let reserved = self
            .workloads
            .iter()
            .filter(|other| other.id != w.id && other.destination == e.id)
            .map(|w| w.reservation_tpm)
            .sum::<i64>()
            + w.reservation_tpm;
        ensure!(
            reserved <= e.budget_tpm,
            "full workload commitments exceed destination budget"
        );
        Ok(e)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Evidence {
    pub id: String,
    pub run_epoch: String,
    pub status: String,
    pub expires_ms: i64,
    pub observed_ms: i64,
    pub sequence: i64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Decision {
    pub run_epoch: String,
    pub revision: i64,
    pub policy_revision: i64,
    pub basis: String,
    pub state: String,
    pub reason: String,
    pub model: Model,
    pub evidence: BTreeMap<String, Evidence>,
    pub eligibility: BTreeMap<String, BTreeMap<String, Value>>,
    pub assignments: BTreeMap<String, String>,
    pub candidate_id: String,
    pub valid: bool,
    pub failure: Option<String>,
}

pub fn hash(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

pub fn policy(rows: &BTreeMap<String, Value>) -> Result<Option<Decision>> {
    let Some(row) = rows.values().next() else {
        return Ok(None);
    };
    let decode_model = |value: &Value| -> Result<Model> {
        Ok(if let Some(text) = value.as_str() {
            serde_json::from_str(text)?
        } else {
            serde_json::from_value(value.clone())?
        })
    };
    let model = decode_model(&row["payload"])?;
    model.validate()?;
    let epoch = row["run_epoch"].as_str().context("epoch")?.to_owned();
    let revision = row["revision"].as_i64().context("revision")?;
    let mut evidence = BTreeMap::new();
    let mut coherent = true;
    for row in rows.values() {
        coherent &= row["run_epoch"] == epoch
            && row["revision"] == revision
            && serde_json::to_value(decode_model(&row["payload"])?)?
                == serde_json::to_value(&model)?;
        let e: Evidence = serde_json::from_value(row.clone())?;
        coherent &= ["ok", "failed", "throttled"].contains(&e.status.as_str());
        evidence.insert(e.id.clone(), e);
    }
    coherent &= evidence.len() == model.endpoints.len()
        && model.endpoints.iter().all(|e| evidence.contains_key(&e.id));
    let states: BTreeMap<_, _> = evidence.iter().map(|(id, e)| (id, &e.status)).collect();
    let basis = hash(
        &json!({"epoch":epoch,"revision":revision,"policy":model.policy,
        "states":states,"coherent":coherent}),
    )?;
    let mut eligibility = BTreeMap::new();
    for w in &model.workloads {
        let mut destinations = BTreeMap::new();
        for e in &model.endpoints {
            destinations.insert(e.id.clone(), model.eligibility(&w.id, &e.id)?);
        }
        eligibility.insert(w.id.clone(), destinations);
    }
    Ok(Some(Decision {
        run_epoch: epoch,
        revision,
        policy_revision: model.policy.revision,
        basis,
        state: if coherent { "fresh" } else { "unknown" }.into(),
        reason: if coherent {
            "complete same-basis query-maintained evidence"
        } else {
            "missing, mixed or expired evidence"
        }
        .into(),
        model,
        evidence,
        eligibility,
        assignments: BTreeMap::new(),
        candidate_id: String::new(),
        valid: false,
        failure: None,
    }))
}

/// Enumerate every complete assignment (at most 3^2), minimize moved commitments,
/// then break ties by sorted workload/destination identity. Never shed a workload.
pub fn allocate(mut decision: Decision, failure: Option<&str>) -> Result<Decision> {
    decision.failure = failure.map(str::to_owned);
    decision.assignments.clear();
    decision.valid = false;
    if decision.state == "unknown" {
        return Ok(decision);
    }
    let mut workloads = decision.model.workloads.iter().collect::<Vec<_>>();
    workloads.sort_by_key(|w| &w.id);
    let mut candidates = vec![(0usize, BTreeMap::new(), BTreeMap::<String, i64>::new())];
    for w in workloads {
        let mut next = vec![];
        for (moves, assigned, used) in candidates {
            for e in &decision.model.endpoints {
                if Some(e.id.as_str()) == failure
                    || decision.evidence[&e.id].status != "ok"
                    || decision.eligibility[&w.id][&e.id]["allowed"] != true
                {
                    continue;
                }
                let amount = used.get(&e.id).copied().unwrap_or(0) + w.reservation_tpm;
                if amount > e.budget_tpm {
                    continue;
                }
                let mut assigned = assigned.clone();
                assigned.insert(w.id.clone(), e.id.clone());
                let mut used = used.clone();
                used.insert(e.id.clone(), amount);
                next.push((moves + usize::from(w.destination != e.id), assigned, used));
            }
        }
        candidates = next;
    }
    candidates.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    if let Some((_, assignments, _)) = candidates.into_iter().next() {
        decision.assignments = assignments;
        decision.valid = failure.is_none();
        decision.state = "feasible".into();
        decision.reason = "complete assignment preserves every commitment under current Rego policy and observed availability".into();
    } else {
        decision.state = "infeasible".into();
        decision.reason =
            "no complete assignment satisfies policy, observed availability, model and capacity"
                .into();
    }
    decision.candidate_id = hash(&(&decision.basis, &decision.assignments, failure))?;
    Ok(decision)
}

pub fn fresh(decision: &Decision, now: i64) -> bool {
    decision.state != "unknown"
        && decision.evidence.len() == 3
        && decision.evidence.values().all(|e| {
            e.run_epoch == decision.run_epoch
                && e.expires_ms > now
                && e.observed_ms <= now
                && ["ok", "failed", "throttled"].contains(&e.status.as_str())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> BTreeMap<String, Value> {
        let sql = include_str!("../../database/reset.sql");
        let payload = sql
            .split("1, '")
            .nth(1)
            .unwrap()
            .split("'::jsonb")
            .next()
            .unwrap();
        let m: Value = serde_json::from_str(payload).unwrap();
        ["EU-A", "EU-B", "US-C"]
            .into_iter()
            .map(|id| {
                (
                    id.into(),
                    json!({
                        "id":id,"run_epoch":"test","revision":1,"payload":m,"status":"ok",
                        "expires_ms":9000,"observed_ms":1000,"sequence":1
                    }),
                )
            })
            .collect()
    }
    #[test]
    fn real_rego_complete_search_and_counterfactual() {
        let mut rows = fixture();
        rows.get_mut("EU-A").unwrap()["status"] = json!("failed");
        let p = policy(&rows).unwrap().unwrap();
        assert_eq!(p.eligibility["support"]["US-C"]["allowed"], false);
        let d = allocate(p.clone(), None).unwrap();
        assert_eq!(
            d.assignments,
            BTreeMap::from([
                ("support".into(), "EU-B".into()),
                ("batch".into(), "US-C".into())
            ])
        );
        assert!(d.valid);
        let mut variant = p;
        variant
            .model
            .endpoints
            .iter_mut()
            .find(|e| e.id == "US-C")
            .unwrap()
            .budget_tpm = 70_000;
        assert_eq!(allocate(variant, None).unwrap().state, "infeasible");
        let mut healthy = policy(&fixture()).unwrap().unwrap();
        healthy
            .model
            .workloads
            .iter_mut()
            .find(|w| w.id == "batch")
            .unwrap()
            .destination = "US-C".into();
        healthy
            .model
            .endpoints
            .iter_mut()
            .find(|e| e.id == "EU-B")
            .unwrap()
            .budget_tpm = 50_000;
        assert_eq!(allocate(healthy.clone(), None).unwrap().state, "feasible");
        assert_eq!(
            allocate(healthy.clone(), Some("EU-A")).unwrap().state,
            "infeasible"
        );
        healthy
            .model
            .endpoints
            .iter_mut()
            .find(|e| e.id == "EU-B")
            .unwrap()
            .budget_tpm = 100_000;
        let resilience = allocate(healthy, Some("EU-A")).unwrap();
        assert_eq!(resilience.state, "feasible");
        assert!(!resilience.valid);
    }
    #[test]
    fn evidence_and_policy_changes_are_not_heartbeat_revisions() {
        let mut rows = fixture();
        let old = policy(&rows).unwrap().unwrap();
        rows.get_mut("EU-A").unwrap()["sequence"] = json!(2);
        assert_eq!(old.basis, policy(&rows).unwrap().unwrap().basis);
        rows.get_mut("EU-A").unwrap()["status"] = json!("unknown");
        assert_eq!(
            allocate(policy(&rows).unwrap().unwrap(), None)
                .unwrap()
                .state,
            "unknown"
        );
        rows.remove("EU-A");
        assert_eq!(policy(&rows).unwrap().unwrap().state, "unknown");
        let mut m = old.model;
        m.policy
            .destinations
            .get_mut("support")
            .unwrap()
            .retain(|id| id != "EU-B");
        assert!(!m.allowed("support", "EU-B").unwrap());
        m.policy
            .destinations
            .get_mut("support")
            .unwrap()
            .push("US-C".into());
        assert!(!m.allowed("support", "US-C").unwrap());
    }

    #[test]
    fn postgres_json_strings_and_reordered_inputs_preserve_determinism() {
        let rows = fixture();
        let expected = allocate(policy(&rows).unwrap().unwrap(), Some("EU-A")).unwrap();
        let mut reordered = rows;
        for row in reordered.values_mut() {
            let mut model: Model = serde_json::from_value(row["payload"].clone()).unwrap();
            model.endpoints.reverse();
            model.workloads.reverse();
            row["payload"] = json!(serde_json::to_string(&model).unwrap());
        }
        let actual = allocate(policy(&reordered).unwrap().unwrap(), Some("EU-A")).unwrap();
        assert_eq!(actual.assignments, expected.assignments);
        assert_eq!(actual.state, expected.state);
    }

    #[test]
    fn model_compatibility_and_immutable_classification_are_enforced() {
        let mut rows = fixture();
        for row in rows.values_mut() {
            row["payload"]["endpoints"][2]["model"] = json!("another-model");
            row["status"] = if row["id"] == "EU-A" {
                json!("failed")
            } else {
                json!("ok")
            };
        }
        let p = policy(&rows).unwrap().unwrap();
        assert_eq!(allocate(p.clone(), None).unwrap().state, "infeasible");
        let mut m = p.model;
        m.workloads[0].region = "any".into();
        assert!(m.validate().is_err());
        m.workloads[0].region = "EU".into();
        m.endpoints[2].region = "EU".into();
        assert!(m.validate().is_err());
    }
}
