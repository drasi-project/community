//! From input rows to output elements: plain Rust, no host types.
//!
//! The component keeps the current rows of each input port and hands a copy to
//! its worker, which calls [`elements`]. Everything the plugin emits is decided
//! here: reading rows through the configured property names, the basis and
//! candidate hashes, and the `Plan`/`Assignment` or `LossScenario`/`Resilience`
//! elements (plan/contracts.md, Element contracts).
use crate::{
    config::{Config, Mode},
    solve::{self, Bin, Eligible, Health, Item, Outcome, Problem, Safety, State},
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Instant};

pub const PLAN_LABEL: &str = "Plan";
pub const ASSIGNMENT_LABEL: &str = "Assignment";
pub const LOSS_LABEL: &str = "LossScenario";
pub const RESILIENCE_LABEL: &str = "Resilience";
pub const PLAN_ID: &str = "plan";
pub const RESILIENCE_ID: &str = "resilience";

/// Current rows of each input port, plus the ports with no incoming edge.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inputs {
    pub items: Vec<Value>,
    pub bins: Vec<Value>,
    pub eligibility: Vec<Value>,
    pub unbound: Vec<String>,
}

/// One emitted graph node: its label and properties (a JSON object).
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub label: &'static str,
    pub properties: Value,
}
/// Element ID -> element; the full intended output state.
pub type Elements = BTreeMap<String, Element>;

/// The run and scenario revision the inputs belong to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    pub epoch: Value,
    pub revision: Value,
}

/// Lowercase hex SHA-256 of canonical JSON (sorted keys, no whitespace).
pub fn hash(value: &Value) -> String {
    let mut text = String::new();
    canonical(value, &mut text);
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (n, key) in keys.into_iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                canonical(&map[key], out);
            }
            out.push('}');
        }
        Value::Array(values) => {
            out.push('[');
            for (n, v) in values.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                canonical(v, out);
            }
            out.push(']');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}

/// Read the rows through the configuration. `Err` is the reason the answer is unknown.
pub fn problem(config: &Config, inputs: &Inputs) -> Result<(Problem, Context), (String, Context)> {
    let mut context = Context::default();
    if let Some(port) = inputs.unbound.first() {
        return Err((format!("No input is connected to port {port}"), context));
    }
    let epoch = &config.epoch_property;
    let items_config = &config.items;
    let mut items = BTreeMap::new();
    let mut epochs = Vec::new();
    let mut revisions = Vec::new();
    for row in &inputs.items {
        let key = text(row, &items_config.key).ok_or_else(|| {
            (format!("An item has no {}", items_config.key), context.clone())
        })?;
        let demand = integer(row, &items_config.demand).ok_or_else(|| {
            (format!("Demand of {key} is not a whole number"), context.clone())
        })?;
        let current = text(row, &items_config.current);
        epochs.push(row.get(epoch).cloned().unwrap_or(Value::Null));
        revisions.push(row.get(&items_config.revision).cloned().unwrap_or(Value::Null));
        if items.insert(key.clone(), Item { key: key.clone(), demand, current }).is_some() {
            return Err((format!("Item {key} appears twice"), context));
        }
    }
    if items.is_empty() {
        return Err(("No items yet".into(), context));
    }
    if epochs.iter().any(|e| e != &epochs[0]) || epochs[0].is_null() {
        return Err(("Items disagree on the run".into(), context));
    }
    context.epoch = epochs[0].clone();
    if revisions.iter().any(|r| r != &revisions[0]) || revisions[0].is_null() {
        return Err(("Items disagree on the scenario revision".into(), context));
    }
    context.revision = revisions[0].clone();
    let other_run = |row: &Value| row.get(epoch).is_some_and(|e| !e.is_null() && e != &context.epoch);

    let bins_config = &config.bins;
    let mut bins = BTreeMap::new();
    for row in &inputs.bins {
        let key = text(row, &bins_config.key)
            .ok_or_else(|| (format!("A bin has no {}", bins_config.key), context.clone()))?;
        if other_run(row) {
            return Err((format!("{key} belongs to another run"), context));
        }
        let capacity = integer(row, &bins_config.capacity).ok_or_else(|| {
            (format!("Capacity of {key} is not a whole number"), context.clone())
        })?;
        let status = text(row, &bins_config.health.property);
        let health = match status {
            Some(s) if bins_config.health.healthy.contains(&s) => Health::Healthy,
            Some(s) if bins_config.health.unhealthy.contains(&s) => Health::Unhealthy,
            _ => Health::Unknown,
        };
        if bins.insert(key.clone(), Bin { key: key.clone(), capacity, health }).is_some() {
            return Err((format!("Bin {key} appears twice"), context));
        }
    }
    if bins.is_empty() {
        return Err(("No bins yet".into(), context));
    }

    let e = &config.eligibility;
    let mut eligibility: BTreeMap<String, BTreeMap<String, Eligible>> = BTreeMap::new();
    for row in &inputs.eligibility {
        let (Some(item), Some(bin)) = (text(row, &e.item_key), text(row, &e.bin_key)) else {
            return Err(("An eligibility row has no item or bin".into(), context));
        };
        if !items.contains_key(&item) || !bins.contains_key(&bin) {
            continue;
        }
        if other_run(row) {
            return Err((format!("Policy for {item} on {bin} belongs to another run"), context));
        }
        let state = text(row, &e.state);
        let allowed = row.get(&e.allowed).and_then(Value::as_bool);
        let eligible = match (state.as_deref(), allowed) {
            (Some("allowed"), Some(true)) => Eligible::Allowed,
            (Some("denied"), Some(false)) => Eligible::Denied,
            _ => Eligible::Unknown,
        };
        if eligibility.entry(item.clone()).or_default().insert(bin.clone(), eligible).is_some() {
            return Err((format!("Policy for {item} on {bin} appears twice"), context));
        }
    }
    Ok((
        Problem {
            items: items.into_values().collect(),
            bins: bins.into_values().collect(),
            eligibility,
        },
        context,
    ))
}

