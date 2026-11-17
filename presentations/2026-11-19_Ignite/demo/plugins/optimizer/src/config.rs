//! Configuration: plain Rust, no host types. See plan/contracts.md.
use anyhow::{ensure, Context, Result};
use serde::Deserialize;

pub const PLAN: &str = "ignite/assignment-plan";
pub const RESILIENCE: &str = "ignite/assignment-resilience";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Plan,
    Resilience,
}
impl Mode {
    pub fn implementation(self) -> &'static str {
        match self {
            Self::Plan => PLAN,
            Self::Resilience => RESILIENCE,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub items: Items,
    pub bins: Bins,
    pub eligibility: Eligibility,
    pub constraints: Vec<Constraint>,
    pub objective: Vec<Objective>,
    #[serde(default = "epoch")]
    pub epoch_property: String,
    /// Input changes within this window are solved together.
    #[serde(default = "coalesce")]
    pub coalesce_ms: u64,
    /// One solve; exceeding it yields state `unknown`.
    #[serde(default = "limit")]
    pub time_limit_ms: u64,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Items {
    pub key: String,
    /// Non-negative integer demand.
    pub demand: String,
    /// The bin the item is assigned to now (a move is any change from it).
    pub current: String,
    /// Scenario revision the inputs belong to; all items must agree. Copied to the Plan.
    pub revision: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bins {
    pub key: String,
    /// Non-negative integer capacity.
    pub capacity: String,
    pub health: Health,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Health {
    pub property: String,
    /// Values meaning usable. Any value in neither list makes the result `unknown`.
    pub healthy: Vec<String>,
    pub unhealthy: Vec<String>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Eligibility {
    pub item_key: String,
    pub bin_key: String,
    /// `allowed`, `denied` or `unknown`; only `allowed` with `allowed = true` permits.
    pub state: String,
    pub allowed: String,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Constraint {
    /// Every item is assigned to exactly one bin; nothing is shed.
    AssignEachItemOnce,
    /// Sum of demand in a bin does not exceed its capacity.
    BinCapacity,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Objective {
    /// Number of items whose bin differs from `current`.
    FewestMoves,
    /// Largest demand / capacity over healthy bins.
    LowestPeakUtilisation,
}
fn epoch() -> String {
    "run_epoch".into()
}
fn coalesce() -> u64 {
    100
}
fn limit() -> u64 {
    2000
}
impl Config {
    pub fn parse(value: &serde_json::Value) -> Result<Self> {
        let config: Self = serde_json::from_value(value.clone()).context("optimizer configuration")?;
        ensure!(
            config.constraints.contains(&Constraint::AssignEachItemOnce)
                && config.constraints.contains(&Constraint::BinCapacity),
            "constraints must include assignEachItemOnce and binCapacity"
        );
        ensure!(!config.objective.is_empty(), "objective must list at least one term (lexicographic order)");
        ensure!(!config.bins.health.healthy.is_empty(), "health.healthy must not be empty");
        ensure!(config.coalesce_ms <= 5_000, "coalesceMs must be at most 5000");
        ensure!((1..=60_000).contains(&config.time_limit_ms), "timeLimitMs must be 1..=60000");
        Ok(config)
    }
}
