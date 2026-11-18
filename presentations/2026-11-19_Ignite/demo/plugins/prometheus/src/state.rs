//! The exporter's current rows per output alias and their rendering as
//! Prometheus gauges: plain Rust, no host types (see plan/contracts.md §8).
//!
//! Each configured output is in one of three states:
//!
//! - **Unattached**: the graph has no edge from that output into this sink.
//! - **Loading**: waiting for the output's snapshot. Live changes are buffered
//!   (bounded) with their producer sequence and replayed on top of the snapshot.
//! - **Ready**: the snapshot is loaded and every later change is applied.
//!
//! Only Ready outputs export series. `drasi_reaction_input_ready{output}` is
//! 1 for Ready and 0 otherwise, for every configured output.
use crate::config::{Config, Metric, READY_METRIC};
use crate::text::{self, Family};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub const READY_HELP: &str =
    "1 once the output's snapshot is loaded and its changes are applied; 0 otherwise.";
/// Envelopes buffered per output while its snapshot loads; beyond this the
/// buffer is dropped and the snapshot is read again.
pub const MAX_BUFFERED: usize = 4096;

/// Output-scoped element identity: namespace and element ID.
pub type RowKey = (String, String);
pub type Row = Map<String, Value>;

/// One row change, decoded from a graph change.
#[derive(Clone, Debug, PartialEq)]
pub enum RowChange {
    /// Add or full replacement: the complete row.
    Upsert(RowKey, Row),
    /// Set some properties and remove others.
    Patch {
        key: RowKey,
        set: Row,
        removed: Vec<String>,
    },
    Delete(RowKey),
}

#[derive(Debug)]
enum Status {
    Unattached,
    Loading {
        generation: u64,
        buffer: Vec<(u64, Vec<RowChange>)>,
        overflowed: bool,
    },
    Ready,
}

#[derive(Debug)]
struct OutputState {
    status: Status,
    rows: BTreeMap<RowKey, Row>,
}

/// Outcome of applying live changes.
#[derive(Debug, PartialEq, Eq)]
pub enum Applied {
    Applied,
    Buffered,
    /// The output was not ready and its buffer overflowed, or a patch named an
    /// unknown row: the output is now Loading and its snapshot must be re-read.
    ReloadNeeded,
    /// Unattached or unconfigured alias: nothing to do.
    Ignored,
}

/// Outcome of completing a snapshot load.
#[derive(Debug, PartialEq, Eq)]
pub enum Loaded {
    Ready,
    /// The load was superseded or its buffer overflowed: read it again.
    Retry,
}

/// The rows of every configured output and the metrics they produce.
#[derive(Debug)]
pub struct Exporter {
    metrics: BTreeMap<String, Vec<Metric>>,
    outputs: BTreeMap<String, OutputState>,
    next_generation: u64,
}

