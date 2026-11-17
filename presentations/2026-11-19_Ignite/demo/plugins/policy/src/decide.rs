//! The policy model: current input rows in, one `PolicyDecision` per
//! subject × target out, and only the decisions that changed. Plain Rust, no
//! host types; the component (lib.rs) feeds it rows and encodes its changes.
use crate::{
    evaluate::{Assessment, Evaluator},
    Config,
};
use anyhow::Result;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Duration};

/// Every current row of the three input ports. `faults` names rows that
/// could not be read; any fault makes every decision `unknown`.
#[derive(Clone, Debug, Default)]
pub struct Tables {
    pub subjects: Vec<Value>,
    pub targets: Vec<Value>,
    pub parameters: Vec<Value>,
    pub faults: Vec<String>,
}

/// One output change, in sorted `id` order.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// `replaces` is true when a decision with this `id` was already emitted.
    Upsert { id: String, properties: Value, replaces: bool },
    Delete { id: String },
}

pub struct Policy {
    config: Config,
    evaluator: Evaluator,
    current: BTreeMap<String, Value>,
}

impl Policy {
    /// Compiles the module; an invalid module is a configuration error.
    pub fn new(config: Config) -> Result<Self> {
        let evaluator = Evaluator::new(&config.module, &config.rule, Duration::from_millis(config.timeout_ms))?;
        Ok(Self { config, evaluator, current: BTreeMap::new() })
    }

    pub fn policy_hash(&self) -> &str {
        &self.evaluator.policy_hash
    }

    /// The decisions last returned, by `id`.
    pub fn current(&self) -> &BTreeMap<String, Value> {
        &self.current
    }

    /// Decide every pair for `tables` and return what changed since the last call.
    pub fn update(&mut self, tables: &Tables) -> Vec<Change> {
        let next = self.decide(tables);
        let mut changes = Vec::new();
        for (id, properties) in &next {
            let previous = self.current.get(id);
            if previous != Some(properties) {
                changes.push(Change::Upsert {
                    id: id.clone(),
                    properties: properties.clone(),
                    replaces: previous.is_some(),
                });
            }
        }
        changes.extend(
            self.current
                .keys()
                .filter(|id| !next.contains_key(*id))
                .map(|id| Change::Delete { id: id.clone() }),
        );
        changes.sort_by(|a, b| change_id(a).cmp(change_id(b)));
        self.current = next;
        changes
    }

