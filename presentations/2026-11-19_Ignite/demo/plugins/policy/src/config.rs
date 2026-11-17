//! Configuration: plain Rust, no host types. See plan/contracts.md.
use anyhow::{ensure, Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    /// Rego source (one module). Changing it is a configuration update.
    pub module: String,
    /// Rule evaluated per pair; must yield `{"allowed": bool, "reason": string}`.
    pub rule: String,
    pub subjects: Keyed,
    pub targets: Keyed,
    pub parameters: Parameters,
    /// Copied from the subject row onto each decision.
    #[serde(default = "epoch")]
    pub epoch_property: String,
    /// Bound on one pair's evaluation; exceeding it yields `unknown`.
    #[serde(default = "timeout")]
    pub timeout_ms: u64,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Keyed {
    pub key: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameters {
    pub subject_key: String,
    pub target_key: String,
}
fn epoch() -> String {
    "run_epoch".into()
}
fn timeout() -> u64 {
    250
}
impl Config {
    pub fn parse(value: &serde_json::Value) -> Result<Self> {
        let config: Self = serde_json::from_value(value.clone()).context("rego-policy configuration")?;
        ensure!(!config.module.trim().is_empty(), "module must not be empty");
        ensure!(config.rule.starts_with("data."), "rule must be a data.* path");
        ensure!((1..=10_000).contains(&config.timeout_ms), "timeoutMs must be 1..=10000");
        Ok(config)
    }
}