impl Exporter {
    /// `attached` names the aliases that have an incoming edge; the others
    /// stay Unattached. Attached outputs start Loading.
    pub fn new<'a>(config: &Config, attached: impl IntoIterator<Item = &'a str>) -> Self {
        let mut exporter = Self {
            metrics: config.metrics.clone(),
            outputs: config
                .outputs
                .keys()
                .map(|alias| {
                    (
                        alias.clone(),
                        OutputState {
                            status: Status::Unattached,
                            rows: BTreeMap::new(),
                        },
                    )
                })
                .collect(),
            next_generation: 0,
        };
        for alias in attached {
            exporter.begin_load(alias);
        }
        exporter
    }

    /// Forget the output's rows and start (again) waiting for its snapshot.
    /// Returns the load generation the snapshot result must present, or
    /// `None` for an unconfigured alias.
    pub fn begin_load(&mut self, alias: &str) -> Option<u64> {
        self.next_generation += 1;
        let generation = self.next_generation;
        let output = self.outputs.get_mut(alias)?;
        output.rows.clear();
        output.status = Status::Loading {
            generation,
            buffer: Vec::new(),
            overflowed: false,
        };
        Some(generation)
    }

    /// The current load generation of a Loading output.
    pub fn loading(&self, alias: &str) -> Option<u64> {
        match self.outputs.get(alias)?.status {
            Status::Loading { generation, .. } => Some(generation),
            _ => None,
        }
    }

    /// Install a snapshot read at producer `sequence`, then replay buffered
    /// changes with a later sequence.
    pub fn complete_load(
        &mut self,
        alias: &str,
        generation: u64,
        sequence: u64,
        rows: impl IntoIterator<Item = (RowKey, Row)>,
    ) -> Loaded {
        let Some(output) = self.outputs.get_mut(alias) else {
            return Loaded::Retry;
        };
        let buffer = match &mut output.status {
            Status::Loading {
                generation: current,
                buffer,
                overflowed,
            } if *current == generation => {
                if *overflowed {
                    *overflowed = false;
                    return Loaded::Retry;
                }
                std::mem::take(buffer)
            }
            _ => return Loaded::Retry,
        };
        output.rows = rows.into_iter().collect();
        for (_, changes) in buffer.into_iter().filter(|(seq, _)| *seq > sequence) {
            if !apply_all(&mut output.rows, changes) {
                self.begin_load(alias);
                return Loaded::Retry;
            }
        }
        output.status = Status::Ready;
        Loaded::Ready
    }

    /// Apply (Ready) or buffer (Loading) one envelope's changes.
    pub fn apply(&mut self, alias: &str, sequence: u64, changes: Vec<RowChange>) -> Applied {
        let Some(output) = self.outputs.get_mut(alias) else {
            return Applied::Ignored;
        };
        match &mut output.status {
            Status::Unattached => Applied::Ignored,
            Status::Loading {
                buffer, overflowed, ..
            } => {
                if *overflowed {
                    return Applied::Buffered;
                }
                if buffer.len() >= MAX_BUFFERED {
                    buffer.clear();
                    *overflowed = true;
                    return Applied::ReloadNeeded;
                }
                buffer.push((sequence, changes));
                Applied::Buffered
            }
            Status::Ready => {
                if apply_all(&mut output.rows, changes) {
                    Applied::Applied
                } else {
                    self.begin_load(alias);
                    Applied::ReloadNeeded
                }
            }
        }
    }

    /// The output's input failed: export nothing for it until it reloads.
    pub fn fail(&mut self, alias: &str) -> Option<u64> {
        self.begin_load(alias)
    }

    pub fn is_ready(&self, alias: &str) -> bool {
        self.outputs
            .get(alias)
            .is_some_and(|output| matches!(output.status, Status::Ready))
    }

    pub fn rows(&self, alias: &str) -> Option<&BTreeMap<RowKey, Row>> {
        self.outputs.get(alias).map(|output| &output.rows)
    }

    /// Every metric family with at least one series (sorted by name), then
    /// the readiness family for every configured output.
    pub fn families(&self) -> Vec<Family> {
        let mut families: BTreeMap<&str, Family> = BTreeMap::new();
        for (alias, metrics) in &self.metrics {
            let Some(output) = self.outputs.get(alias) else {
                continue;
            };
            if !matches!(output.status, Status::Ready) {
                continue;
            }
            for metric in metrics {
                for row in output.rows.values() {
                    let Some(value) = row.get(&metric.value).and_then(number) else {
                        continue;
                    };
                    let mut labels: BTreeMap<String, String> = metric.const_labels.clone();
                    for (label, property) in &metric.labels {
                        labels.insert(
                            label.clone(),
                            row.get(property).map(label_text).unwrap_or_default(),
                        );
                    }
                    families
                        .entry(&metric.metric)
                        .or_insert_with(|| Family {
                            name: metric.metric.clone(),
                            help: metric.help.clone(),
                            series: BTreeMap::new(),
                        })
                        .series
                        .insert(labels.into_iter().collect(), value);
                }
            }
        }
        let mut families: Vec<Family> = families.into_values().collect();
        families.push(Family {
            name: READY_METRIC.into(),
            help: READY_HELP.into(),
            series: self
                .outputs
                .iter()
                .map(|(alias, output)| {
                    (
                        vec![("output".to_string(), alias.clone())],
                        if matches!(output.status, Status::Ready) {
                            1.0
                        } else {
                            0.0
                        },
                    )
                })
                .collect(),
        });
        families
    }

    pub fn render(&self) -> String {
        text::render(&self.families())
    }
}

