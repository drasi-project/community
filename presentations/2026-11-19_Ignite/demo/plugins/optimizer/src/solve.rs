//! The assignment problem: plain Rust over `good_lp` (microlp), no host types.
//!
//! Every item is assigned to exactly one healthy bin it is allowed on, and no
//! bin holds more demand than its capacity. The objective is lexicographic in
//! the configured order (fewest moves, then lowest peak utilisation): each term
//! is minimised with the earlier terms held at their optimum. A final pass
//! breaks remaining ties by sorted item and bin keys, so equal inputs always
//! give the same plan. Results are re-checked in exact integer arithmetic; a
//! solver answer that fails the check, an error or the time limit gives
//! `Unknown`, never a guess and never `Infeasible`.
use crate::config::Objective;
use good_lp::{
    constraint, microlp, variable, Expression, ProblemVariables, ResolutionError, Solution,
    SolutionStatus, SolverModel, Variable, WithTimeLimit,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Instant};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub key: String,
    pub demand: i64,
    /// The bin the item uses now; `None` when it has none.
    pub current: Option<String>,
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
/// Items and bins are sorted by key; `eligibility` is item -> bin -> eligible.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub items: Vec<Item>,
    pub bins: Vec<Bin>,
    pub eligibility: BTreeMap<String, BTreeMap<String, Eligible>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Feasible,
    Infeasible,
    Unknown,
}
/// Result of one solve. `assignments` is empty unless `Feasible`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    pub state: State,
    /// item key -> bin key.
    pub assignments: BTreeMap<String, String>,
    pub moves: u32,
    /// Largest assigned demand / capacity over the bins used (0–1).
    pub peak_utilisation: f64,
    pub reason: String,
}
impl Outcome {
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self::empty(State::Unknown, reason)
    }
    fn infeasible(reason: impl Into<String>) -> Self {
        Self::empty(State::Infeasible, reason)
    }
    fn empty(state: State, reason: impl Into<String>) -> Self {
        Self {
            state,
            assignments: BTreeMap::new(),
            moves: 0,
            peak_utilisation: 0.0,
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Safety {
    /// Every single loss of a healthy bin still leaves a feasible plan.
    Safe,
    /// Some single loss, or the baseline, leaves no plan.
    AtRisk,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Loss {
    pub lost: String,
    pub outcome: Outcome,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resilience {
    pub state: Safety,
    /// Lowest-sorted healthy bin whose loss leaves no plan; empty if none.
    pub first_failure: String,
    pub baseline: State,
    pub unhealthy_bins: usize,
    /// One per healthy bin, sorted; empty when the inputs are unknown.
    pub losses: Vec<Loss>,
    pub reason: String,
}

impl Problem {
    /// Why this problem cannot be answered, if it cannot.
    pub fn unknown_reason(&self) -> Option<String> {
        if let Some(bin) = self.bins.iter().find(|b| b.health == Health::Unknown) {
            return Some(format!("Health of {} is unknown", bin.key));
        }
        for item in &self.items {
            for bin in &self.bins {
                let eligible = self
                    .eligibility
                    .get(&item.key)
                    .and_then(|bins| bins.get(&bin.key))
                    .copied()
                    .unwrap_or(Eligible::Unknown);
                if eligible == Eligible::Unknown {
                    return Some(format!(
                        "Policy for {} on {} is unknown",
                        item.key, bin.key
                    ));
                }
            }
        }
        if let Some(item) = self.items.iter().find(|i| i.demand < 0) {
            return Some(format!("Demand of {} is negative", item.key));
        }
        if let Some(bin) = self.bins.iter().find(|b| b.capacity < 0) {
            return Some(format!("Capacity of {} is negative", bin.key));
        }
        None
    }
    fn allowed(&self, item: &str, bin: &str) -> bool {
        self.eligibility
            .get(item)
            .and_then(|bins| bins.get(bin))
            .is_some_and(|e| *e == Eligible::Allowed)
    }
}

/// The best plan with every healthy bin.
pub fn plan(problem: &Problem, objective: &[Objective], deadline: Instant) -> Outcome {
    plan_without(problem, None, objective, deadline)
}

/// The best plan when `lost` (if any) is unavailable as well as every unhealthy bin.
pub fn plan_without(
    problem: &Problem,
    lost: Option<&str>,
    objective: &[Objective],
    deadline: Instant,
) -> Outcome {
    if let Some(reason) = problem.unknown_reason() {
        return Outcome::unknown(reason);
    }
    let mut items: Vec<&Item> = problem.items.iter().collect();
    items.sort_by(|a, b| a.key.cmp(&b.key));
    let mut bins: Vec<&Bin> = problem
        .bins
        .iter()
        .filter(|b| b.health == Health::Healthy && Some(b.key.as_str()) != lost)
        .collect();
    bins.sort_by(|a, b| a.key.cmp(&b.key));
    let mut edges = Vec::new();
    for (i, item) in items.iter().enumerate() {
        for (b, bin) in bins.iter().enumerate() {
            if problem.allowed(&item.key, &bin.key) && item.demand <= bin.capacity {
                edges.push(Edge { item: i, bin: b });
            }
        }
    }
    let model = Model {
        items: &items,
        bins: &bins,
        edges: &edges,
    };
    if let Some(reason) = model.obviously_infeasible(problem) {
        return Outcome::infeasible(reason);
    }
    let mut fixed: Vec<(Term, f64)> = Vec::new();
    let mut chosen = None;
    let terms = objective
        .iter()
        .map(|o| match o {
            Objective::FewestMoves => Term::Moves,
            Objective::LowestPeakUtilisation => Term::Peak,
        })
        .chain(std::iter::once(Term::Order));
    for (phase, term) in terms.enumerate() {
        match model.solve(term, &fixed, deadline) {
            Phase::Solved(x) => {
                fixed.push((term, model.value(term, &x)));
                chosen = Some(x);
            }
            Phase::Infeasible if phase == 0 => return Outcome::infeasible(model.diagnose()),
            Phase::Infeasible => {
                return Outcome::unknown(
                    "The solver rejected its own optimum in a later objective phase",
                )
            }
            Phase::Unknown(reason) => return Outcome::unknown(reason),
        }
    }
    let Some(x) = chosen else {
        return Outcome::unknown("The solver did not run");
    };
    model.outcome(&x)
}

/// Remove each healthy bin in turn (sorted) and solve again.
pub fn resilience(problem: &Problem, objective: &[Objective], deadline: Instant) -> Resilience {
    let unhealthy_bins = problem
        .bins
        .iter()
        .filter(|b| b.health == Health::Unhealthy)
        .count();
    let unknown = |reason: String| Resilience {
        state: Safety::Unknown,
        first_failure: String::new(),
        baseline: State::Unknown,
        unhealthy_bins,
        losses: Vec::new(),
        reason,
    };
    if let Some(reason) = problem.unknown_reason() {
        return unknown(reason);
    }
    let baseline = plan(problem, objective, deadline);
    if baseline.state == State::Unknown {
        return unknown(baseline.reason);
    }
    let mut healthy: Vec<&Bin> = problem
        .bins
        .iter()
        .filter(|b| b.health == Health::Healthy)
        .collect();
    healthy.sort_by(|a, b| a.key.cmp(&b.key));
    let losses: Vec<Loss> = healthy
        .iter()
        .map(|bin| Loss {
            lost: bin.key.clone(),
            outcome: if baseline.state == State::Infeasible {
                Outcome::infeasible(format!(
                    "No plan even before losing {}: {}",
                    bin.key, baseline.reason
                ))
            } else {
                plan_without(problem, Some(&bin.key), objective, deadline)
            },
        })
        .collect();
    let failure = losses.iter().find(|l| l.outcome.state != State::Feasible);
    let (state, first_failure, reason) = match failure {
        Some(loss) if loss.outcome.state == State::Unknown => {
            let reason = format!(
                "Losing {} could not be checked: {}",
                loss.lost, loss.outcome.reason
            );
            return Resilience {
                losses,
                ..unknown(reason)
            };
        }
        Some(loss) => (
            Safety::AtRisk,
            loss.lost.clone(),
            if baseline.state == State::Infeasible {
                format!("No plan even now: {}", baseline.reason)
            } else {
                format!("Losing {} leaves no plan: {}", loss.lost, loss.outcome.reason)
            },
        ),
        None if baseline.state == State::Infeasible => (
            Safety::AtRisk,
            String::new(),
            format!("No plan even now: {}", baseline.reason),
        ),
        None => (
            Safety::Safe,
            String::new(),
            "Losing any one healthy destination still leaves a plan".into(),
        ),
    };
    Resilience {
        state,
        first_failure,
        baseline: baseline.state,
        unhealthy_bins,
        losses,
        reason,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Term {
    Moves,
    Peak,
    /// Prefer lower-sorted bins for lower-sorted items (deterministic ties).
    Order,
}
struct Edge {
    item: usize,
    bin: usize,
}
enum Phase {
    Solved(Vec<bool>),
    Infeasible,
    Unknown(String),
}
struct Model<'a> {
    items: &'a [&'a Item],
    bins: &'a [&'a Bin],
    edges: &'a [Edge],
}
impl Model<'_> {
    fn moved(&self, edge: &Edge) -> bool {
        self.items[edge.item].current.as_deref() != Some(self.bins[edge.bin].key.as_str())
    }
    fn order_weights(&self) -> Vec<f64> {
        let bins = self.bins.len().max(1) as f64;
        let items = self.items.len() as i32;
        // Strictly lexicographic while the weights stay exact in f64; otherwise a
        // cheaper deterministic preference with the same direction.
        let exact = bins.powi(items) <= 1e9;
        self.edges
            .iter()
            .map(|e| {
                let rank = e.bin as f64;
                if exact {
                    rank * bins.powi(items - 1 - e.item as i32)
                } else {
                    rank * (items as f64 - e.item as f64)
                }
            })
            .collect()
    }
    fn value(&self, term: Term, x: &[bool]) -> f64 {
        match term {
            Term::Moves => self
                .edges
                .iter()
                .zip(x)
                .filter(|(e, on)| **on && self.moved(e))
                .count() as f64,
            Term::Peak => self.peak(x),
            Term::Order => self
                .order_weights()
                .iter()
                .zip(x)
                .filter(|(_, on)| **on)
                .map(|(w, _)| w)
                .sum(),
        }
    }
    fn loads(&self, x: &[bool]) -> Vec<i64> {
        let mut loads = vec![0i64; self.bins.len()];
        for (edge, on) in self.edges.iter().zip(x) {
            if *on {
                loads[edge.bin] += self.items[edge.item].demand;
            }
        }
        loads
    }
    fn peak(&self, x: &[bool]) -> f64 {
        self.loads(x)
            .iter()
            .zip(self.bins)
            .filter(|(load, bin)| **load > 0 && bin.capacity > 0)
            .map(|(load, bin)| *load as f64 / bin.capacity as f64)
            .fold(0.0, f64::max)
    }
    fn solve(&self, minimise: Term, fixed: &[(Term, f64)], deadline: Instant) -> Phase {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Phase::Unknown("The solver reached its time limit".into());
        }
        let mut vars = ProblemVariables::new();
        let x: Vec<Variable> = self
            .edges
            .iter()
            .map(|_| vars.add(variable().binary()))
            .collect();
        let peak = vars.add(variable().min(0.0).max(1.0));
        let weights = self.order_weights();
        let expression = |term: Term| -> Expression {
            match term {
                Term::Moves => self
                    .edges
                    .iter()
                    .zip(&x)
                    .filter(|(e, _)| self.moved(e))
                    .map(|(_, v)| *v)
                    .sum(),
                Term::Peak => Expression::from(peak),
                Term::Order => x.iter().zip(&weights).map(|(v, w)| *v * *w).sum(),
            }
        };
        let mut model = vars
            .minimise(expression(minimise))
            .using(microlp)
            .with_time_limit(remaining.as_secs_f64());
        for i in 0..self.items.len() {
            let assigned: Expression = self
                .edges
                .iter()
                .zip(&x)
                .filter(|(e, _)| e.item == i)
                .map(|(_, v)| *v)
                .sum();
            model.add_constraint(constraint!(assigned == 1));
        }
        for (b, bin) in self.bins.iter().enumerate() {
            if bin.capacity == 0 {
                continue; // only zero-demand items have an edge to it
            }
            // Utilisation (demand / capacity) keeps coefficients near 1.
            let use_of: Expression = self
                .edges
                .iter()
                .zip(&x)
                .filter(|(e, _)| e.bin == b)
                .map(|(e, v)| *v * (self.items[e.item].demand as f64 / bin.capacity as f64))
                .sum();
            model.add_constraint(constraint!(use_of.clone() <= 1.0));
            model.add_constraint(constraint!(use_of <= peak));
        }
        for (term, optimum) in fixed {
            let tolerance = match term {
                Term::Moves => 0.5,
                Term::Peak => 1e-7,
                Term::Order => 0.5,
            };
            model.add_constraint(constraint!(expression(*term) <= *optimum + tolerance));
        }
        match model.solve() {
            Ok(solution) if matches!(solution.status(), SolutionStatus::Optimal) => {
                Phase::Solved(x.iter().map(|v| solution.value(*v) > 0.5).collect())
            }
            Ok(_) => Phase::Unknown("The solver reached its time limit".into()),
            Err(ResolutionError::Infeasible) => Phase::Infeasible,
            Err(error) => Phase::Unknown(format!("The solver failed: {error}")),
        }
    }
    /// Exact integer check of the solver's answer, then the outcome.
    fn outcome(&self, x: &[bool]) -> Outcome {
        let mut assignments = BTreeMap::new();
        for (edge, on) in self.edges.iter().zip(x) {
            if *on {
                let item = &self.items[edge.item].key;
                if assignments
                    .insert(item.clone(), self.bins[edge.bin].key.clone())
                    .is_some()
                {
                    return Outcome::unknown(format!("The solver assigned {item} twice"));
                }
            }
        }
        if let Some(item) = self.items.iter().find(|i| !assignments.contains_key(&i.key)) {
            return Outcome::unknown(format!("The solver left {} unassigned", item.key));
        }
        for (load, bin) in self.loads(x).iter().zip(self.bins) {
            if *load > bin.capacity {
                return Outcome::unknown(format!("The solver overfilled {}", bin.key));
            }
        }
        let moved: Vec<String> = self
            .items
            .iter()
            .filter(|i| i.current.as_ref() != assignments.get(&i.key))
            .map(|i| format!("{} to {}", i.key, assignments[&i.key]))
            .collect();
        let peak = (self.peak(x) * 1e6).round() / 1e6;
        let percent = (peak * 100.0).round();
        let reason = if moved.is_empty() {
            format!(
                "Everything already runs on an allowed, healthy destination with room (peak use {percent}%)"
            )
        } else {
            format!(
                "Move {} ({} move{}, peak use {percent}%)",
                join(&moved),
                moved.len(),
                if moved.len() == 1 { "" } else { "s" }
            )
        };
        Outcome {
            state: State::Feasible,
            moves: moved.len() as u32,
            assignments,
            peak_utilisation: peak,
            reason,
        }
    }
    /// Items with nowhere to go make the problem infeasible without a solve.
    fn obviously_infeasible(&self, problem: &Problem) -> Option<String> {
        let stranded: Vec<&Item> = self
            .items
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.edges.iter().any(|e| e.item == *i))
            .map(|(_, item)| *item)
            .collect();
        let first = stranded.first()?;
        let allowed: Vec<&&Bin> = self
            .bins
            .iter()
            .filter(|b| problem.allowed(&first.key, &b.key))
            .collect();
        Some(if allowed.is_empty() {
            format!("No allowed, healthy destination for {}", first.key)
        } else {
            format!(
                "{} needs {}, more than any allowed, healthy destination can take",
                first.key, first.demand
            )
        })
    }
    fn diagnose(&self) -> String {
        let needed: i64 = self.items.iter().map(|i| i.demand).sum();
        let reachable: i64 = self
            .bins
            .iter()
            .enumerate()
            .filter(|(b, _)| self.edges.iter().any(|e| e.bin == *b))
            .map(|(_, bin)| bin.capacity)
            .sum();
        format!(
            "Allowed, healthy destinations cannot take everything together (needed {needed}, available {reachable})"
        )
    }
}

