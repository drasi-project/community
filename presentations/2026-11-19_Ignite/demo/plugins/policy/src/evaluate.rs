//! Rego evaluation for one subject × target pair: plain Rust, no host types.
//! The component (lib.rs) keeps the input rows and calls this per pair.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// What the rule returned for one pair. `Unknown` is never permission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Assessment {
    Allowed { reason: String },
    Denied { reason: String },
    Unknown { reason: String },
}

pub struct Evaluator {
    engine: regorus::Engine,
    rule: String,
    /// sha256 of module and rule; carried on every decision as `policy_hash`.
    pub policy_hash: String,
}

impl Evaluator {
    pub fn new(module: &str, rule: &str) -> Result<Self> {
        let mut engine = regorus::Engine::new();
        engine
            .add_policy("policy.rego".into(), module.into())
            .context("compile Rego module")?;
        let policy_hash = format!("{:x}", Sha256::digest(format!("{rule}\n{module}")));
        Ok(Self { engine, rule: rule.into(), policy_hash })
    }

    /// `input` is `{"subject": row, "target": row, "parameters": [rows]}`.
    /// Any error or a result not shaped `{"allowed": bool, "reason": string}`
    /// yields `Unknown`. The time bound is enforced by the caller.
    pub fn evaluate(&mut self, subject: &Value, target: &Value, parameters: &[Value]) -> Assessment {
        let input = json!({"subject": subject, "target": target, "parameters": parameters});
        let result = (|| -> Result<Value> {
            self.engine.set_input(regorus::Value::from_json_str(&input.to_string())?);
            Ok(serde_json::from_str(&self.engine.eval_rule(self.rule.clone())?.to_json_str()?)?)
        })();
        match result {
            Ok(value) => match (value["allowed"].as_bool(), value["reason"].as_str()) {
                (Some(true), Some(reason)) => Assessment::Allowed { reason: reason.into() },
                (Some(false), Some(reason)) => Assessment::Denied { reason: reason.into() },
                _ => Assessment::Unknown { reason: format!("rule returned {value}") },
            },
            Err(error) => Assessment::Unknown { reason: format!("policy evaluation failed: {error:#}") },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODULE: &str = include_str!("../ignite.rego");

    fn assess(workload: &str, deployment: &str, allowed: &[&str]) -> Assessment {
        let workloads = json!({
            "support": {"id":"support","data_class":"restricted","region_requirement":"EU","model":"support-model","demand_tpm":60000},
            "batch": {"id":"batch","data_class":"public","region_requirement":"any","model":"support-model","demand_tpm":80000},
        });
        let deployments = json!({
            "EU-A": {"id":"EU-A","region":"EU","model":"support-model","capacity_tpm":100000},
            "EU-B": {"id":"EU-B","region":"EU","model":"support-model","capacity_tpm":100000},
            "US-C": {"id":"US-C","region":"US","model":"support-model","capacity_tpm":100000},
        });
        let parameters: Vec<_> = allowed
            .iter()
            .map(|d| json!({"id":format!("{workload}/{d}"),"workload":workload,"deployment":d}))
            .collect();
        Evaluator::new(MODULE, "data.ignite.assessment")
            .unwrap()
            .evaluate(&workloads[workload], &deployments[deployment], &parameters)
    }

    #[test]
    fn example_policy_reproduces_todays_decisions() {
        let support = ["EU-A", "EU-B"];
        let batch = ["EU-A", "EU-B", "US-C"];
        assert!(matches!(assess("support", "EU-B", &support), Assessment::Allowed { .. }));
        assert!(matches!(assess("batch", "US-C", &batch), Assessment::Allowed { .. }));
        // Restricted data stays in the EU even if an operator allows US-C.
        assert_eq!(
            assess("support", "US-C", &["EU-A", "EU-B", "US-C"]),
            Assessment::Denied { reason: "restricted data must stay in its region".into() }
        );
        // The talk's policy edit: EU-B removed from support's destinations.
        assert_eq!(
            assess("support", "EU-B", &["EU-A"]),
            Assessment::Denied { reason: "destination excluded by current policy".into() }
        );
    }

    #[test]
    fn malformed_results_are_unknown_never_allowed() {
        let mut evaluator = Evaluator::new("package t\nimport rego.v1\nx := 1", "data.t.x").unwrap();
        assert!(matches!(evaluator.evaluate(&json!({}), &json!({}), &[]), Assessment::Unknown { .. }));
        assert!(Evaluator::new("not rego", "data.t.x").is_err());
    }
}