/// Apply changes in order; `false` when a patch names an unknown row (the
/// rows no longer match the producer).
fn apply_all(rows: &mut BTreeMap<RowKey, Row>, changes: Vec<RowChange>) -> bool {
    for change in changes {
        match change {
            RowChange::Upsert(key, row) => {
                rows.insert(key, row);
            }
            RowChange::Patch { key, set, removed } => {
                let Some(row) = rows.get_mut(&key) else {
                    return false;
                };
                for name in removed {
                    row.remove(&name);
                }
                row.extend(set);
            }
            RowChange::Delete(key) => {
                rows.remove(&key);
            }
        }
    }
    true
}

/// A number, or a boolean as 1/0. Anything else (null, missing, text) gives
/// no series.
fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::Bool(flag) => Some(if *flag { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// A label value: text as is, numbers and booleans in JSON form, null as "".
fn label_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config(value: Value) -> Config {
        Config::parse(&value).unwrap()
    }

    fn example() -> Config {
        config(serde_json::from_str(include_str!("../examples/ignite.json")).unwrap())
    }

    fn key(id: &str) -> RowKey {
        ("q".into(), id.into())
    }

    fn row(value: Value) -> Row {
        value.as_object().unwrap().clone()
    }

    fn upsert(id: &str, value: Value) -> RowChange {
        RowChange::Upsert(key(id), row(value))
    }

    fn small() -> Config {
        config(json!({
            "listen": "127.0.0.1:0",
            "outputs": {"overdue": {"component": "tile-overdue"}, "action": {"component": "tile-action"}},
            "metrics": {
                "overdue": [{"metric": "ignite_drasi_overdue_requests", "help": "Overdue.", "value": "value", "labels": {"workload": "workload"}}],
                "action": [{"metric": "ignite_drasi_action_needed", "help": "Action.", "value": "value"}]
            }
        }))
    }

    /// Golden rendering of the full Ignite configuration with every tile loaded.
    #[test]
    fn golden_ignite_text_format() {
        let config = example();
        let mut exporter = Exporter::new(&config, config.outputs.keys().map(String::as_str));
        let mut load = |alias: &str, rows: Vec<(&str, Value)>| {
            let generation = exporter.loading(alias).unwrap();
            let rows = rows.into_iter().map(|(id, value)| (key(id), row(value)));
            assert_eq!(
                exporter.complete_load(alias, generation, 0, rows),
                Loaded::Ready
            );
        };
        load(
            "overdue",
            vec![("a", json!({"workload": "support", "value": 3}))],
        );
        load(
            "deployments",
            vec![
                (
                    "a",
                    json!({"deployment": "EU-A", "status": 2, "capacity_tpm": 100000}),
                ),
                (
                    "b",
                    json!({"deployment": "EU-B", "status": 0, "capacity_tpm": 100000}),
                ),
                (
                    "c",
                    json!({"deployment": "US-C", "status": -1, "capacity_tpm": 100000}),
                ),
            ],
        );
        load(
            "capacity",
            vec![("a", json!({"total_tpm": 300000, "healthy_tpm": 200000}))],
        );
        load(
            "workloads",
            vec![
                (
                    "a",
                    json!({"workload": "support", "name": "Support", "deployment": "EU-A", "status": 2}),
                ),
                (
                    "b",
                    json!({"workload": "batch", "name": "Summaries", "deployment": "EU-B", "status": 0}),
                ),
            ],
        );
        load(
            "change",
            vec![("a", json!({"value": 3, "text": "Move summaries to US-C and support to EU-B"}))],
        );
        load("demand", vec![("a", json!({"needed_tpm": 140000}))]);
        load("recovery", vec![("a", json!({"value": 0}))]);
        load("action", vec![("a", json!({"value": 1}))]);
        load(
            "resilience",
            vec![("a", json!({"value": 0, "lost": "EU-B"}))],
        );
        load(
            "running",
            vec![
                (
                    "a",
                    json!({"workload": "support", "deployment": "EU-A", "value": 60000}),
                ),
                (
                    "b",
                    json!({"workload": "batch", "deployment": "EU-B", "value": 80000}),
                ),
            ],
        );
        load(
            "proposed",
            vec![
                (
                    "a",
                    json!({"workload": "support", "deployment": "EU-B", "demand_tpm": 60000}),
                ),
                (
                    "b",
                    json!({"workload": "batch", "deployment": "US-C", "demand_tpm": 80000}),
                ),
            ],
        );
        load(
            "policy",
            vec![
                (
                    "a",
                    json!({"workload": "support", "deployment": "EU-A", "value": 1}),
                ),
                (
                    "b",
                    json!({"workload": "support", "deployment": "US-C", "value": 0}),
                ),
            ],
        );
        // last-approval stays Loading: no series, ready 0.
        assert_eq!(
            exporter.render(),
            r#"# HELP ignite_drasi_capacity_tpm Capacity in tokens per minute: total, on healthy deployments, and needed.
# TYPE ignite_drasi_capacity_tpm gauge
ignite_drasi_capacity_tpm{kind="healthy"} 200000
ignite_drasi_capacity_tpm{kind="needed"} 140000
ignite_drasi_capacity_tpm{kind="total"} 300000
# HELP ignite_drasi_change_status Latest change: 0 none in progress, 1 awaiting approval, 2 approved and applying, 3 applied and verifying, 4 verified, 5 rejected, -1 unknown; text names the approved change.
# TYPE ignite_drasi_change_status gauge
ignite_drasi_change_status{text="Move summaries to US-C and support to EU-B"} 3
# HELP ignite_drasi_deployment_capacity_tpm Configured capacity of each deployment, tokens per minute.
# TYPE ignite_drasi_deployment_capacity_tpm gauge
ignite_drasi_deployment_capacity_tpm{deployment="EU-A"} 100000
ignite_drasi_deployment_capacity_tpm{deployment="EU-B"} 100000
ignite_drasi_deployment_capacity_tpm{deployment="US-C"} 100000
# HELP ignite_drasi_deployment_status Latest health report: 0 healthy, 1 throttled, 2 failed, -1 unknown.
# TYPE ignite_drasi_deployment_status gauge
ignite_drasi_deployment_status{deployment="EU-A"} 2
ignite_drasi_deployment_status{deployment="EU-B"} 0
ignite_drasi_deployment_status{deployment="US-C"} -1
# HELP ignite_drasi_failover_readiness Failover readiness, described by the text label: 1 survives losing any one deployment, 0 at risk (lost and workload name the first loss that leaves no plan), 2 already degraded, -1 unknown.
# TYPE ignite_drasi_failover_readiness gauge
ignite_drasi_failover_readiness{lost="EU-B",text="",workload=""} 0
# HELP ignite_drasi_overdue_requests Requests past their deadline and still unanswered, per workload.
# TYPE ignite_drasi_overdue_requests gauge
ignite_drasi_overdue_requests{workload="support"} 3
# HELP ignite_drasi_policy_allowed Placement policy: 1 allowed, 0 not allowed, -1 unknown.
# TYPE ignite_drasi_policy_allowed gauge
ignite_drasi_policy_allowed{deployment="EU-A",workload="support"} 1
ignite_drasi_policy_allowed{deployment="US-C",workload="support"} 0
# HELP ignite_drasi_proposed_tpm Demand of each workload on the deployment Drasi's feasible plan assigns.
# TYPE ignite_drasi_proposed_tpm gauge
ignite_drasi_proposed_tpm{deployment="EU-B",workload="support"} 60000
ignite_drasi_proposed_tpm{deployment="US-C",workload="batch"} 80000
# HELP ignite_drasi_recommended_action Recommended action, named by the text label: 0 none, 1 approve the proposed change, 2 no change fits policy and capacity, 3 add capacity (at risk), -1 unknown.
# TYPE ignite_drasi_recommended_action gauge
ignite_drasi_recommended_action{text=""} 1
# HELP ignite_drasi_recovery_state Recovery after an applied plan: 0 awaiting evidence, 1 verified, 2 nothing to verify, -1 unknown.
# TYPE ignite_drasi_recovery_state gauge
ignite_drasi_recovery_state 0
# HELP ignite_drasi_running_tpm Demand of each workload on its configured deployment; negative when policy no longer allows it.
# TYPE ignite_drasi_running_tpm gauge
ignite_drasi_running_tpm{deployment="EU-A",workload="support"} 60000
ignite_drasi_running_tpm{deployment="EU-B",workload="batch"} 80000
# HELP ignite_drasi_workload_status Each workload where it runs now: 0 healthy, 1 over quota, 2 failing (its deployment failed), 3 not allowed there by policy, -1 unknown.
# TYPE ignite_drasi_workload_status gauge
ignite_drasi_workload_status{deployment="EU-A",name="Support",workload="support"} 2
ignite_drasi_workload_status{deployment="EU-B",name="Summaries",workload="batch"} 0
# HELP drasi_reaction_input_ready 1 once the output's snapshot is loaded and its changes are applied; 0 otherwise.
# TYPE drasi_reaction_input_ready gauge
drasi_reaction_input_ready{output="action"} 1
drasi_reaction_input_ready{output="capacity"} 1
drasi_reaction_input_ready{output="change"} 1
drasi_reaction_input_ready{output="demand"} 1
drasi_reaction_input_ready{output="deployments"} 1
drasi_reaction_input_ready{output="last-approval"} 0
drasi_reaction_input_ready{output="overdue"} 1
drasi_reaction_input_ready{output="policy"} 1
drasi_reaction_input_ready{output="proposed"} 1
drasi_reaction_input_ready{output="recovery"} 1
drasi_reaction_input_ready{output="resilience"} 1
drasi_reaction_input_ready{output="running"} 1
drasi_reaction_input_ready{output="workloads"} 1
"#
        );
    }

    #[test]
    fn unattached_and_loading_outputs_export_only_readiness() {
        let config = small();
        let mut exporter = Exporter::new(&config, ["overdue"]);
        // Live changes before the snapshot are buffered, not exported.
        assert_eq!(
            exporter.apply(
                "overdue",
                1,
                vec![upsert("a", json!({"workload": "support", "value": 1}))]
            ),
            Applied::Buffered
        );
        assert_eq!(
            exporter.apply("action", 1, vec![upsert("a", json!({"value": 1}))]),
            Applied::Ignored
        );
        assert_eq!(exporter.apply("other", 1, vec![]), Applied::Ignored);
        assert_eq!(
            exporter.render(),
            "# HELP drasi_reaction_input_ready 1 once the output's snapshot is loaded and its changes are applied; 0 otherwise.\n\
             # TYPE drasi_reaction_input_ready gauge\n\
             drasi_reaction_input_ready{output=\"action\"} 0\n\
             drasi_reaction_input_ready{output=\"overdue\"} 0\n"
        );
    }

    #[test]
    fn insert_update_patch_and_delete() {
        let config = small();
        let mut exporter = Exporter::new(&config, ["overdue", "action"]);
        for alias in ["overdue", "action"] {
            let generation = exporter.loading(alias).unwrap();
            assert_eq!(
                exporter.complete_load(alias, generation, 0, []),
                Loaded::Ready
            );
        }
        let series = |exporter: &Exporter| {
            exporter
                .render()
                .lines()
                .filter(|line| !line.starts_with('#') && !line.starts_with(READY_METRIC))
                .map(String::from)
                .collect::<Vec<_>>()
        };
        assert!(series(&exporter).is_empty());
        exporter.apply(
            "overdue",
            1,
            vec![
                upsert("s", json!({"workload": "support", "value": 1})),
                upsert("b", json!({"workload": "batch", "value": 0})),
            ],
        );
        exporter.apply("action", 1, vec![upsert("x", json!({"value": true}))]);
        assert_eq!(
            series(&exporter),
            [
                "ignite_drasi_action_needed 1",
                "ignite_drasi_overdue_requests{workload=\"batch\"} 0",
                "ignite_drasi_overdue_requests{workload=\"support\"} 1",
            ]
        );
        // Replace, patch (set and remove), delete.
        exporter.apply(
            "overdue",
            2,
            vec![upsert("s", json!({"workload": "support", "value": 4}))],
        );
        exporter.apply(
            "overdue",
            3,
            vec![RowChange::Patch {
                key: key("b"),
                set: row(json!({"value": 2})),
                removed: vec![],
            }],
        );
        exporter.apply(
            "action",
            2,
            vec![RowChange::Patch {
                key: key("x"),
                set: Map::new(),
                removed: vec!["value".into()],
            }],
        );
        assert_eq!(
            series(&exporter),
            [
                "ignite_drasi_overdue_requests{workload=\"batch\"} 2",
                "ignite_drasi_overdue_requests{workload=\"support\"} 4",
            ]
        );
        exporter.apply(
            "overdue",
            4,
            vec![RowChange::Delete(key("s")), RowChange::Delete(key("gone"))],
        );
        exporter.apply("action", 3, vec![upsert("x", json!({"value": -1.5}))]);
        assert_eq!(
            series(&exporter),
            [
                "ignite_drasi_action_needed -1.5",
                "ignite_drasi_overdue_requests{workload=\"batch\"} 2"
            ]
        );
        // A patch of an unknown row means the rows no longer match: reload.
        assert_eq!(
            exporter.apply(
                "overdue",
                5,
                vec![RowChange::Patch {
                    key: key("nope"),
                    set: Map::new(),
                    removed: vec![]
                }]
            ),
            Applied::ReloadNeeded
        );
        assert!(!exporter.is_ready("overdue"));
        assert_eq!(series(&exporter), ["ignite_drasi_action_needed -1.5"]);
    }

    #[test]
    fn null_missing_and_text_values_produce_no_series() {
        let config = small();
        let mut exporter = Exporter::new(&config, ["overdue"]);
        let generation = exporter.loading("overdue").unwrap();
        exporter.complete_load(
            "overdue",
            generation,
            0,
            [
                (key("a"), row(json!({"workload": "a", "value": null}))),
                (key("b"), row(json!({"workload": "b"}))),
                (key("c"), row(json!({"workload": "c", "value": "3"}))),
                (key("d"), row(json!({"value": 1}))),
                (key("e"), row(json!({"workload": 7, "value": false}))),
            ],
        );
        let text = exporter.render();
        assert!(
            text.contains("ignite_drasi_overdue_requests{workload=\"\"} 1\n"),
            "{text}"
        );
        assert!(
            text.contains("ignite_drasi_overdue_requests{workload=\"7\"} 0\n"),
            "{text}"
        );
        assert!(
            !text.contains("workload=\"a\"")
                && !text.contains("workload=\"b\"")
                && !text.contains("workload=\"c\"")
        );
    }

    #[test]
    fn label_values_are_escaped() {
        let config = small();
        let mut exporter = Exporter::new(&config, ["overdue"]);
        let generation = exporter.loading("overdue").unwrap();
        exporter.complete_load(
            "overdue",
            generation,
            0,
            [(key("a"), row(json!({"workload": "a\"b\\c\nd", "value": 1})))],
        );
        assert!(exporter
            .render()
            .contains("ignite_drasi_overdue_requests{workload=\"a\\\"b\\\\c\\nd\"} 1\n"));
    }

    #[test]
    fn snapshot_then_buffered_changes_and_restart() {
        let config = small();
        let mut exporter = Exporter::new(&config, ["overdue"]);
        let generation = exporter.loading("overdue").unwrap();
        // Arrive while the snapshot is read: 5 is already in it, 6 and 7 are not.
        exporter.apply(
            "overdue",
            5,
            vec![upsert("s", json!({"workload": "support", "value": 99}))],
        );
        exporter.apply(
            "overdue",
            6,
            vec![upsert("b", json!({"workload": "batch", "value": 1}))],
        );
        exporter.apply(
            "overdue",
            7,
            vec![RowChange::Patch {
                key: key("s"),
                set: row(json!({"value": 3})),
                removed: vec![],
            }],
        );
        let snapshot = [(key("s"), row(json!({"workload": "support", "value": 2})))];
        assert_eq!(
            exporter.complete_load("overdue", generation, 5, snapshot.clone()),
            Loaded::Ready
        );
        let text = exporter.render();
        assert!(
            text.contains("ignite_drasi_overdue_requests{workload=\"support\"} 3\n"),
            "{text}"
        );
        assert!(
            text.contains("ignite_drasi_overdue_requests{workload=\"batch\"} 1\n"),
            "{text}"
        );
        assert!(text.contains("drasi_reaction_input_ready{output=\"overdue\"} 1\n"));
        // A stale result for a superseded load is discarded.
        assert_eq!(
            exporter.complete_load("overdue", generation, 5, []),
            Loaded::Retry
        );
        assert!(exporter.is_ready("overdue"));

        // Restart: a new exporter (the sink is volatile) rebuilds from the snapshot alone.
        let mut restarted = Exporter::new(&config, ["overdue"]);
        assert!(!restarted.render().contains("ignite_drasi_overdue_requests"));
        let generation = restarted.loading("overdue").unwrap();
        restarted.complete_load(
            "overdue",
            generation,
            9,
            [(key("s"), row(json!({"workload": "support", "value": 3})))],
        );
        let text = restarted.render();
        assert!(
            text.contains("{workload=\"support\"} 3\n") && !text.contains("batch"),
            "{text}"
        );

        // An input failure drops the output to not ready until it reloads.
        let generation = restarted.fail("overdue").unwrap();
        assert!(!restarted.render().contains("ignite_drasi_overdue_requests"));
        assert!(restarted
            .render()
            .contains("drasi_reaction_input_ready{output=\"overdue\"} 0\n"));
        assert_eq!(
            restarted.complete_load("overdue", generation, 10, []),
            Loaded::Ready
        );
    }

    #[test]
    fn buffer_overflow_forces_a_new_snapshot() {
        let config = small();
        let mut exporter = Exporter::new(&config, ["overdue"]);
        let generation = exporter.loading("overdue").unwrap();
        for sequence in 0..MAX_BUFFERED as u64 {
            assert_eq!(
                exporter.apply("overdue", sequence, vec![]),
                Applied::Buffered
            );
        }
        assert_eq!(
            exporter.apply("overdue", 1_000_000, vec![]),
            Applied::ReloadNeeded
        );
        assert_eq!(
            exporter.apply("overdue", 1_000_001, vec![]),
            Applied::Buffered
        );
        assert_eq!(
            exporter.complete_load("overdue", generation, 0, []),
            Loaded::Retry
        );
        // The retry (same generation, buffer reset) succeeds.
        assert_eq!(
            exporter.complete_load("overdue", generation, 2_000_000, []),
            Loaded::Ready
        );
    }

    #[test]
    fn duplicate_series_keep_the_last_row_by_identity() {
        let config = small();
        let mut exporter = Exporter::new(&config, ["overdue"]);
        let generation = exporter.loading("overdue").unwrap();
        exporter.complete_load(
            "overdue",
            generation,
            0,
            [
                (key("a"), row(json!({"workload": "support", "value": 1}))),
                (key("b"), row(json!({"workload": "support", "value": 2}))),
            ],
        );
        let text = exporter.render();
        assert_eq!(text.matches("ignite_drasi_overdue_requests{").count(), 1);
        assert!(text.contains("{workload=\"support\"} 2\n"));
    }
}