/// Hash of the exact inputs solved (and the run and revision they belong to).
pub fn basis(problem: &Problem, context: &Context) -> String {
    hash(&json!({
        "run": context.epoch,
        "revision": context.revision,
        "items": problem.items,
        "bins": problem.bins,
        "eligibility": problem.eligibility,
    }))
}
/// When the inputs cannot be read, the basis is the hash of the raw rows.
fn raw_basis(inputs: &Inputs) -> String {
    let sorted = |rows: &[Value]| {
        let mut rows: Vec<String> = rows.iter().map(hash).collect();
        rows.sort();
        rows
    };
    hash(&json!({
        "items": sorted(&inputs.items),
        "bins": sorted(&inputs.bins),
        "eligibility": sorted(&inputs.eligibility),
        "unbound": inputs.unbound,
    }))
}

/// Everything this factory should currently emit for these inputs.
pub fn elements(mode: Mode, config: &Config, inputs: &Inputs, deadline: Instant, now_ms: u64) -> Elements {
    match problem(config, inputs) {
        Ok((problem, context)) => {
            let basis = basis(&problem, &context);
            match mode {
                Mode::Plan => plan_elements(
                    config,
                    &problem,
                    &context,
                    &basis,
                    solve::plan(&problem, &config.objective, deadline),
                    now_ms,
                ),
                Mode::Resilience => resilience_elements(
                    config,
                    &context,
                    &basis,
                    solve::resilience(&problem, &config.objective, deadline),
                ),
            }
        }
        Err((reason, context)) => unknown(mode, config, inputs, &context, reason, now_ms),
    }
}

/// The elements for an unknown answer (incomplete inputs, timeout, error).
pub fn unknown(
    mode: Mode,
    config: &Config,
    inputs: &Inputs,
    context: &Context,
    reason: String,
    now_ms: u64,
) -> Elements {
    let (basis, context) = match problem(config, inputs) {
        Ok((problem, context)) => (basis(&problem, &context), context),
        Err(_) => (raw_basis(inputs), context.clone()),
    };
    match mode {
        Mode::Plan => plan_elements(
            config,
            &Problem::default(),
            &context,
            &basis,
            Outcome::unknown(reason),
            now_ms,
        ),
        Mode::Resilience => resilience_elements(
            config,
            &context,
            &basis,
            solve::Resilience {
                state: Safety::Unknown,
                first_failure: String::new(),
                baseline: State::Unknown,
                unhealthy_bins: 0,
                losses: Vec::new(),
                reason,
            },
        ),
    }
}