fn join(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const OBJECTIVE: [Objective; 2] = [Objective::FewestMoves, Objective::LowestPeakUtilisation];

    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(2)
    }

    /// Today's seed: support (restricted, EU) 60k on EU-A; batch 80k on EU-B;
    /// three deployments of 100k; support may use EU-A and EU-B, batch all three.
    fn seed() -> Problem {
        let item = |key: &str, demand, current: &str| Item {
            key: key.into(),
            demand,
            current: Some(current.into()),
        };
        let bin = |key: &str| Bin {
            key: key.into(),
            capacity: 100_000,
            health: Health::Healthy,
        };
        let mut eligibility = BTreeMap::new();
        for (item, allowed) in [
            ("support", ["EU-A", "EU-B"].as_slice()),
            ("batch", ["EU-A", "EU-B", "US-C"].as_slice()),
        ] {
            eligibility.insert(
                item.to_string(),
                ["EU-A", "EU-B", "US-C"]
                    .into_iter()
                    .map(|b| {
                        let e = if allowed.contains(&b) {
                            Eligible::Allowed
                        } else {
                            Eligible::Denied
                        };
                        (b.to_string(), e)
                    })
                    .collect(),
            );
        }
        Problem {
            items: vec![item("batch", 80_000, "EU-B"), item("support", 60_000, "EU-A")],
            bins: vec![bin("EU-A"), bin("EU-B"), bin("US-C")],
            eligibility,
        }
    }
    fn set(problem: &mut Problem, bin: &str, f: impl FnOnce(&mut Bin)) {
        f(problem.bins.iter_mut().find(|b| b.key == bin).unwrap())
    }
    fn assignments(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn good_lp_microlp_is_available() {
        let mut vars = ProblemVariables::new();
        let a = vars.add(variable().binary());
        let b = vars.add(variable().binary());
        let solution = vars
            .minimise(a + 2 * b)
            .using(microlp)
            .with(constraint!(a + b == 1))
            .solve()
            .unwrap();
        assert_eq!(solution.value(a).round(), 1.0);
    }

    #[test]
    fn seed_all_healthy_keeps_everything_and_is_safe() {
        let p = seed();
        let plan = plan(&p, &OBJECTIVE, deadline());
        assert_eq!(plan.state, State::Feasible);
        assert_eq!(plan.moves, 0);
        assert_eq!(plan.assignments, assignments(&[("batch", "EU-B"), ("support", "EU-A")]));
        assert!((plan.peak_utilisation - 0.8).abs() < 1e-9);
        let r = resilience(&p, &OBJECTIVE, deadline());
        assert_eq!(r.state, Safety::Safe);
        assert_eq!(r.first_failure, "");
        assert_eq!(r.losses.len(), 3);
    }

    #[test]
    fn eu_a_failed_moves_support_to_eu_b_and_summaries_to_us_c() {
        let mut p = seed();
        set(&mut p, "EU-A", |b| b.health = Health::Unhealthy);
        let plan = plan(&p, &OBJECTIVE, deadline());
        assert_eq!(plan.state, State::Feasible, "{}", plan.reason);
        assert_eq!(plan.assignments, assignments(&[("batch", "US-C"), ("support", "EU-B")]));
        assert_eq!(plan.moves, 2);
        let r = resilience(&p, &OBJECTIVE, deadline());
        assert_eq!(r.state, Safety::AtRisk);
        assert_eq!(r.first_failure, "EU-B");
        assert_eq!(r.unhealthy_bins, 1);
        assert_eq!(r.losses.iter().map(|l| l.lost.as_str()).collect::<Vec<_>>(), ["EU-B", "US-C"]);
    }

    #[test]
    fn policy_edit_after_failure_is_infeasible() {
        let mut p = seed();
        set(&mut p, "EU-A", |b| b.health = Health::Unhealthy);
        p.eligibility.get_mut("support").unwrap().insert("EU-B".into(), Eligible::Denied);
        let plan = plan(&p, &OBJECTIVE, deadline());
        assert_eq!(plan.state, State::Infeasible);
        assert!(plan.assignments.is_empty());
        assert_eq!(plan.reason, "No allowed, healthy destination for support");
        assert_eq!(resilience(&p, &OBJECTIVE, deadline()).state, Safety::AtRisk);
    }

    #[test]
    fn capacity_beat_not_if_eu_b_fails_with_eu_a_at_50k_then_safe_at_100k() {
        // After the approved recovery: support on EU-B, summaries on US-C; EU-A returns at 50k.
        let mut p = seed();
        p.items = vec![
            Item { key: "batch".into(), demand: 80_000, current: Some("US-C".into()) },
            Item { key: "support".into(), demand: 60_000, current: Some("EU-B".into()) },
        ];
        set(&mut p, "EU-A", |b| b.capacity = 50_000);
        let plan_now = plan(&p, &OBJECTIVE, deadline());
        assert_eq!(plan_now.state, State::Feasible);
        assert_eq!(plan_now.moves, 0);
        let r = resilience(&p, &OBJECTIVE, deadline());
        assert_eq!(r.state, Safety::AtRisk);
        assert_eq!(r.first_failure, "EU-B");
        assert_eq!(r.baseline, State::Feasible);
        set(&mut p, "EU-A", |b| b.capacity = 100_000);
        let r = resilience(&p, &OBJECTIVE, deadline());
        assert_eq!(r.state, Safety::Safe, "{}", r.reason);
        assert_eq!(r.first_failure, "");
    }

    #[test]
    fn contract_table_rows() {
        // Healthy, batch on US-C, EU-B capacity 50 000: feasible, first failure EU-A.
        let mut p = seed();
        p.items[0].current = Some("US-C".into());
        set(&mut p, "EU-B", |b| b.capacity = 50_000);
        assert_eq!(plan(&p, &OBJECTIVE, deadline()).state, State::Feasible);
        let r = resilience(&p, &OBJECTIVE, deadline());
        assert_eq!((r.state, r.first_failure.as_str()), (Safety::AtRisk, "EU-A"));
        // US-C failed: feasible, first failure EU-A.
        let mut p = seed();
        set(&mut p, "US-C", |b| b.health = Health::Unhealthy);
        assert_eq!(plan(&p, &OBJECTIVE, deadline()).state, State::Feasible);
        let r = resilience(&p, &OBJECTIVE, deadline());
        assert_eq!((r.state, r.first_failure.as_str()), (Safety::AtRisk, "EU-A"));
        // A report expired: unknown everywhere.
        let mut p = seed();
        set(&mut p, "EU-B", |b| b.health = Health::Unknown);
        let plan = plan(&p, &OBJECTIVE, deadline());
        assert_eq!(plan.state, State::Unknown);
        assert_eq!(plan.reason, "Health of EU-B is unknown");
        let r = resilience(&p, &OBJECTIVE, deadline());
        assert_eq!(r.state, Safety::Unknown);
        assert!(r.losses.is_empty());
    }

    #[test]
    fn missing_or_unknown_eligibility_is_unknown_never_a_guess() {
        let mut p = seed();
        p.eligibility.get_mut("batch").unwrap().remove("US-C");
        assert_eq!(plan(&p, &OBJECTIVE, deadline()).state, State::Unknown);
        let mut p = seed();
        p.eligibility.get_mut("batch").unwrap().insert("US-C".into(), Eligible::Unknown);
        assert_eq!(plan(&p, &OBJECTIVE, deadline()).reason, "Policy for batch on US-C is unknown");
    }

    #[test]
    fn exhausted_time_limit_is_unknown_not_infeasible() {
        let mut p = seed();
        set(&mut p, "EU-A", |b| b.health = Health::Unhealthy);
        let outcome = plan(&p, &OBJECTIVE, Instant::now());
        assert_eq!(outcome.state, State::Unknown);
        assert_eq!(outcome.reason, "The solver reached its time limit");
    }

    #[test]
    fn peak_breaks_move_ties_and_order_breaks_the_rest() {
        // Nothing has a current bin, so every plan moves both items: peak decides.
        let mut p = seed();
        for item in &mut p.items {
            item.current = None;
        }
        let both = plan(&p, &OBJECTIVE, deadline());
        assert_eq!(both.moves, 2);
        assert!((both.peak_utilisation - 0.8).abs() < 1e-9);
        assert_ne!(both.assignments["batch"], both.assignments["support"]);
        // Same inputs, same plan; ties go to the lowest-sorted bins in item order.
        assert_eq!(both, plan(&p, &OBJECTIVE, deadline()));
        assert_eq!(both.assignments, assignments(&[("batch", "EU-A"), ("support", "EU-B")]));
        // Peak first: spread even if it costs a move.
        let mut p = seed();
        p.items[0].current = Some("EU-A".into());
        let moves_first = plan(&p, &OBJECTIVE, deadline());
        let peak_first = plan(&p, &[Objective::LowestPeakUtilisation, Objective::FewestMoves], deadline());
        assert!(moves_first.state == State::Feasible && peak_first.state == State::Feasible);
        assert!(peak_first.peak_utilisation <= moves_first.peak_utilisation);
    }

    #[test]
    fn capacity_is_exact_at_the_boundary() {
        let mut p = seed();
        set(&mut p, "EU-A", |b| b.health = Health::Unhealthy);
        set(&mut p, "US-C", |b| b.health = Health::Unhealthy);
        set(&mut p, "EU-B", |b| b.capacity = 140_000);
        let fits = plan(&p, &OBJECTIVE, deadline());
        assert_eq!(fits.state, State::Feasible);
        assert_eq!(fits.peak_utilisation, 1.0);
        set(&mut p, "EU-B", |b| b.capacity = 139_999);
        let full = plan(&p, &OBJECTIVE, deadline());
        assert_eq!(full.state, State::Infeasible);
        assert!(full.reason.starts_with("Allowed, healthy destinations cannot take everything"));
    }
}
