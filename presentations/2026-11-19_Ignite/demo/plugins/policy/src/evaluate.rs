//! Rego evaluation for one subject × target pair: plain Rust, no host types.
//! The component (lib.rs) keeps the input rows and calls this per pair.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    num::NonZeroU32,
    time::{Duration, Instant},
};

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
    timeout: Duration,
    /// sha256 of module and rule; carried on every decision as `policy_hash`.
    pub policy_hash: String,
}

impl Evaluator {
    /// Compiles `module`; `timeout` bounds each pair's evaluation.
    pub fn new(module: &str, rule: &str, timeout: Duration) -> Result<Self> {
        let mut engine = regorus::Engine::new();
        engine
            .add_policy("policy.rego".into(), module.into())
            .context("compile Rego module")?;
        // Cooperative limit inside regorus: checked on every unit of work.
        engine.set_execution_timer_config(regorus::utils::limits::ExecutionTimerConfig {
            limit: timeout,
            check_interval: NonZeroU32::MIN,
        });
        let policy_hash = format!("{:x}", Sha256::digest(format!("{rule}\n{module}")));
        Ok(Self { engine, rule: rule.into(), timeout, policy_hash })
    }

    /// `input` is `{"subject": row, "target": row, "parameters": [rows]}`.
    /// Any error, an evaluation longer than the time bound, or a result not
    /// shaped `{"allowed": bool, "reason": string}` yields `Unknown`. The bound
    /// is checked between regorus's units of work, so it stops iteration; a
    /// single builtin call runs to completion and is then reported as unknown.
    pub fn evaluate(&mut self, subject: &Value, target: &Value, parameters: &[Value]) -> Assessment {
        let input = json!({"subject": subject, "target": target, "parameters": parameters});
        let started = Instant::now();
        let result = (|| -> Result<Value> {
            self.engine.set_input(regorus::Value::from_json_str(&input.to_string())?);
            Ok(serde_json::from_str(&self.engine.eval_rule(self.rule.clone())?.to_json_str()?)?)
        })();
        if started.elapsed() > self.timeout {
            return Assessment::Unknown {
                reason: format!("policy evaluation exceeded {} ms", self.timeout.as_millis()),
            };
        }
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
        Evaluator::new(MODULE, "data.ignite.assessment", Duration::from_millis(250))
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
    fn todays_placement_cases() {
        let support = ["EU-A", "EU-B"];
        let batch = ["EU-A", "EU-B", "US-C"];
        let allowed = |w, d, a: &[&str]| matches!(assess(w, d, a), Assessment::Allowed { .. });
        // Seed: support on EU-A/EU-B only; batch anywhere.
        assert!(allowed("support", "EU-A", &support));
        assert!(allowed("support", "EU-B", &support));
        assert!(!allowed("support", "US-C", &support));
        for deployment in batch {
            assert!(allowed("batch", deployment, &batch));
        }
        // Not in the destinations and outside the region: both reasons, sorted.
        assert_eq!(
            assess("support", "US-C", &support),
            Assessment::Denied {
                reason: "destination excluded by current policy; restricted data must stay in its region".into()
            }
        );
        // After the talk's policy edit only support → EU-B changes.
        assert!(allowed("support", "EU-A", &["EU-A"]));
        assert!(!allowed("support", "EU-B", &["EU-A"]));
        assert_eq!(
            assess("support", "EU-A", &support),
            Assessment::Allowed { reason: "Rego permits classification, destination and model".into() }
        );
    }

    #[test]
    fn malformed_results_are_unknown_never_allowed() {
        let bound = Duration::from_millis(250);
        let mut evaluator = Evaluator::new("package t\nimport rego.v1\nx := 1", "data.t.x", bound).unwrap();
        assert!(matches!(evaluator.evaluate(&json!({}), &json!({}), &[]), Assessment::Unknown { .. }));
        // An undefined rule (no result) is unknown too.
        let mut evaluator = Evaluator::new("package t\nimport rego.v1\nx := 1", "data.t.missing", bound).unwrap();
        assert!(matches!(evaluator.evaluate(&json!({}), &json!({}), &[]), Assessment::Unknown { .. }));
        assert!(Evaluator::new("not rego", "data.t.x", bound).is_err());
    }

    #[test]
    fn slow_evaluation_is_unknown() {
        let module = "package t\nimport rego.v1\nn := count([1 | some x in numbers.range(1, 3000); some y in numbers.range(1, 3000)])\na := {\"allowed\": n > 0, \"reason\": \"slow\"}";
        let mut evaluator = Evaluator::new(module, "data.t.a", Duration::from_millis(5)).unwrap();
        let started = Instant::now();
        let result = evaluator.evaluate(&json!({}), &json!({}), &[]);
        assert!(matches!(result, Assessment::Unknown { .. }), "{result:?}");
        assert!(started.elapsed() < Duration::from_secs(2), "bound not enforced: {:?}", started.elapsed());
    }
}
