//! Configuration: plain Rust, no host types. See plan/contracts.md.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, net::SocketAddr};

pub const READY_METRIC: &str = "drasi_reaction_input_ready";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    /// `host:port` to serve on (from IGNITE_DRASI_METRICS_PORT).
    pub listen: SocketAddr,
    #[serde(default = "path")]
    pub path: String,
    /// Output alias -> the producing component and port, as in Drasi's HTTP and SSE sinks.
    pub outputs: BTreeMap<String, Output>,
    /// Output alias -> the metrics each of its rows produces.
    pub metrics: BTreeMap<String, Vec<Metric>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Output {
    pub component: String,
    #[serde(default = "out")]
    pub port: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Metric {
    /// Prometheus metric name (gauge).
    pub metric: String,
    pub help: String,
    /// Row property holding the value: a number, or a boolean (1/0). Null or
    /// missing omits the series for that row.
    pub value: String,
    /// Label name -> row property. Null or missing renders "".
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub const_labels: BTreeMap<String, String>,
}
fn path() -> String {
    "/metrics".into()
}
fn out() -> String {
    "out".into()
}
fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == ':')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}
impl Config {
    pub fn parse(value: &serde_json::Value) -> Result<Self> {
        let config: Self =
            serde_json::from_value(value.clone()).context("prometheus-exporter configuration")?;
        ensure!(config.path.starts_with('/'), "path must start with /");
        let mut endpoints = BTreeMap::new();
        for (alias, output) in &config.outputs {
            ensure!(
                !alias.is_empty() && !output.component.is_empty() && !output.port.is_empty(),
                "outputs.{alias} is incomplete"
            );
            if let Some(other) = endpoints.insert((&output.component, &output.port), alias) {
                ensure!(false, "outputs {other} and {alias} name the same output");
            }
        }
        let mut help = BTreeMap::new();
        for (alias, metrics) in &config.metrics {
            ensure!(
                config.outputs.contains_key(alias),
                "metrics.{alias} names no output"
            );
            ensure!(!metrics.is_empty(), "metrics.{alias} is empty");
            for metric in metrics {
                ensure!(
                    valid_name(&metric.metric) && metric.metric != READY_METRIC,
                    "invalid metric name {}",
                    metric.metric
                );
                for label in metric.labels.keys().chain(metric.const_labels.keys()) {
                    ensure!(
                        valid_name(label) && !label.starts_with("__") && !label.contains(':'),
                        "invalid label {label}"
                    );
                }
                for label in metric.labels.keys() {
                    ensure!(
                        !metric.const_labels.contains_key(label),
                        "{} names label {label} twice",
                        metric.metric
                    );
                }
                // One HELP per metric name across outputs.
                if let Some(previous) = help.insert(metric.metric.clone(), metric.help.clone()) {
                    ensure!(
                        previous == metric.help,
                        "{} has two different help texts",
                        metric.metric
                    );
                }
            }
        }
        Ok(config)
    }
}