    fn decide(&mut self, tables: &Tables) -> BTreeMap<String, Value> {
        let mut faults = tables.faults.clone();
        let subjects = group(&tables.subjects, &self.config.subjects.key, "subjects", &mut faults);
        let targets = group(&tables.targets, &self.config.targets.key, "targets", &mut faults);
        let mut parameters: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for row in &tables.parameters {
            let subject = key(row, &self.config.parameters.subject_key);
            let target = key(row, &self.config.parameters.target_key);
            match (subject, target) {
                (Some(subject), Some(_)) => parameters.entry(subject).or_default().push(row.clone()),
                _ => faults.push(format!(
                    "a parameters row has no '{}' or '{}'",
                    self.config.parameters.subject_key, self.config.parameters.target_key
                )),
            }
        }
        for rows in parameters.values_mut() {
            rows.sort_by_cached_key(canonical);
            rows.dedup();
        }
        faults.sort();
        faults.dedup();
        let no_parameters = Vec::new();
        let epoch = self.config.epoch_property.clone();
        let mut decisions = BTreeMap::new();
        for (subject_key, subject_rows) in &subjects {
            for (target_key, target_rows) in &targets {
                let id = format!("{subject_key}/{target_key}");
                let subject = &subject_rows[0];
                let target = &target_rows[0];
                let params = parameters.get(subject_key).unwrap_or(&no_parameters);
                let run_epoch = subject.get(&epoch).cloned().unwrap_or(Value::Null);
                let basis = sha256(&json!({
                    "policy_hash": self.evaluator.policy_hash,
                    "subject": subject,
                    "target": target,
                    "parameters": params,
                }));
                let incomplete = if let Some(fault) = faults.first() {
                    Some(format!("policy inputs are incomplete: {fault}"))
                } else if subject_rows.len() > 1 {
                    Some(format!("{} subject rows share the key '{subject_key}'", subject_rows.len()))
                } else if target_rows.len() > 1 {
                    Some(format!("{} target rows share the key '{target_key}'", target_rows.len()))
                } else if differs(&run_epoch, target.get(&epoch)) {
                    Some("subject and target rows are from different runs".to_owned())
                } else if params.iter().any(|row| differs(&run_epoch, row.get(&epoch))) {
                    Some("policy parameters are from a different run".to_owned())
                } else {
                    None
                };
                // Evaluation is deterministic for the same basis, so a settled
                // answer is reused; an unknown one is always retried.
                let reuse = self
                    .current
                    .get(&id)
                    .filter(|d| incomplete.is_none() && d["basis"] == basis.as_str() && d["state"] != "unknown");
                let properties = if let Some(previous) = reuse {
                    previous.clone()
                } else {
                    let assessment = match incomplete {
                        Some(reason) => Assessment::Unknown { reason },
                        None => self.evaluator.evaluate(subject, target, params),
                    };
                    let (state, reason) = match assessment {
                        Assessment::Allowed { reason } => ("allowed", reason),
                        Assessment::Denied { reason } => ("denied", reason),
                        Assessment::Unknown { reason } => ("unknown", reason),
                    };
                    json!({
                        "id": id,
                        "subject": subject_key,
                        "target": target_key,
                        "state": state,
                        "allowed": state == "allowed",
                        "reason": reason,
                        "policy_hash": self.evaluator.policy_hash,
                        "basis": basis,
                        "run_epoch": run_epoch,
                    })
                };
                decisions.insert(id, properties);
            }
        }
        decisions
    }
}

fn change_id(change: &Change) -> &str {
    match change {
        Change::Upsert { id, .. } | Change::Delete { id } => id,
    }
}

/// Rows by key; distinct rows sharing a key stay together so the pair can be
/// reported as unknown rather than picking one.
fn group(rows: &[Value], property: &str, port: &str, faults: &mut Vec<String>) -> BTreeMap<String, Vec<Value>> {
    let mut grouped: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for row in rows {
        match key(row, property) {
            Some(key) => grouped.entry(key).or_default().push(row.clone()),
            None => faults.push(format!("a {port} row has no '{property}'")),
        }
    }
    for rows in grouped.values_mut() {
        rows.sort_by_cached_key(canonical);
        rows.dedup();
    }
    grouped
}