fn plan_elements(
    config: &Config,
    problem: &Problem,
    context: &Context,
    basis: &str,
    outcome: Outcome,
    now_ms: u64,
) -> Elements {
    let feasible = outcome.state == State::Feasible;
    let candidate_id = if feasible {
        hash(&json!({"basis": basis, "assignments": outcome.assignments}))
    } else {
        String::new()
    };
    let mut elements = Elements::new();
    if feasible {
        for item in &problem.items {
            let bin = &outcome.assignments[&item.key];
            let mut properties = Map::new();
            properties.insert("item".into(), json!(item.key));
            properties.insert("bin".into(), json!(bin));
            properties.insert("current".into(), json!(item.current));
            properties.insert("moved".into(), json!(item.current.as_ref() != Some(bin)));
            properties.insert(config.items.demand.clone(), json!(item.demand));
            properties.insert("candidate_id".into(), json!(candidate_id));
            properties.insert(config.epoch_property.clone(), context.epoch.clone());
            elements.insert(
                item.key.clone(),
                Element { label: ASSIGNMENT_LABEL, properties: Value::Object(properties) },
            );
        }
    }
    let mut plan = Map::new();
    plan.insert("state".into(), json!(outcome.state));
    plan.insert("candidate_id".into(), json!(candidate_id));
    plan.insert("basis".into(), json!(basis));
    plan.insert("revision".into(), context.revision.clone());
    plan.insert("assignments".into(), json!(json!(outcome.assignments).to_string()));
    plan.insert("moves".into(), json!(outcome.moves));
    plan.insert("peak_utilisation".into(), json!(outcome.peak_utilisation));
    plan.insert("reason".into(), json!(outcome.reason));
    plan.insert(config.epoch_property.clone(), context.epoch.clone());
    plan.insert("solved_ms".into(), json!(now_ms));
    elements.insert(PLAN_ID.into(), Element { label: PLAN_LABEL, properties: Value::Object(plan) });
    elements
}

fn resilience_elements(
    config: &Config,
    context: &Context,
    basis: &str,
    outcome: solve::Resilience,
) -> Elements {
    let mut elements = Elements::new();
    let with_epoch = |mut value: Value| {
        value[config.epoch_property.as_str()] = context.epoch.clone();
        value
    };
    for loss in &outcome.losses {
        elements.insert(
            loss.lost.clone(),
            Element {
                label: LOSS_LABEL,
                properties: with_epoch(json!({
                    "lost": loss.lost,
                    "state": loss.outcome.state,
                    "assignments": json!(loss.outcome.assignments).to_string(),
                    "reason": loss.outcome.reason,
                    "basis": basis,
                })),
            },
        );
    }
    elements.insert(
        RESILIENCE_ID.into(),
        Element {
            label: RESILIENCE_LABEL,
            properties: with_epoch(json!({
                "state": outcome.state,
                "first_failure": outcome.first_failure,
                "baseline": outcome.baseline,
                "unhealthy_bins": outcome.unhealthy_bins,
                "reason": outcome.reason,
                "basis": basis,
                "revision": context.revision,
            })),
        },
    );
    elements
}

