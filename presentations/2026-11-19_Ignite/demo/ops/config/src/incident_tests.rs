//! The incident card queries (contracts.md section 15) run through drasi-core with a
//! data-driven clock: each card appears once per event, after its debounce, within 60 s
//! of the event, never for a state found after a restart, and never across a reset.
use std::{collections::BTreeMap, sync::Arc};

use drasi_core::{
    evaluation::{context::QueryPartEvaluationContext as Row, functions::FunctionRegistry},
    models::{Element, ElementMetadata, ElementPropertyMap, ElementReference, QueryJoin, QueryJoinKey, SourceChange},
    query::{ContinuousQuery, QueryBuilder},
};
use drasi_functions_cypher::CypherFunctionSet;
use drasi_query_cypher::CypherParser;
use serde_json::{json, Value};

use super::{queries, Health, HEALTH, HEALTH_LABEL, JOINS, TABLES};

/// `name` as the generator configures it for `health`: its text and its join labels.
async fn build_in(name: &str, health: Health) -> ContinuousQuery {
    let text = queries(health).into_iter().find(|q| q.name == name).expect("query").text.into_owned();
    let registry = Arc::new(FunctionRegistry::new()).with_cypher_function_set();
    let joins = JOINS
        .iter()
        .map(|(id, (l1, p1), (l2, p2))| QueryJoin {
            id: id.to_string(),
            keys: vec![
                QueryJoinKey { label: health.label(l1).to_string(), property: p1.to_string() },
                QueryJoinKey { label: health.label(l2).to_string(), property: p2.to_string() },
            ],
        })
        .collect();
    QueryBuilder::new(text, Arc::new(CypherParser::new(registry.clone())))
        .with_function_registry(registry)
        .with_joins(joins)
        .try_build()
        .await
        .expect("query builds")
}