fn key(row: &Value, property: &str) -> Option<String> {
    match row.get(property)? {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Two run epochs differ only when both are present.
fn differs(epoch: &Value, other: Option<&Value>) -> bool {
    match other {
        Some(other) if !other.is_null() && !epoch.is_null() => !same_number_or_value(epoch, other),
        _ => false,
    }
}

fn same_number_or_value(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}

/// Lowercase hex SHA-256 of canonical JSON.
pub fn sha256(value: &Value) -> String {
    format!("{:x}", Sha256::digest(canonical(value)))
}

/// JSON with object keys sorted and no whitespace, whatever serde_json's map order.
pub fn canonical(value: &Value) -> String {
    fn write(value: &Value, out: &mut String) {
        match value {
            Value::Object(map) => {
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();
                out.push('{');
                for (index, key) in keys.into_iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&Value::String(key.clone()).to_string());
                    out.push(':');
                    write(&map[key], out);
                }
                out.push('}');
            }
            Value::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write(item, out);
                }
                out.push(']');
            }
            other => out.push_str(&other.to_string()),
        }
    }
    let mut out = String::new();
    write(value, &mut out);
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn config(module: &str) -> Config {
        let mut value: Value = serde_json::from_str(include_str!("../examples/ignite.json")).unwrap();
        value["module"] = json!(module);
        Config::parse(&value).unwrap()
    }

    pub(crate) fn workload(id: &str) -> Value {
        match id {
            "support" => json!({"id":"support","run_epoch":1,"data_class":"restricted","region_requirement":"EU","model":"support-model","demand_tpm":60000}),
            _ => json!({"id":"batch","run_epoch":1,"data_class":"public","region_requirement":"any","model":"support-model","demand_tpm":80000}),
        }
    }

    pub(crate) fn deployment(id: &str) -> Value {
        let region = if id.starts_with("EU") { "EU" } else { "US" };
        json!({"id":id,"run_epoch":1,"region":region,"model":"support-model","capacity_tpm":100000})
    }

    pub(crate) fn destination(workload: &str, deployment: &str) -> Value {
        json!({"id":format!("{workload}/{deployment}"),"run_epoch":1,"workload":workload,"deployment":deployment})
    }

    /// Today's seed (reset.sql).
    pub(crate) fn seed() -> Tables {
        Tables {
            subjects: vec![workload("support"), workload("batch")],
            targets: ["EU-A", "EU-B", "US-C"].map(deployment).to_vec(),
            parameters: [("support", "EU-A"), ("support", "EU-B"), ("batch", "EU-A"), ("batch", "EU-B"), ("batch", "US-C")]
                .map(|(w, d)| destination(w, d))
                .to_vec(),
            faults: vec![],
        }
    }

    fn policy() -> Policy {
        Policy::new(config(include_str!("../ignite.rego"))).unwrap()
    }

    fn states(policy: &Policy) -> Vec<(String, String)> {
        policy
            .current()
            .iter()
            .map(|(id, d)| (id.clone(), d["state"].as_str().unwrap().to_owned()))
            .collect()
    }

    fn ids(changes: &[Change]) -> Vec<&str> {
        changes.iter().map(change_id).collect()
    }

    #[test]
    fn seed_reproduces_todays_grid() {
        let mut policy = policy();
        let changes = policy.update(&seed());
        assert_eq!(
            ids(&changes),
            ["batch/EU-A", "batch/EU-B", "batch/US-C", "support/EU-A", "support/EU-B", "support/US-C"]
        );
        assert!(changes.iter().all(|c| matches!(c, Change::Upsert { replaces: false, .. })));
        let expected = [
            ("batch/EU-A", "allowed"),
            ("batch/EU-B", "allowed"),
            ("batch/US-C", "allowed"),
            ("support/EU-A", "allowed"),
            ("support/EU-B", "allowed"),
            ("support/US-C", "denied"),
        ]
        .map(|(a, b)| (a.to_owned(), b.to_owned()));
        assert_eq!(states(&policy), expected);
        let denied = &policy.current()["support/US-C"];
        assert_eq!(denied["allowed"], false);
        assert_eq!(denied["subject"], "support");
        assert_eq!(denied["target"], "US-C");
        assert_eq!(denied["run_epoch"], 1);
        assert_eq!(denied["policy_hash"], policy.policy_hash());
        assert_eq!(denied["basis"].as_str().unwrap().len(), 64);
        // Nothing changed, nothing emitted.
        assert!(policy.update(&seed()).is_empty());
        // Row order does not matter.
        let mut shuffled = seed();
        shuffled.subjects.reverse();
        shuffled.parameters.reverse();
        assert!(policy.update(&shuffled).is_empty());
    }

    #[test]
    fn talks_policy_edit_is_a_data_change() {
        let mut policy = policy();
        policy.update(&seed());
        let mut edited = seed();
        edited.parameters.retain(|row| row["id"] != "support/EU-B");
        let changes = policy.update(&edited);
        // Every support pair's basis changes; only support/EU-B changes state.
        assert_eq!(ids(&changes), ["support/EU-A", "support/EU-B", "support/US-C"]);
        assert!(changes.iter().all(|c| matches!(c, Change::Upsert { replaces: true, .. })));
        let decision = &policy.current()["support/EU-B"];
        assert_eq!(decision["state"], "denied");
        assert_eq!(decision["allowed"], false);
        assert_eq!(decision["reason"], "destination excluded by current policy");
        assert_eq!(policy.current()["support/EU-A"]["state"], "allowed");
        // Restoring it restores the seed decisions exactly.
        policy.update(&seed());
        let mut fresh = self::policy();
        fresh.update(&seed());
        assert_eq!(policy.current(), fresh.current());
    }

    #[test]
    fn rule_change_is_a_new_policy_hash() {
        let mut seeded = policy();
        seeded.update(&seed());
        let edited = include_str!("../ignite.rego").replace(
            "region_permitted if input.subject.region_requirement == \"any\"",
            "region_permitted if input.subject.region_requirement == \"any\"\n\nregion_permitted if input.target.region == \"US\"",
        );
        let mut changed = Policy::new(config(&edited)).unwrap();
        changed.update(&seed());
        assert_ne!(seeded.policy_hash(), changed.policy_hash());
        // Still denied: US-C is not in support's destinations.
        assert_eq!(changed.current()["support/US-C"]["state"], "denied");
        assert_eq!(changed.current()["support/US-C"]["reason"], "destination excluded by current policy");
        assert_ne!(seeded.current()["batch/EU-A"]["basis"], changed.current()["batch/EU-A"]["basis"]);
    }

    #[test]
    fn deleted_rows_delete_their_pairs() {
        let mut policy = policy();
        policy.update(&seed());
        let mut without = seed();
        without.targets.retain(|row| row["id"] != "US-C");
        let changes = policy.update(&without);
        assert_eq!(
            changes,
            [
                Change::Delete { id: "batch/US-C".into() },
                Change::Delete { id: "support/US-C".into() }
            ]
        );
        without.subjects.clear();
        assert_eq!(policy.update(&without).len(), 4);
        assert!(policy.current().is_empty());
    }

    #[test]
    fn incomplete_inputs_are_unknown_never_allowed() {
        let unknown = |tables: Tables, id: &str| {
            let mut policy = policy();
            policy.update(&tables);
            let decision = policy.current()[id].clone();
            assert_eq!(decision["state"], "unknown", "{decision}");
            assert_eq!(decision["allowed"], false);
            decision["reason"].as_str().unwrap().to_owned()
        };
        let mut tables = seed();
        tables.parameters.push(json!({"id":"x","run_epoch":1}));
        assert!(unknown(tables, "batch/EU-A").contains("parameters row"));

        let mut tables = seed();
        tables.faults.push("undecodable row".into());
        assert!(unknown(tables, "support/EU-A").contains("undecodable row"));

        let mut tables = seed();
        let mut other = workload("support");
        other["demand_tpm"] = json!(1);
        tables.subjects.push(other);
        assert!(unknown(tables, "support/EU-A").contains("share the key"));

        let mut tables = seed();
        tables.targets[0]["run_epoch"] = json!(2);
        assert!(unknown(tables, "support/EU-A").contains("different runs"));

        let mut tables = seed();
        tables.parameters[0]["run_epoch"] = json!(2);
        assert!(unknown(tables, "support/EU-B").contains("different run"));

        // A rule that does not return the expected shape.
        let mut policy = Policy::new(config("package ignite\nimport rego.v1\nassessment := {\"allowed\": true}")).unwrap();
        policy.update(&seed());
        assert!(policy.current().values().all(|d| d["state"] == "unknown" && d["allowed"] == false));
    }

    #[test]
    fn unknown_recovers_when_inputs_complete() {
        let mut policy = policy();
        let mut faulty = seed();
        faulty.faults.push("undecodable row".into());
        policy.update(&faulty);
        let changes = policy.update(&seed());
        assert_eq!(changes.len(), 6);
        assert_eq!(policy.current()["support/EU-A"]["state"], "allowed");
    }

    #[test]
    fn canonical_json_sorts_keys() {
        assert_eq!(canonical(&json!({"b":1,"a":[{"d":true,"c":"x"}]})), r#"{"a":[{"c":"x","d":true}],"b":1}"#);
    }
}