fn text(row: &Value, property: &str) -> Option<String> {
    match row.get(property)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
fn integer(row: &Value, property: &str) -> Option<i64> {
    match row.get(property)? {
        Value::Number(n) => n.as_i64().or_else(|| {
            n.as_f64()
                .filter(|f| f.fract() == 0.0 && f.abs() < 9e15)
                .map(|f| f as i64)
        }),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::time::Duration;

    pub fn config() -> Config {
        Config::parse(&serde_json::from_str(include_str!("../examples/ignite.json")).unwrap()).unwrap()
    }
    /// Rows as the contract's queries return them for today's seed (all healthy).
    pub fn seed() -> Inputs {
        let item = |id: &str, demand: i64, deployment: &str| {
            json!({"id": id, "run_epoch": "run-1", "revision": 1, "demand_tpm": demand, "deployment": deployment})
        };
        let bin = |id: &str| json!({"id": id, "run_epoch": "run-1", "capacity_tpm": 100_000, "status": "ok"});
        let mut eligibility = vec![];
        for (subject, allowed) in [("support", ["EU-A", "EU-B"].as_slice()), ("batch", ["EU-A", "EU-B", "US-C"].as_slice())] {
            for target in ["EU-A", "EU-B", "US-C"] {
                let ok = allowed.contains(&target);
                eligibility.push(json!({
                    "id": format!("{subject}/{target}"), "subject": subject, "target": target,
                    "state": if ok { "allowed" } else { "denied" }, "allowed": ok,
                    "reason": "", "policy_hash": "p", "basis": "b", "run_epoch": "run-1"
                }));
            }
        }
        Inputs {
            items: vec![item("support", 60_000, "EU-A"), item("batch", 80_000, "EU-B")],
            bins: vec![bin("EU-A"), bin("EU-B"), bin("US-C")],
            eligibility,
            unbound: vec![],
        }
    }
    pub fn row<'a>(rows: &'a mut [Value], id: &str) -> &'a mut Value {
        rows.iter_mut().find(|r| r["id"] == id).unwrap()
    }
    fn run(mode: Mode, inputs: &Inputs) -> Elements {
        elements(mode, &config(), inputs, Instant::now() + Duration::from_secs(2), 7)
    }
    fn plan(inputs: &Inputs) -> Value {
        run(Mode::Plan, inputs)[PLAN_ID].properties.clone()
    }
    fn resilience(inputs: &Inputs) -> Value {
        run(Mode::Resilience, inputs)[RESILIENCE_ID].properties.clone()
    }

    #[test]
    fn canonical_hash_sorts_keys_and_has_no_whitespace() {
        assert_eq!(hash(&json!({"b": 1, "a": [true, "x"]})), hash(&json!({"a": [true, "x"], "b": 1})));
        let mut text = String::new();
        canonical(&json!({"b": 1, "a": {"d": null, "c": "é"}}), &mut text);
        assert_eq!(text, r#"{"a":{"c":"é","d":null},"b":1}"#);
        assert_eq!(hash(&json!({})).len(), 64);
    }

    #[test]
    fn seed_plan_is_feasible_with_no_moves() {
        let out = run(Mode::Plan, &seed());
        let p = &out[PLAN_ID].properties;
        assert_eq!(p["state"], "feasible");
        assert_eq!(p["moves"], 0);
        assert_eq!(p["revision"], 1);
        assert_eq!(p["run_epoch"], "run-1");
        assert_eq!(p["assignments"], r#"{"batch":"EU-B","support":"EU-A"}"#);
        assert_eq!(p["candidate_id"].as_str().unwrap().len(), 64);
        let support = &out["support"];
        assert_eq!(support.label, ASSIGNMENT_LABEL);
        assert_eq!(support.properties["bin"], "EU-A");
        assert_eq!(support.properties["moved"], false);
        assert_eq!(support.properties["demand_tpm"], 60_000);
        assert_eq!(support.properties["candidate_id"], p["candidate_id"]);
        assert_eq!(resilience(&seed())["state"], "safe");
    }

    #[test]
    fn failure_plan_then_policy_edit_through_rows() {
        let mut inputs = seed();
        row(&mut inputs.bins, "EU-A")["status"] = json!("failed");
        let out = run(Mode::Plan, &inputs);
        let p = &out[PLAN_ID].properties;
        assert_eq!(p["state"], "feasible");
        assert_eq!(p["moves"], 2);
        assert_eq!(p["assignments"], r#"{"batch":"US-C","support":"EU-B"}"#);
        assert_eq!(out["batch"].properties["moved"], true);
        let r = resilience(&inputs);
        assert_eq!((r["state"].as_str(), r["first_failure"].as_str()), (Some("at_risk"), Some("EU-B")));
        assert_eq!(r["unhealthy_bins"], 1);
        assert_eq!(r["baseline"], "feasible");

        // The talk's policy edit: EU-B is no longer an allowed destination for support.
        let edit = row(&mut inputs.eligibility, "support/EU-B");
        edit["state"] = json!("denied");
        edit["allowed"] = json!(false);
        let out = run(Mode::Plan, &inputs);
        let p = &out[PLAN_ID].properties;
        assert_eq!(p["state"], "infeasible");
        assert_eq!(p["candidate_id"], "");
        assert_eq!(p["assignments"], "{}");
        assert_eq!(out.len(), 1, "no Assignment while infeasible");
        assert_eq!(resilience(&inputs)["state"], "at_risk");
    }

    #[test]
    fn capacity_beat_through_rows() {
        let mut inputs = seed();
        row(&mut inputs.items, "support")["deployment"] = json!("EU-B");
        row(&mut inputs.items, "batch")["deployment"] = json!("US-C");
        row(&mut inputs.bins, "EU-A")["capacity_tpm"] = json!(50_000);
        let r = resilience(&inputs);
        assert_eq!((r["state"].as_str(), r["first_failure"].as_str()), (Some("at_risk"), Some("EU-B")));
        let losses = run(Mode::Resilience, &inputs);
        assert_eq!(losses["EU-B"].label, LOSS_LABEL);
        assert_eq!(losses["EU-B"].properties["state"], "infeasible");
        assert_eq!(losses["EU-A"].properties["state"], "feasible");
        assert_eq!(losses["EU-B"].properties["basis"], r["basis"]);
        row(&mut inputs.bins, "EU-A")["capacity_tpm"] = json!(100_000);
        assert_eq!(resilience(&inputs)["state"], "safe");
    }

    #[test]
    fn incomplete_or_inconsistent_inputs_are_unknown() {
        let unknown = |inputs: &Inputs| {
            let p = plan(inputs);
            assert_eq!(p["state"], "unknown", "{p}");
            assert_eq!(resilience(inputs)["state"], "unknown");
            p["reason"].as_str().unwrap().to_owned()
        };
        let mut i = seed();
        row(&mut i.bins, "EU-B")["status"] = json!("unknown");
        assert_eq!(unknown(&i), "Health of EU-B is unknown");
        let mut i = seed();
        row(&mut i.items, "batch")["revision"] = json!(2);
        assert_eq!(unknown(&i), "Items disagree on the scenario revision");
        let mut i = seed();
        row(&mut i.bins, "US-C")["run_epoch"] = json!("run-0");
        unknown(&i);
        let mut i = seed();
        i.eligibility.retain(|r| r["id"] != "batch/US-C");
        assert_eq!(unknown(&i), "Policy for batch on US-C is unknown");
        let mut i = seed();
        row(&mut i.eligibility, "batch/US-C")["state"] = json!("unknown");
        unknown(&i);
        let mut i = seed();
        i.unbound = vec!["eligibility".into()];
        assert_eq!(unknown(&i), "No input is connected to port eligibility");
        let mut i = seed();
        i.items.clear();
        unknown(&i);
        // Unknown still names the run, the revision and the exact inputs.
        let mut i = seed();
        row(&mut i.bins, "EU-B")["status"] = json!("unknown");
        let p = plan(&i);
        assert_eq!((p["revision"].clone(), p["run_epoch"].clone()), (json!(1), json!("run-1")));
        assert_ne!(p["basis"], plan(&seed())["basis"]);
    }

    #[test]
    fn basis_ignores_row_order_and_unread_columns_but_not_inputs() {
        let a = seed();
        let mut b = seed();
        b.items.reverse();
        b.eligibility.reverse();
        for r in &mut b.eligibility {
            r["reason"] = json!("reworded");
        }
        assert_eq!(plan(&a)["basis"], plan(&b)["basis"]);
        assert_eq!(plan(&a)["candidate_id"], plan(&b)["candidate_id"]);
        row(&mut b.items, "support")["demand_tpm"] = json!(60_001);
        assert_ne!(plan(&a)["basis"], plan(&b)["basis"]);
        // Throttled and failed are both unhealthy: same problem, same basis.
        let mut c = seed();
        row(&mut c.bins, "EU-A")["status"] = json!("failed");
        let mut d = seed();
        row(&mut d.bins, "EU-A")["status"] = json!("throttled");
        assert_eq!(plan(&c)["basis"], plan(&d)["basis"]);
    }
}
