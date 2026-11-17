//! The assignment problem: plain Rust over `good_lp` (microlp), no host types.
//! The component (lib.rs) builds a [`Problem`] from its input rows and calls
//! [`plan`] or [`resilience`] on its bounded worker. Workstream 3 implements
//! both; the types here are the contract (plan/contracts.md, Optimizer plugin).
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub key: String,
    pub demand: i64,
    pub current: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Health {
    Healthy,
    Unhealthy,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bin {
    pub key: String,
    pub capacity: i64,
    pub health: Health,
}
/// Eligibility of (item, bin); a missing pair is `Unknown`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Eligible {
    Allowed,
    Denied,
    Unknown,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub items: Vec<Item>,
    pub bins: Vec<Bin>,
    pub eligibility: BTreeMap<(String, String), Eligible>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Feasible,
    Infeasible,
    Unknown,
}
/// Result of one solve. `assignments` is empty unless `Feasible`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub state: State,
    /// item key -> bin key.
    pub assignments: BTreeMap<String, String>,
    pub moves: u32,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use good_lp::{constraint, default_solver, variable, ProblemVariables, Solution, SolverModel};

    /// The backend the contract names is available offline and solves a tiny assignment.
    #[test]
    fn good_lp_microlp_is_available() {
        let mut vars = ProblemVariables::new();
        let a = vars.add(variable().binary());
        let b = vars.add(variable().binary());
        let solution = vars
            .minimise(a + 2 * b)
            .using(default_solver)
            .with(constraint!(a + b == 1))
            .solve()
            .unwrap();
        assert_eq!(solution.value(a).round(), 1.0);
    }
}