/// One query fed with element changes. Rows it adds are cards; `upstream` feeds a labelled
/// helper query's rows in, as a query-to-query edge does (changes only, no bootstrap).
struct Sim {
    name: &'static str,
    health: Health,
    query: ContinuousQuery,
    now: u64,
    store: BTreeMap<(String, String), Value>,
    added: Vec<(u64, Value)>,
    upstream: Option<(Box<Sim>, &'static str)>,
    forwarded: Vec<(u64, u64, Option<Value>)>,
}

impl Sim {
    async fn new(name: &'static str) -> Self {
        Sim::new_in(name, Health::Postgres).await
    }
    async fn new_in(name: &'static str, health: Health) -> Self {
        let query = build_in(name, health).await;
        Sim { name, health, query, now: 0, store: BTreeMap::new(), added: vec![], upstream: None, forwarded: vec![] }
    }
    /// `name` reading deployment health from telemetry, fed by `deployment-health`.
    async fn from_telemetry(name: &'static str) -> Self {
        let mut sim = Sim::new_in(name, Health::Otel).await;
        sim.upstream = Some((Box::new(Sim::new_in(HEALTH, Health::Otel).await), HEALTH_LABEL));
        sim
    }
    async fn with_upstream(name: &'static str, upstream: &'static str, label: &'static str) -> Self {
        let mut sim = Sim::new(name).await;
        sim.upstream = Some((Box::new(Sim::new(upstream).await), label));
        sim
    }
    fn record(&mut self, results: Vec<Row>) {
        for row in results {
            let vars = |v: &drasi_core::evaluation::context::QueryVariables| {
                Value::Object(v.iter().map(|(k, v)| (k.to_string(), Value::from(v.clone()))).collect())
            };
            match row {
                Row::Adding { after, row_signature } => {
                    let after = vars(&after);
                    self.added.push((self.now, after.clone()));
                    self.forwarded.push((self.now, row_signature, Some(after)));
                }
                Row::Updating { after, row_signature, .. } | Row::Aggregation { after, row_signature, .. } => {
                    self.forwarded.push((self.now, row_signature, Some(vars(&after))))
                }
                Row::Removing { row_signature, .. } => self.forwarded.push((self.now, row_signature, None)),
                Row::Noop => {}
            }
        }
    }
    async fn own_advance(&mut self, to: u64) {
        while let Some(due) = self.query.future_queue().peek_due_time().await.unwrap() {
            if due > to {
                break;
            }
            self.now = self.now.max(due);
            if let Some(result) = self.query.process_due_futures().await.unwrap() {
                self.record(result.results);
            }
        }
        self.now = self.now.max(to);
    }
    async fn change(&mut self, label: &str, id: &str, props: Option<Value>) {
        let key = (label.to_string(), id.to_string());
        let metadata = ElementMetadata {
            reference: ElementReference::new("pg", &format!("{label}:{id}")),
            labels: Arc::new([Arc::from(label)]),
            effective_from: self.now,
        };
        let change = match props {
            None => {
                if self.store.remove(&key).is_none() {
                    return;
                }
                SourceChange::Delete { metadata }
            }
            Some(props) => {
                let mut merged = self.store.get(&key).cloned().unwrap_or(json!({}));
                for (k, v) in props.as_object().unwrap() {
                    merged[k] = v.clone();
                }
                let existed = self.store.insert(key, merged.clone()).is_some();
                // `__from`/`__to` (label:id of the end nodes) make a relation.
                let element = match (merged["__from"].as_str(), merged["__to"].as_str()) {
                    (Some(from), Some(to)) => {
                        let mut properties = merged.clone();
                        properties.as_object_mut().unwrap().retain(|k, _| !k.starts_with("__"));
                        Element::Relation {
                            metadata,
                            in_node: ElementReference::new("pg", from),
                            out_node: ElementReference::new("pg", to),
                            properties: ElementPropertyMap::from(properties),
                        }
                    }
                    _ => Element::Node { metadata, properties: ElementPropertyMap::from(merged) },
                };
                if existed {
                    SourceChange::Update { element, removed_properties: Default::default() }
                } else {
                    SourceChange::Insert { element }
                }
            }
        };
        let results = self.query.process_source_change(change).await.unwrap();
        self.record(results);
    }
    async fn pump(&mut self) {
        if let Some((up, label)) = self.upstream.as_mut() {
            let label = *label;
            let rows = std::mem::take(&mut up.forwarded);
            for (t, signature, row) in rows {
                self.own_advance(t).await;
                self.change(label, &format!("{signature:x}"), row).await;
            }
        }
    }
    async fn at(&mut self, t: u64) {
        if let Some((up, _)) = self.upstream.as_mut() {
            up.own_advance(t).await;
        }
        self.pump().await;
        self.own_advance(t).await;
    }
    async fn put(&mut self, label: &str, id: &str, props: Value) {
        if let Some((up, _)) = self.upstream.as_mut() {
            up.now = self.now;
            up.change(label, id, Some(props.clone())).await;
        }
        self.pump().await;
        self.change(label, id, Some(props)).await;
    }
    /// A Server restart: every query starts again from the tables; model and helper
    /// results arrive again only as they change.
    /// The OTel source's state is volatile: after a restart its elements return only as
    /// the next probes arrive.
    async fn restart(&mut self) {
        let volatile = |label: &str| ["Service", "Metric", "REPORTS"].contains(&label);
        let tables: Vec<_> = self.store.iter().filter(|((l, _), _)| TABLES.contains(&l.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        let models: Vec<_> = self.store.iter().filter(|((l, _), _)| !TABLES.contains(&l.as_str()) && !volatile(l)).map(|(k, v)| (k.clone(), v.clone())).collect();
        let label = self.upstream.as_ref().map(|(_, l)| *l);
        self.query = build_in(self.name, self.health).await;
        self.store.clear();
        if let Some((up, _)) = self.upstream.as_mut() {
            up.query = build_in(up.name, up.health).await;
            let upstore = std::mem::take(&mut up.store);
            for ((l, id), v) in upstore.into_iter().filter(|((l, _), _)| !volatile(l)) {
                up.change(&l, &id, Some(v)).await;
            }
            up.forwarded.clear();
        }
        for ((l, id), v) in tables {
            self.change(&l, &id, Some(v)).await;
        }
        for ((l, id), v) in models.into_iter().filter(|((l, _), _)| Some(l.as_str()) != label) {
            self.change(&l, &id, Some(v)).await;
        }
    }
    /// The query's current rows, from the changes it emitted.
    fn rows(&self) -> Vec<Value> {
        let mut rows = BTreeMap::new();
        for (_, signature, row) in &self.forwarded {
            match row {
                Some(row) => rows.insert(*signature, row.clone()),
                None => rows.remove(signature),
            };
        }
        rows.into_values().collect()
    }
    fn cards(&self) -> Vec<(u64, String)> {
        self.added.iter().map(|(t, row)| (*t, row["id"].as_str().unwrap().to_string())).collect()
    }
}

/// The talk's world: two workloads, three deployments, health reports every second.
struct World {
    sim: Sim,
    run: String,
    status: BTreeMap<&'static str, &'static str>,
    sequence: u64,
}

impl World {
    async fn new(sim: Sim) -> Self {
        let mut w = World { sim, run: "run-1".into(), status: BTreeMap::new(), sequence: 0 };
        w.sim.now = 1000;
        w.put("routing_config", "singleton", json!({"singleton":true,"run_epoch":"run-1","revision":1})).await;
        for (id, class, region, demand) in [("support", "restricted", "EU", 60000), ("batch", "public", "any", 80000)] {
            w.put("ai_workloads", id, json!({"id":id,"run_epoch":"run-1","data_class":class,"region_requirement":region,
                "model":"support-model","demand_tpm":demand})).await;
        }
        for (id, region) in [("EU-A", "EU"), ("EU-B", "EU"), ("US-C", "US")] {
            w.status.insert(id, "ok");
            w.put("model_deployments", id, json!({"id":id,"run_epoch":"run-1","region":region,"model":"support-model","capacity_tpm":100000})).await;
        }
        w.route("support", "EU-A").await;
        w.route("batch", "EU-B").await;
        w.tick().await;
        w
    }
    async fn put(&mut self, label: &str, id: &str, props: Value) {
        self.sim.put(label, id, props).await;
    }
    async fn route(&mut self, workload: &str, deployment: &str) {
        let props = json!({"workload":workload,"run_epoch":self.run,"deployment":deployment,"applied_ms":self.sim.now});
        self.put("workload_routes", workload, props).await;
    }
    async fn tick(&mut self) {
        let statuses: Vec<_> = self.status.iter().map(|(k, v)| (*k, *v)).collect();
        for (id, status) in statuses {
            self.sequence += 1;
            let now = self.sim.now;
            if self.sim.health == Health::Otel {
                self.probe(id, status, now).await;
                continue;
            }
            let props = json!({"id":id,"run_epoch":self.run,"observed_ms":now,"expires_ms":now + 8000,"status":status,"sequence":self.sequence});
            self.put("deployment_health_reports", id, props).await;
        }
    }
    /// One health probe as Drasi's OTel source projects it (drasi-core PR 1030, DESIGN.md):
    /// the Service, its `deployment_health_probe_status` Metric per deployment and run, and REPORTS.
    async fn probe(&mut self, deployment: &str, status: &str, now: u64) {
        let service = "svc:ai-gateway";
        let metric = format!("metric:{service}:deployment_health_probe_status:deployment={deployment},run_epoch={}", self.run);
        let code = match status { "ok" => 0.0, "throttled" => 1.0, "failed" => 2.0, _ => 3.0 };
        let at = iso(now);
        self.put("Service", service, json!({"name":"ai-gateway","lastSeen":at})).await;
        self.put("Metric", &metric, json!({"name":"deployment_health_probe_status","unit":"","value":code,"observedAt":at,"receivedAt":at})).await;
        let reports = format!("reports:{metric}");
        if !self.sim.store.contains_key(&("REPORTS".to_string(), reports.clone())) {
            let ends = json!({"__from":format!("Service:{service}"),"__to":format!("Metric:{metric}")});
            self.put("REPORTS", &reports, ends).await;
        }
    }
    /// Runs the clock to `t`, with a health report from every deployment each second.
    async fn until(&mut self, t: u64) {
        while self.sim.now + 1000 <= t {
            let next = self.sim.now + 1000;
            self.sim.at(next).await;
            self.tick().await;
        }
        self.sim.at(t).await;
    }
    async fn plan(&mut self, state: &str, candidate: &str, moves: i64, assignments: &str, reason: &str) {
        let props = json!({"id":"plan","state":state,"candidate_id":candidate,"basis":format!("basis-{candidate}"),"revision":1,
            "assignments":assignments,"moves":moves,"peak_utilisation":0.8,"reason":reason,"run_epoch":self.run,"solved_ms":self.sim.now});
        self.put("PlanResult", "plan", props).await;
    }
    async fn resilience(&mut self, state: &str, first_failure: &str, unhealthy: i64, reason: &str) {
        let props = json!({"id":"resilience","state":state,"first_failure":first_failure,"baseline":"feasible",
            "unhealthy_bins":unhealthy,"reason":reason,"basis":"b","revision":1,"run_epoch":self.run});
        self.put("ResilienceResult", "resilience", props).await;
    }
    async fn facts(&mut self, problems: &str) {
        let run = self.run.clone();
        self.put("DeploymentFacts", "f", json!({"run_epoch":run,"problems":problems,"healthy":"EU-B 100k, US-C 100k"})).await;
        self.put("PolicyFacts", "g", json!({"run_epoch":run,"policy":"support: EU-A, EU-B (restricted data, EU only) · summaries: EU-A, EU-B, US-C",
            "needed":"support 60k, summaries 80k"})).await;
    }
    async fn reset(&mut self) {
        self.run = "run-2".into();
        self.put("routing_config", "singleton", json!({"run_epoch":"run-2","revision":1})).await;
    }
}

/// RFC 3339 for a test clock in milliseconds since the epoch (under a day).
fn iso(ms: u64) -> String {
    let s = ms / 1000;
    format!("1970-01-01T{:02}:{:02}:{:02}.{:03}+00:00", s / 3600, s / 60 % 60, s % 60, ms % 1000)
}

fn ids(cards: &[(u64, String)]) -> Vec<(u64, &str)> {
    cards.iter().map(|(t, id)| (*t, id.as_str())).collect()
}

#[tokio::test]
async fn failed_card_once_per_failure_after_3_s_and_never_after_a_restart() {
    let mut w = World::new(Sim::new("incident-failed").await).await;
    w.until(10500).await;
    w.status.insert("EU-A", "failed");
    w.tick().await;
    w.until(20000).await;
    w.status.insert("EU-A", "ok");
    w.tick().await;
    w.until(25000).await;
    // A 2-second blip posts nothing.
    w.status.insert("EU-A", "failed");
    w.tick().await;
    w.until(27000).await;
    w.status.insert("EU-A", "ok");
    w.tick().await;
    w.until(30000).await;
    w.route("support", "EU-B").await;
    w.route("batch", "US-C").await;
    w.status.insert("EU-B", "failed");
    w.tick().await;
    w.until(40000).await;
    w.sim.restart().await;
    w.until(50000).await;
    assert_eq!(ids(&w.sim.cards()), [(13500, "failed-EU-A-10000"), (33000, "failed-EU-B-30000")]);
    let card = &w.sim.added[0].1;
    assert_eq!(card["title"], "Deployment failed: EU-A");
    assert_eq!(card["text"], "EU-A has reported Failed for 3 seconds. Support runs there, so its requests are failing.");
    assert_eq!((card["region"].as_str(), card["capacity"].as_str(), card["runs"].as_str()), (Some("EU"), Some("100k tokens/min"), Some("support")));
}

#[tokio::test]
async fn overdue_card_once_per_episode_after_3_s() {
    let mut w = World::new(Sim::with_upstream("incident-overdue", "incident-overdue-count", "SupportOverdue").await).await;
    let task = |i: u64, now: u64| json!({"id":format!("t{i}"),"run_epoch":"run-1","workload":"support","created_ms":now,"deadline_ms":now + 15000});
    let mut pending = vec![];
    for i in 1..=12u64 {
        w.until(i * 5000).await;
        let now = w.sim.now;
        w.put("ai_requests", &format!("t{i}"), task(i, now)).await;
        // Support answers within a second, except between 10 s and 60 s.
        if !(2..=11).contains(&i) {
            w.put("ai_requests", &format!("t{i}"), json!({"completed_ms":now + 500})).await;
        } else {
            pending.push(i);
        }
        if i == 12 {
            for p in &pending {
                w.put("ai_requests", &format!("t{p}"), json!({"completed_ms":now})).await;
            }
        }
    }
    w.until(70000).await;
    assert_eq!(ids(&w.sim.cards()), [(28000, "overdue-25000")]);
    let card = &w.sim.added[0].1;
    assert_eq!(card["text"], "1 customer-support request is past its 15-second deadline. More follow until support can run somewhere.");
    assert_eq!(card["overdue"], "1");
}

const MOVE: &str = "Move batch to US-C and support to EU-B (2 moves, peak use 80%)";
const NONE: &str = "Everything already runs on an allowed, healthy destination with room (peak use 80%)";
const APPLIED: &str = r#"{"batch":"US-C","support":"EU-B"}"#;

#[tokio::test]
async fn proposals_out_of_date_cards_and_restarts() {
    for query in ["incident-proposal", "incident-superseded"] {
        let mut w = World::new(Sim::new(query).await).await;
        w.facts("").await;
        w.plan("feasible", "c0", 0, r#"{"batch":"EU-B","support":"EU-A"}"#, NONE).await;
        w.until(11000).await;
        w.facts("EU-A is down").await;
        w.plan("feasible", "c1", 2, APPLIED, MOVE).await;
        w.until(40000).await;
        // The talk's policy edit: no plan, then a new proposal once the policy is restored.
        w.plan("infeasible", "", 0, "{}", "No allowed, healthy destination for batch").await;
        w.until(60000).await;
        w.plan("feasible", "c3", 2, APPLIED, MOVE).await;
        w.until(70000).await;
        // Applied: the plan now has no moves and the same assignments. Not out of date.
        w.plan("feasible", "c4", 0, APPLIED, NONE).await;
        w.until(90000).await;
        // A proposal replaced by another soon after its card; and one replaced before its card,
        // which no Out-of-date card mentions.
        w.plan("feasible", "c8", 1, r#"{"batch":"EU-A","support":"EU-B"}"#, "Move batch to EU-A (1 move, peak use 80%)").await;
        w.until(96000).await;
        w.plan("feasible", "c9", 1, r#"{"batch":"EU-A","support":"EU-B"}"#, "Move batch to EU-A (1 move, peak use 70%)").await;
        w.until(110000).await;
        w.plan("feasible", "c10", 1, r#"{"batch":"EU-A","support":"EU-B"}"#, "Move batch to EU-A (1 move, peak use 60%)").await;
        w.until(113000).await;
        w.plan("feasible", "c11", 1, r#"{"batch":"EU-A","support":"EU-B"}"#, "Move batch to EU-A (1 move, peak use 50%)").await;
        w.until(125000).await;
        // A restart with c9 standing, and a reset, post nothing.
        w.sim.restart().await;
        w.until(140000).await;
        w.reset().await;
        w.facts("").await;
        w.plan("feasible", "r2", 0, r#"{"batch":"EU-B","support":"EU-A"}"#, NONE).await;
        w.until(200000).await;
        let cards = w.sim.cards();
        if query == "incident-proposal" {
            assert_eq!(ids(&cards), [(16000, "proposal-c1"), (65000, "proposal-c3"), (95000, "proposal-c8"), (101000, "proposal-c9"), (118000, "proposal-c11")]);
            let card = &w.sim.added[0].1;
            assert_eq!(card["title"], "Proposed change: Move summaries to US-C and support to EU-B");
            assert_eq!(card["why"], "EU-A is down");
            assert_eq!(card["peak"], "80%");
            assert_eq!(card["capacity"], "healthy: EU-B 100k, US-C 100k · needed: support 60k, summaries 80k");
            assert_eq!(card["based_on"], "revision 1 · plan c1");
            assert_eq!((card["candidate_id"].as_str(), card["basis"].as_str()), (Some("c1"), Some("basis-c1")));
        } else {
            assert_eq!(ids(&cards), [(42000, "superseded-c1"), (98000, "superseded-c8"), (112000, "superseded-c9")]);
            assert_eq!(w.sim.added[0].1["now"], "No valid plan");
            assert_eq!(
                w.sim.added[0].1["text"],
                "The proposed change “Move summaries to US-C and support to EU-B” no longer matches Drasi’s plan, so Drasi would reject an approval of it. No change fits policy and capacity now: No allowed, healthy destination for summaries."
            );
            assert_eq!(w.sim.added[1].1["text"].as_str().unwrap().ends_with("Drasi’s new proposal follows."), true);
        }
    }
}

#[tokio::test]
async fn outcomes_once_per_status_in_the_approved_cards_thread() {
    let mut w = World::new(Sim::new("incident-outcome").await).await;
    w.until(70000).await;
    let approval = json!({"id":"mm-post1","run_epoch":"run-1","candidate_id":"c3","basis":"b","revision":1,"status":"pending",
        "latest":true,"requested_ms":70000,"requested_by":"@oncall","post_id":"post1","change":"Move summaries to US-C and support to EU-B"});
    w.put("change_approvals", "mm-post1", approval).await;
    w.until(70100).await;
    w.route("support", "EU-B").await;
    w.route("batch", "US-C").await;
    w.put("change_approvals", "mm-post1", json!({"status":"applied","decided_ms":70100})).await;
    w.until(80000).await;
    w.put("change_approvals", "mm-post1", json!({"status":"verified","decided_ms":80000})).await;
    w.until(90000).await;
    let desk = json!({"id":"desk-1","run_epoch":"run-1","candidate_id":"c8","basis":"b","revision":1,"status":"pending","latest":true,"requested_ms":90000});
    w.put("change_approvals", "desk-1", desk).await;
    w.put("change_approvals", "desk-1", json!({"status":"rejected","decided_ms":90010,"reason":"The facts changed since this plan was proposed"})).await;
    // A person's Reject is shown on the card itself, not by Drasi.
    let declined = json!({"id":"mm-post3","run_epoch":"run-1","candidate_id":"c9","basis":"b","revision":1,"status":"declined",
        "latest":true,"requested_ms":95000,"decided_ms":95000,"post_id":"post3"});
    w.put("change_approvals", "mm-post3", declined).await;
    w.until(200000).await;
    w.sim.restart().await;
    w.until(210000).await;
    assert_eq!(ids(&w.sim.cards()), [(71100, "mm-post1-applied"), (81000, "mm-post1-verified"), (91010, "desk-1-rejected")]);
    let applied = &w.sim.added[0].1;
    assert_eq!((applied["root_id"].as_str(), applied["title"].as_str()), (Some("post1"), Some("Applied: Move summaries to US-C and support to EU-B")));
    assert_eq!((applied["detail_title"].as_str(), applied["detail"].as_str()), (Some("Routes"), Some("support → EU-B · summaries → US-C")));
    let rejected = &w.sim.added[2].1;
    assert_eq!((rejected["root_id"].as_str(), rejected["approved_by"].as_str()), (Some(""), Some("desk")));
    assert_eq!(rejected["text"], "The facts changed since this plan was proposed.");
}

#[tokio::test]
async fn failover_at_risk_and_restored() {
    for query in ["incident-at-risk", "incident-restored"] {
        let mut w = World::new(Sim::new(query).await).await;
        w.plan("feasible", "c0", 0, APPLIED, NONE).await;
        w.resilience("safe", "", 0, "Losing any one healthy destination still leaves a plan").await;
        w.until(10000).await;
        // Already degraded is not "at risk".
        w.resilience("at_risk", "EU-B", 1, "Losing EU-B leaves no plan: No allowed, healthy destination for support").await;
        w.until(20000).await;
        w.resilience("safe", "", 0, "Losing any one healthy destination still leaves a plan").await;
        w.until(30000).await;
        w.plan("feasible", "c1", 0, APPLIED, NONE).await;
        w.resilience("at_risk", "EU-B", 0, "Losing EU-B leaves no plan: support needs 60000, more than any allowed, healthy destination can take").await;
        w.until(60000).await;
        w.sim.restart().await;
        w.until(70000).await;
        w.plan("feasible", "c2", 0, APPLIED, NONE).await;
        w.resilience("safe", "", 0, "Losing any one healthy destination still leaves a plan").await;
        w.until(80000).await;
        w.reset().await;
        w.until(90000).await;
        if query == "incident-at-risk" {
            assert_eq!(ids(&w.sim.cards()), [(33000, "at-risk-30000")]);
            let card = &w.sim.added[0].1;
            assert_eq!(card["title"], "Failover at risk: losing EU-B leaves support nowhere to run");
            assert_eq!((card["lost"].as_str(), card["affects"].as_str()), (Some("EU-B"), Some("support")));
        } else {
            // Drasi saw the at-risk state after the restart, then saw it end; the reset posts nothing.
            assert_eq!(ids(&w.sim.cards()), [(73000, "restored-70000")]);
        }
    }
}

#[tokio::test]
async fn restored_after_at_risk() {
    let mut w = World::new(Sim::new("incident-restored").await).await;
    w.plan("feasible", "c0", 0, APPLIED, NONE).await;
    w.resilience("safe", "", 0, "ok").await;
    w.until(10000).await;
    w.resilience("at_risk", "EU-B", 0, "Losing EU-B leaves no plan: support needs 60000").await;
    w.until(30000).await;
    w.plan("feasible", "c1", 0, APPLIED, NONE).await;
    w.resilience("safe", "", 0, "ok").await;
    w.until(100000).await;
    assert_eq!(ids(&w.sim.cards()), [(33000, "restored-30000")]);
    assert_eq!(w.sim.added[0].1["title"], "Failover restored");
}

#[tokio::test]
async fn proposal_facts_are_sorted_and_complete() {
    let mut w = World::new(Sim::new("incident-deployments").await).await;
    w.until(5000).await;
    w.status.insert("EU-A", "failed");
    w.status.insert("US-C", "throttled");
    w.tick().await;
    w.until(8000).await;
    let last = &w.sim.forwarded.last().unwrap().2.clone().unwrap();
    assert_eq!((last["problems"].as_str(), last["healthy"].as_str()), (Some("EU-A is down, US-C is throttled"), Some("EU-B 100k")));

    let mut w = World::new(Sim::new("incident-policy").await).await;
    for workload in ["support", "batch"] {
        for target in ["EU-A", "EU-B", "US-C"] {
            let allowed = !(workload == "support" && target == "US-C");
            let id = format!("{workload}/{target}");
            let props = json!({"id":id,"subject":workload,"target":target,"state":if allowed {"allowed"} else {"denied"},"run_epoch":"run-1"});
            w.put("PolicyResult", &id, props).await;
        }
    }
    let last = &w.sim.forwarded.last().unwrap().2.clone().unwrap();
    assert_eq!(last["policy"], "support: EU-A, EU-B (restricted data, EU only) · summaries: EU-A, EU-B, US-C");
    assert_eq!(last["needed"], "support 60k, summaries 80k");
}

/// Health from telemetry: the same timeline as `failed_card_once_per_failure_after_3_s_and_never_after_a_restart`
/// with probes through Drasi's OTel source instead of `deployment_health_reports`, and the same cards.
#[tokio::test]
async fn failed_card_from_telemetry_is_the_failed_card_from_reports() {
    let mut w = World::new(Sim::from_telemetry("incident-failed").await).await;
    w.until(10500).await;
    w.status.insert("EU-A", "failed");
    w.tick().await;
    w.until(20000).await;
    w.status.insert("EU-A", "ok");
    w.tick().await;
    w.until(25000).await;
    w.status.insert("EU-A", "failed");
    w.tick().await;
    w.until(27000).await;
    w.status.insert("EU-A", "ok");
    w.tick().await;
    w.until(30000).await;
    w.route("support", "EU-B").await;
    w.route("batch", "US-C").await;
    w.status.insert("EU-B", "failed");
    w.tick().await;
    w.until(40000).await;
    w.sim.restart().await;
    w.until(50000).await;
    assert_eq!(ids(&w.sim.cards()), [(13500, "failed-EU-A-10000"), (33000, "failed-EU-B-30000")]);
    assert_eq!(w.sim.added[0].1["text"], "EU-A has reported Failed for 3 seconds. Support runs there, so its requests are failing.");
}

fn statuses(sim: &Sim, column: &str) -> Vec<(String, Value)> {
    let mut rows: Vec<_> = sim.rows().into_iter().map(|r| (r[column].as_str().unwrap().to_string(), r["status"].clone())).collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

/// Healthy, Failed, Unknown after 8 s of silence, recovery, and a reset, from probes alone.
#[tokio::test]
async fn deployment_tile_from_telemetry_detects_failure_silence_and_recovery() {
    let mut w = World::new(Sim::from_telemetry("tile-deployments").await).await;
    w.until(5000).await;
    assert_eq!(statuses(&w.sim, "deployment"), [("EU-A".into(), json!(0)), ("EU-B".into(), json!(0)), ("US-C".into(), json!(0))]);
    w.status.insert("EU-A", "failed");
    w.status.insert("US-C", "throttled");
    w.tick().await;
    w.until(7000).await;
    assert_eq!(statuses(&w.sim, "deployment"), [("EU-A".into(), json!(2)), ("EU-B".into(), json!(0)), ("US-C".into(), json!(1))]);
    // EU-B's reporting pauses: no probes, so Unknown (-1) once its last probe expires.
    w.status.remove("EU-B");
    w.until(14500).await;
    assert_eq!(statuses(&w.sim, "deployment")[1], ("EU-B".into(), json!(0)), "within 8 s of its last probe");
    w.until(16000).await;
    assert_eq!(statuses(&w.sim, "deployment")[1], ("EU-B".into(), json!(-1)));
    w.status.insert("EU-B", "ok");
    w.status.insert("EU-A", "ok");
    w.tick().await;
    w.until(18000).await;
    assert_eq!(statuses(&w.sim, "deployment")[..2], [("EU-A".into(), json!(0)), ("EU-B".into(), json!(0))]);
    // A new run: the last run's probes no longer count; the new run's do.
    w.reset().await;
    w.put("model_deployments", "EU-A", json!({"run_epoch":"run-2"})).await;
    w.until(20000).await;
    let rows = statuses(&w.sim, "deployment");
    assert_eq!(rows, [("EU-A".into(), json!(0))], "only EU-A belongs to run-2 here");
}

/// The planner's bins: health from telemetry (failed and throttled are unhealthy),
/// unknown after silence.
#[tokio::test]
async fn plan_bins_from_telemetry() {
    let mut w = World::new(Sim::from_telemetry("plan-bins").await).await;
    w.until(3000).await;
    w.status.insert("EU-A", "failed");
    w.status.insert("EU-B", "throttled");
    w.status.remove("US-C");
    w.tick().await;
    w.until(13000).await;
    let mut rows: Vec<_> = w.sim.rows().into_iter().map(|r| (r["id"].as_str().unwrap().to_string(), r["health"].clone())).collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        rows,
        [("EU-A".into(), json!("unhealthy")), ("EU-B".into(), json!("unhealthy")), ("US-C".into(), json!("unknown"))]
    );
}

/// A `drasi.standard/optimization` status row as the `plan` component publishes it
/// (drasi/optimization): EU-A failed, support and batch moved, losing EU-B leaves no plan.
fn plan_status(extra: Value) -> Value {
    let mut row = json!({
        "status": "optimal", "detail": "all objectives are optimal", "objective": 0.8,
        "objectives": {"moves": 2.0, "peak": 0.8}, "signature": "00aa", "solutionSignature": "11bb",
        "variables": 4, "constraints": 6, "members": 20, "setSizes": {"unhealthy": 1},
        "provenance": {"run_epoch": "run-1", "revision": 3}, "precondition": null,
        "summaries": {
            "assignments": [{"subject": "batch", "target": "US-C", "value": 1}, {"subject": "support", "target": "EU-B", "value": 1}],
            "moved": [{"subject": "batch", "target": "US-C", "value": 1}, {"subject": "support", "target": "EU-B", "value": 1}]},
        "scenarios": {"loss": {"values": 2, "feasible": 0, "failures": [
            {"value": "EU-B", "status": "infeasible", "detail": "constraint assign-once[support] has no eligible variables and cannot hold"},
            {"value": "US-C", "status": "infeasible", "detail": "no assignment satisfies every constraint"}]}},
        "scenariosSignature": "00aa"});
    for (k, v) in extra.as_object().unwrap() {
        row[k] = v.clone();
    }
    row
}

async fn plan_row(query: &'static str, status: Value) -> Value {
    let mut sim = Sim::new(query).await;
    sim.change("PlanStatus", "status", Some(status)).await;
    let rows = sim.rows();
    assert_eq!(rows.len(), 1, "{query}: {rows:?}");
    rows[0].clone()
}

/// The result queries read the official optimizer's status row with today's columns.
#[tokio::test]
async fn plan_and_resilience_results_from_the_optimization_status() {
    let p = plan_row("plan-results", plan_status(json!({}))).await;
    assert_eq!(p["state"], "feasible");
    // The plan for these facts: its solution, then its basis.
    assert_eq!(p["candidate_id"], "11bb00aa");
    assert_eq!(p["basis"], "00aa");
    assert_eq!(p["revision"], 3);
    assert_eq!(p["run_epoch"], "run-1");
    assert_eq!(p["moves"], 2);
    assert_eq!(p["assignments"], r#"{"batch":"US-C","support":"EU-B"}"#);
    assert_eq!(p["reason"], "Move batch to US-C and support to EU-B (2 moves, peak use 80%)");
    let r = plan_row("resilience-results", plan_status(json!({}))).await;
    assert_eq!((r["state"].clone(), r["first_failure"].clone()), (json!("at_risk"), json!("EU-B")));
    assert_eq!(r["unhealthy_bins"], 1);
    assert_eq!(r["reason"], "Losing EU-B leaves no plan: No allowed, healthy destination with room for support");

    let seed = json!({"objectives": {"moves": 0.0, "peak": 0.8}, "setSizes": {"unhealthy": 0},
        "summaries": {"assignments": [{"subject": "batch", "target": "EU-B", "value": 1}, {"subject": "support", "target": "EU-A", "value": 1}], "moved": []},
        "scenarios": {"loss": {"values": 3, "feasible": 3, "failures": []}}});
    let p = plan_row("plan-results", plan_status(seed.clone())).await;
    assert_eq!(p["moves"], 0);
    assert!(p["reason"].as_str().unwrap().starts_with("Everything already runs"));
    assert_eq!(plan_row("resilience-results", plan_status(seed)).await["state"], "safe");

    let infeasible = json!({"status": "infeasible", "objectives": {}, "objective": null,
        "detail": "constraint assign-once[support] has no eligible variables and cannot hold",
        "solutionSignature": null, "summaries": null});
    let p = plan_row("plan-results", plan_status(infeasible.clone())).await;
    assert_eq!((p["state"].clone(), p["candidate_id"].clone(), p["assignments"].clone()), (json!("infeasible"), json!(""), json!("{}")));
    assert_eq!(p["reason"], "No allowed, healthy destination with room for support");
    assert_eq!(plan_row("resilience-results", plan_status(infeasible)).await["state"], "at_risk");

    let unknown = json!({"status": "unknown", "precondition": "health-known", "objectives": {}, "objective": null,
        "detail": "precondition health-known does not hold: 1 member of bins matches, expected at most 0: [EU-B]",
        "solutionSignature": null, "summaries": null, "scenarios": null, "scenariosSignature": null});
    let p = plan_row("plan-results", plan_status(unknown.clone())).await;
    assert_eq!((p["state"].clone(), p["reason"].clone()), (json!("unknown"), json!("Health of EU-B is unknown")));
    assert_eq!(plan_row("resilience-results", plan_status(unknown)).await["state"], "unknown");
}
