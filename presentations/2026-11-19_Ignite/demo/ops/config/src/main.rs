//! Generates this copy's Drasi Server configuration (plan/contracts.md section 4):
//! PostgreSQL source → policy-input → Drasi's Rego policy → policy-results → Drasi's
//! optimization (plan, with its single-loss what-if) → result queries → tile queries → Prometheus reaction, and the
//! approval, recovery, annotation and score queries → HTTP reactions; the incident
//! event queries → the `incident-channel` reaction posting cards to Mattermost.
//! With IGNITE_HEALTH_SOURCE=otel, deployment health comes from the health probes instead:
//! OTel source → `deployment-health` → the queries that read `ignite_reports` today.
use anyhow::{Context, Result};
use drasi_host_sdk::computation::{NativeBootstrapNodeFactory, NativeFactory, NativePlugin};
use drasi_lib::computation::v1::*;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

fn resource(value: &str) -> Result<ResourceId> {
    Ok(ResourceId::try_new(value)?)
}
fn stream(value: &str) -> Result<StreamId> {
    Ok(StreamId::try_new(format!("{value}/out"))?)
}
fn literal(value: impl serde::Serialize) -> Result<ConfigurationValue> {
    Ok(ConfigurationValue::Literal(serde_json::to_value(value)?))
}
fn reference(key: &str) -> Result<ConfigurationValue> {
    Ok(ConfigurationValue::Reference {
        resource: resource("configuration")?,
        key: key.into(),
        secret: true,
    })
}

const PG: &str = "postgres";
/// Drasi's native OpenTelemetry source (IGNITE_HEALTH_SOURCE=otel).
const OTEL: &str = "otel";
/// The query turning the health probes into report-shaped rows, and their label.
const HEALTH: &str = "deployment-health";
const HEALTH_LABEL: &str = "DeploymentHealth";

/// Where Drasi learns deployment health (`IGNITE_HEALTH_SOURCE` in ports.env).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Health {
    /// `ignite_reports` rows the health monitor writes, through the PostgreSQL source.
    Postgres,
    /// The health monitor's probe metrics, through the Collector and the OTel source.
    Otel,
}

impl Health {
    fn from_settings() -> Result<Self> {
        // A settings file without the key predates the switch: PostgreSQL.
        match ignite_settings::get().get("IGNITE_HEALTH_SOURCE") {
            Err(_) | Ok("postgres") => Ok(Health::Postgres),
            Ok("otel") => Ok(Health::Otel),
            Ok(other) => anyhow::bail!("IGNITE_HEALTH_SOURCE must be postgres or otel, not {other:?}"),
        }
    }
    /// The PostgreSQL tables the source replicates: not `ignite_reports` when health comes
    /// from telemetry, so no health reaches Drasi through PostgreSQL.
    fn tables(self) -> Vec<&'static str> {
        TABLES
            .into_iter()
            .filter(|table| self == Health::Postgres || *table != "ignite_reports")
            .collect()
    }
    /// A synthetic relationship's label as the queries see it in this mode.
    fn label(self, label: &'static str) -> &'static str {
        match (self, label) {
            (Health::Otel, "ignite_reports") => HEALTH_LABEL,
            _ => label,
        }
    }
}
const TABLES: [&str; 9] = [
    "ignite_scenario",
    "ignite_workloads",
    "ignite_deployments",
    "ignite_routes",
    "ignite_policy_destinations",
    "ignite_approvals",
    "ignite_tasks",
    "ignite_attempts",
    "ignite_reports",
];

/// Synthetic relationships the queries use: name → (label, property) = (label, property).
const JOINS: &[(&str, (&str, &str), (&str, &str))] = &[
    ("WORKLOAD_SCENARIO", ("ignite_workloads", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("DEPLOYMENT_SCENARIO", ("ignite_deployments", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("DESTINATION_SCENARIO", ("ignite_policy_destinations", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("WORKLOAD_DESTINATION", ("ignite_workloads", "id"), ("ignite_policy_destinations", "workload")),
    ("TASK_SCENARIO", ("ignite_tasks", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("ATTEMPT_SCENARIO", ("ignite_attempts", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("APPROVAL_SCENARIO", ("ignite_approvals", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("WORKLOAD_ROUTE", ("ignite_workloads", "id"), ("ignite_routes", "workload")),
    ("DEPLOYMENT_REPORT", ("ignite_deployments", "id"), ("ignite_reports", "id")),
    ("APPROVAL_ROUTE", ("ignite_approvals", "id"), ("ignite_routes", "approval")),
    ("ROUTE_REPORT", ("ignite_routes", "deployment"), ("ignite_reports", "id")),
    ("ROUTE_POLICY", ("ignite_routes", "workload"), ("PolicyResult", "subject")),
    ("TASK_APPROVAL", ("ignite_tasks", "completed_revision"), ("ignite_approvals", "applied_revision")),
    ("APPROVAL_PLAN", ("ignite_approvals", "run_epoch"), ("PlanResult", "run_epoch")),
    ("PLAN_RESILIENCE", ("PlanResult", "run_epoch"), ("ResilienceResult", "run_epoch")),
    ("RECOVERY_SUMMARY", ("RecoveryObservation", "run_epoch"), ("RecoveryTasks", "run_epoch")),
    // Incident cards (contracts.md section 15).
    ("ROUTE_SCENARIO", ("ignite_routes", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("PLAN_SCENARIO", ("PlanResult", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("RESILIENCE_SCENARIO", ("ResilienceResult", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("OVERDUE_SCENARIO", ("SupportOverdue", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("DEPLOYMENT_FACTS_SCENARIO", ("DeploymentFacts", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("POLICY_FACTS_SCENARIO", ("PolicyFacts", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("POLICY_WORKLOAD", ("PolicyResult", "subject"), ("ignite_workloads", "id")),
];

struct Query {
    name: &'static str,
    text: std::borrow::Cow<'static, str>,
    /// Upstream components feeding `in`; `postgres` also bootstraps the query.
    inputs: Vec<&'static str>,
    /// Label of this query's result rows as seen by downstream queries.
    label: Option<&'static str>,
}

macro_rules! query {
    ($name:literal, [$($input:literal),*] $(, $label:literal)?) => {
        Query {
            name: $name,
            text: std::borrow::Cow::Borrowed(include_str!(concat!("../../../drasi/", $name, ".cypher"))),
            inputs: vec![$($input),*],
            label: { #[allow(unused_mut, unused_assignments)] let mut label = None; $(label = Some($label);)? label },
        }
    };
}

/// The queries for `health`. From telemetry, `deployment-health` turns the probes into rows
/// shaped like `ignite_reports`, and every query reading reports reads those rows instead.
fn queries(health: Health) -> Vec<Query> {
    let mut queries = all_queries();
    if health == Health::Otel {
        for query in &mut queries {
            if query.text.contains("ignite_reports") {
                query.text = query.text.replace("ignite_reports", HEALTH_LABEL).into();
                query.inputs.push(HEALTH);
            }
        }
        queries.insert(0, query!("deployment-health", ["otel"], "DeploymentHealth"));
    }
    queries
}

fn all_queries() -> Vec<Query> {
    vec![
        // Context queries over the scenario tables: the plugins' inputs.
        query!("policy-input", ["postgres"], "PolicyInput"),
        query!("plan-context", ["postgres"], "PlanContext"),
        query!("plan-items", ["postgres"], "PlanItem"),
        query!("plan-bins", ["postgres"], "PlanBin"),
        query!("plan-candidates", ["postgres"], "PlanCandidate"),
        // Plugin results.
        query!("policy-results", ["policy"], "PolicyResult"),
        query!("plan-results", ["plan"], "PlanResult"),
        query!("assignment-results", ["plan"], "AssignmentResult"),
        query!("resilience-results", ["plan"], "ResilienceResult"),
        query!("loss-results", ["plan"]),
        // The application's requests.
        query!("overdue-tasks", ["postgres"]),
        query!("task-context", ["postgres"]),
        query!("attempt-context", ["postgres"]),
        query!("deadline-history", ["postgres"]),
        // Recovery after an applied approval.
        query!("recovery-task-context", ["postgres"], "RecoveryTasks"),
        query!("recovery-observation-context", ["postgres"], "RecoveryObservation"),
        query!(
            "recovery-results",
            ["recovery-task-context", "recovery-observation-context"],
            "RecoveryResult"
        ),
        // One query per Grafana tile (metric contract, contracts.md section 9).
        query!("tile-overdue", ["postgres"]),
        query!("tile-deployments", ["postgres"]),
        query!("tile-workloads", ["postgres", "policy-results"]),
        query!("tile-capacity", ["postgres"]),
        query!("tile-demand", ["postgres"]),
        query!("tile-recovery", ["recovery-results"]),
        query!("tile-change", ["postgres", "plan-results"]),
        query!("tile-action", ["plan-results", "resilience-results"]),
        query!("tile-resilience", ["resilience-results"]),
        query!("tile-running", ["postgres", "policy-results"]),
        query!("tile-proposed", ["assignment-results"]),
        query!("tile-policy", ["policy-results"]),
        query!("tile-last-approval", ["postgres"]),
        // Reactions' driving queries (contracts.md section 10).
        query!("approval-valid", ["postgres", "plan-results"]),
        query!("approval-stale", ["postgres", "plan-results"]),
        query!("recovery-verified", ["recovery-results"]),
        query!("approval-events", ["postgres"]),
        query!("plan-events", ["plan-results"]),
        query!("recovery-scores", ["postgres"]),
        // Incident cards (contracts.md section 15): helper rows the cards read...
        query!("incident-overdue-count", ["postgres"], "SupportOverdue"),
        query!("incident-deployments", ["postgres"], "DeploymentFacts"),
        query!("incident-policy", ["postgres", "policy-results"], "PolicyFacts"),
        // ...and one event query per card. Each row is one card: it appears once per event.
        query!("incident-failed", ["postgres"]),
        query!("incident-overdue", ["postgres", "incident-overdue-count"]),
        query!("incident-proposal", ["postgres", "plan-results", "incident-deployments", "incident-policy"]),
        query!("incident-superseded", ["postgres", "plan-results"]),
        query!("incident-outcome", ["postgres"]),
        query!("incident-at-risk", ["postgres", "plan-results", "resilience-results"]),
        query!("incident-restored", ["postgres", "plan-results", "resilience-results"]),
    ]
}

/// Edges into the native plugins' named input ports.
const PLUGIN_INPUTS: &[(&str, &str, &str)] = &[
    ("policy-input", "policy", "in"),
    ("plan-context", "plan", "in"),
    ("plan-items", "plan", "in"),
    ("plan-bins", "plan", "in"),
    ("plan-candidates", "plan", "in"),
    ("policy-results", "plan", "in"),
];

/// Prometheus reaction aliases (plugins/prometheus/examples/ignite.json) → tile query.
const TILES: &[(&str, &str)] = &[
    ("overdue", "tile-overdue"),
    ("deployments", "tile-deployments"),
    ("workloads", "tile-workloads"),
    ("capacity", "tile-capacity"),
    ("demand", "tile-demand"),
    ("recovery", "tile-recovery"),
    ("change", "tile-change"),
    ("action", "tile-action"),
    ("resilience", "tile-resilience"),
    ("running", "tile-running"),
    ("proposed", "tile-proposed"),
    ("policy", "tile-policy"),
    ("last-approval", "tile-last-approval"),
];

/// Results the desk and projector read live from the SSE reaction (alias = query name).
const DESK: &[&str] = &[
    "overdue-tasks", "plan-results", "tile-action", "tile-recovery", "tile-resilience",
    "tile-running", "tile-last-approval",
];

/// HTTP reactions: component → (alias, query).
const REACTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "gateway-reaction",
        &[("approve", "approval-valid"), ("reject", "approval-stale"), ("verified", "recovery-verified")],
    ),
    (
        "grafana-annotations",
        &[("approval-events", "approval-events"), ("plan-events", "plan-events")],
    ),
    ("langfuse-scores", &[("scores", "recovery-scores")]),
    (
        "incident-channel",
        &[
            ("failed", "incident-failed"),
            ("overdue", "incident-overdue"),
            ("proposal", "incident-proposal"),
            ("superseded", "incident-superseded"),
            ("outcome", "incident-outcome"),
            ("at-risk", "incident-at-risk"),
            ("restored", "incident-restored"),
        ],
    ),
];

/// Card colours (contracts.md section 15).
const RED: &str = "#d44a3a";
const ORANGE: &str = "#f2a33a";
const GREEN: &str = "#2fa66a";
const GREY: &str = "#9aa4b0";

/// One attachment field: (title, column, short). A title starting with `{{` is a template.
fn fields(list: &[(&str, &str, bool)]) -> String {
    list.iter()
        .map(|(title, column, short)| {
            let title = if title.starts_with("{{") { title.to_string() } else { json!(title).to_string() };
            format!(r#"{{"title":{title},"value":{{{{json after.{column}}}}},"short":{short}}}"#)
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// The body of `POST /api/v4/posts` for one card. Every string comes from the query;
/// the template only places it (`{{json …}}`). `pending_post_id` (run and card id) makes the
/// post idempotent: Mattermost returns the existing post for a repeated id, so a retried
/// delivery never posts a card twice.
fn card(channel: &str, card: &str, query: &str, color: &str, fields: &str, actions: &str) -> String {
    let color = if color.starts_with("{{") { color.to_string() } else { json!(color).to_string() };
    let actions = if actions.is_empty() { String::new() } else { format!(r#","actions":{actions}"#) };
    format!(
        r#"{{"channel_id":{channel},"root_id":{{{{json after.root_id}}}},"pending_post_id":"{{{{after.run_epoch}}}}/{{{{after.id}}}}","message":"","props":{{"ignite_card":{card},"ignite_id":{{{{json after.id}}}},"ignite_run_epoch":{{{{json after.run_epoch}}}},"attachments":[{{"fallback":{{{{json after.title}}}},"color":{color},"title":{{{{json after.title}}}},"text":{{{{json after.text}}}},"fields":[{fields}],"footer":{footer}{actions}}}]}}}}"#,
        channel = json!(channel),
        card = json!(card),
        footer = json!(format!("Drasi · {query}")),
    )
}

/// The proposal card's Approve and Reject buttons: Mattermost calls the AI service's
/// action endpoint from its container (contracts.md section 14).
fn proposal_actions(url: &str) -> String {
    let button = |id: &str, name: &str, style: &str, action: &str| {
        format!(
            r#"{{"id":{id},"name":{name},"type":"button","style":{style},"integration":{{"url":{url},"context":{{"action":{action},"candidate_id":{{{{json after.candidate_id}}}},"basis":{{{{json after.basis}}}},"revision":{{{{json after.revision}}}},"run_epoch":{{{{json after.run_epoch}}}},"change":{{{{json after.change}}}}}}}}}}"#,
            id = json!(id), name = json!(name), style = json!(style), url = json!(url), action = json!(action),
        )
    };
    format!("[{},{}]", button("approve", "Approve", "primary", "approve"), button("decline", "Reject", "danger", "decline"))
}

/// The `incident-channel` routes: every card is `added` → `POST /api/v4/posts`.
fn incident_routes(channel: &str, grafana_port: u16, workload_port: u16) -> Value {
    let dashboard = format!("[Operations › Support overdue](http://localhost:{grafana_port}/d/ignite-workload)");
    let actions = proposal_actions(&format!("http://host.containers.internal:{workload_port}/mattermost/actions"));
    json!({
        "failed":http_route("POST", "/api/v4/posts", &card(channel, "failed", "incident-failed", RED,
            &fields(&[("Region", "region", true), ("Capacity", "capacity", true), ("Runs here", "runs", true)]), ""), false),
        "overdue":http_route("POST", "/api/v4/posts", &card(channel, "overdue", "incident-overdue", RED,
            &(fields(&[("Overdue when posted", "overdue", true), ("Deadline", "deadline", true)])
                + &format!(r#",{{"title":"Dashboard","value":{},"short":true}}"#,
                    json!(dashboard))), ""), false),
        "proposal":http_route("POST", "/api/v4/posts", &card(channel, "proposal", "incident-proposal", ORANGE,
            &fields(&[("Why", "why", true), ("Peak use after", "peak", true), ("Policy", "policy", false),
                ("Capacity", "capacity", false), ("Based on", "based_on", true)]),
            &actions), false),
        "superseded":http_route("POST", "/api/v4/posts", &card(channel, "superseded", "incident-superseded", GREY,
            &fields(&[("Was", "was", false), ("Now", "now", false)]), ""), false),
        "outcome":http_route("POST", "/api/v4/posts", &card(channel, "outcome", "incident-outcome", "{{json after.color}}",
            &fields(&[("Approved by", "approved_by", true), ("{{json after.detail_title}}", "detail", false)]), ""), false),
        "at-risk":http_route("POST", "/api/v4/posts", &card(channel, "at-risk", "incident-at-risk", ORANGE,
            &fields(&[("Would fail", "lost", true), ("Affects", "affects", true)]), ""), false),
        "restored":http_route("POST", "/api/v4/posts", &card(channel, "restored", "incident-restored", GREEN,
            &fields(&[]), ""), false),
    })
}

/// IDs the Mattermost bootstrap wrote (`ops/mattermost.py`), read when the configuration is generated.
fn mattermost_channel() -> Result<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.native/mattermost.json");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read {} (run ./ops/observability-stack.sh up first)", path.display()))?;
    let ids: Value = serde_json::from_str(&text)?;
    Ok(ids["channel_id"].as_str().context("channel_id in mattermost.json")?.to_string())
}

// Shared by the live source and its bootstrap provider so they bind one slot.
// Database and user are fixed by compose.yaml and ops/observability-env.py;
// port, publication and slot come from ports.env.
fn postgres_settings(health: Health) -> Result<Value> {
    let settings = ignite_settings::get();
    Ok(json!({
        "host":"127.0.0.1", "port":settings.port("POSTGRES")?, "database":"ignite", "user":"ignite",
        "sslMode":"disable", "publication":settings.get("IGNITE_PG_PUBLICATION")?, "tables":health.tables(),
        "slotOwnership":"external", "slotName":settings.get("IGNITE_PG_SLOT")?,
        "maxTransactionChanges":4096, "maxTransactionBytes":8388608,
        "maxTransactionDurationMs":30000, "maxProtocolBytes":8388608,
        "ioTimeoutMs":10000, "feedbackIntervalMs":1000, "reconnectBudgetMs":60000,
    }))
}
fn bootstrap_configuration(
    source: &ComponentId,
    health: Health,
) -> Result<BTreeMap<Arc<str>, ConfigurationValue>> {
    let mut configuration = BTreeMap::from([
        ("source".into(), literal(source.as_str())?),
        ("stream".into(), literal(stream(PG)?)?),
        ("password".into(), reference("env:IGNITE_POSTGRES_PASSWORD")?),
    ]);
    for (key, value) in postgres_settings(health)?.as_object().context("settings")? {
        configuration.insert(key.as_str().into(), literal(value)?);
    }
    Ok(configuration)
}
fn declare(
    builder: ComponentBatchBuilder,
    name: &str,
    role: ResourceRole,
    recipe: Value,
) -> Result<ComponentBatchBuilder> {
    let id = resource(name)?;
    Ok(builder
        .declare_resource(ResourceSpecification {
            id: id.clone(),
            role,
            ownership: if role == ResourceRole::SecretStore {
                ResourceOwnership::Borrowed
            } else {
                ResourceOwnership::Graph
            },
            binding: name.into(),
        })?
        .resource_configuration(id, recipe)?)
}
fn load(directory: &Path, library: &str) -> Result<Arc<NativePlugin>> {
    drasi_host_sdk::computation::load(directory.join(format!(
        "{}{}{}",
        std::env::consts::DLL_PREFIX,
        library,
        std::env::consts::DLL_SUFFIX
    )))
    .with_context(|| format!("load matching native plugin {library}"))
}
fn find_factory(plugin: &NativePlugin, name: &str) -> Result<Arc<NativeFactory>> {
    plugin
        .factories()
        .iter()
        .find(|f| f.metadata().implementation.name.as_ref() == name)
        .cloned()
        .with_context(|| format!("factory {name}"))
}
/// The health probes as Drasi's OTel source sees them: `ignite_probe_status` per deployment
/// and run (`deployment-health` reads the identity attributes from the Metric's id).
fn otel_configuration() -> Result<Value> {
    Ok(json!({
        "stream":stream(OTEL)?, "host":"127.0.0.1", "protocols":["grpc"],
        "grpcPort":ignite_settings::get().port("DRASI_OTLP")?,
        "metricAllowlist":["ignite_probe_status"],
        "metricIdentityAttributes":["deployment", "run_epoch"],
        // Metrics never expire and each run adds one per deployment.
        "maxMetrics":1000000,
        // Only probes reach the source: no dependencies or log events.
        "destinationAttributes":[],
    }))
}
/// Relations a source delivers (Drasi's OTel source: Service-[:REPORTS]->Metric), not joins.
const SOURCE_RELATIONS: &[&str] = &["REPORTS"];
fn joins(text: &str, health: Health) -> Result<Vec<Value>> {
    let used: BTreeSet<_> = text
        .split(":")
        .skip(1)
        .filter_map(|rest| rest.split(']').next())
        .filter(|name| name.chars().all(|c| c.is_ascii_uppercase() || c == '_') && !name.is_empty())
        .filter(|name| !SOURCE_RELATIONS.contains(name))
        .collect();
    used.into_iter()
        .map(|name| {
            let (_, (l1, p1), (l2, p2)) = JOINS
                .iter()
                .find(|(join, ..)| *join == name)
                .with_context(|| format!("unknown relationship {name}"))?;
            let (l1, l2) = (health.label(l1), health.label(l2));
            Ok(json!({"id":name, "keys":[{"label":l1,"property":p1},{"label":l2,"property":p2}]}))
        })
        .collect()
}
fn http_route(method: &str, url: &str, template: &str, also_updated: bool) -> Value {
    let spec = json!({"method":method, "url":url, "template":template});
    if also_updated {
        json!({"added":spec, "updated":spec})
    } else {
        json!({"added":spec})
    }
}

fn main() -> Result<()> {
    let directory = std::env::args()
        .nth(1)
        .context("usage: ignite-config <plugin-directory>")?;
    let directory = Path::new(&directory);
    let settings = ignite_settings::get();
    let health = Health::from_settings()?;
    let otel = match health {
        Health::Postgres => None,
        Health::Otel => Some(
            load(directory, "drasi_computation_source_otel")
                .context("IGNITE_HEALTH_SOURCE=otel needs the native OTel source plugin")?,
        ),
    };
    let pg = load(directory, "drasi_computation_source_postgres")?;
    let bootstrap = load(directory, "drasi_computation_bootstrap_postgres")?;
    let query_plugin = load(directory, "drasi_computation_transformer_query")?;
    let http_sink = load(directory, "drasi_computation_sink_http")?;
    let policy = load(directory, "drasi_computation_transformer_policy")?;
    let optimization = load(directory, "drasi_computation_transformer_optimization")?;
    let prometheus = load(directory, "drasi_computation_ignite_prometheus")?;
    let sse = load(directory, "drasi_computation_sink_sse")?;
    let source_factory = find_factory(&pg, "drasi.postgres/shared-changes")?;
    let bootstrap = bootstrap
        .bootstrap_factories()
        .iter()
        .find(|f| f.metadata().implementation.name.as_ref() == "drasi.postgres/shared-row-bootstrap")
        .cloned()
        .context("PostgreSQL shared-row bootstrap")?;
    let bootstrap = Arc::new(NativeBootstrapNodeFactory::new(bootstrap));
    let query_factory = query_plugin.query_factory()?;
    let queries = queries(health);
    let fed_by_postgres: Vec<&str> = queries
        .iter()
        .filter(|q| q.inputs.contains(&PG))
        .map(|q| q.name)
        .collect();

    let plugins = ["policy", "plan"];
    let ids: BTreeMap<&str, ComponentId> = [PG, "postgres-bootstrap", "query-results", "metrics", "desk-stream"]
        .into_iter()
        .chain(otel.as_ref().map(|_| OTEL))
        .chain(plugins)
        .chain(REACTIONS.iter().map(|(name, _)| *name))
        .chain(queries.iter().map(|q| q.name))
        .map(|name| (name, ComponentId::generate()))
        .collect();
    let endpoint = |name: &str, port: &str| -> Result<Endpoint> {
        Ok(Endpoint::new(
            ids.get(name).with_context(|| format!("component {name}"))?.clone(),
            PortId::try_new(port)?,
        ))
    };
    let outputs = |names: &mut dyn Iterator<Item = &str>| -> Value {
        Value::Array(names.map(|name| json!({"component":ids[name],"port":"out"})).collect())
    };

    // Resources.
    let mut builder = ComponentBatch::builder();
    let consumers: serde_json::Map<String, Value> = fed_by_postgres
        .iter()
        .map(|name| (ids[name].as_str().to_string(), json!(format!("{name}-progress"))))
        .collect();
    for (name, role, recipe) in [
        ("configuration", ResourceRole::SecretStore, json!({"kind":"configuration"})),
        ("indexes", ResourceRole::IndexBackend, json!({"kind":"memoryIndexes"})),
        ("catalog", ResourceRole::QueryCatalog, json!({"kind":"queryCatalog"})),
        (
            "pg-consumers",
            ResourceRole::Checkpoint,
            json!({"kind":"consumerProgress","producer":ids[PG],"consumers":consumers}),
        ),
    ] {
        builder = declare(builder, name, role, recipe)?;
    }
    for name in &fed_by_postgres {
        let progress = format!("{name}-progress");
        builder = declare(
            builder,
            &progress,
            ResourceRole::Checkpoint,
            json!({"kind":"sourceProgress","component":ids[name],"indexes":"indexes"}),
        )?;
        // Source progress inherits its query's index durability.
        builder = builder.resource_dependency(
            resource(&progress)?,
            resource("indexes")?,
            ResourceRole::IndexBackend,
        )?;
        builder = builder.resource_dependency(
            resource("pg-consumers")?,
            resource(&progress)?,
            ResourceRole::Checkpoint,
        )?;
    }
    // Each native plugin bootstraps from its upstream outputs' snapshots.
    let mut snapshot_inputs: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (from, to, _) in PLUGIN_INPUTS {
        let inputs = snapshot_inputs.entry(to).or_default();
        if !inputs.contains(from) {
            inputs.push(from);
        }
    }
    snapshot_inputs.insert("metrics", TILES.iter().map(|(_, query)| *query).collect());
    snapshot_inputs.insert("desk-stream", DESK.to_vec());
    for (component, inputs) in &snapshot_inputs {
        builder = declare(
            builder,
            &format!("{component}-snapshots"),
            ResourceRole::OutputCatalogue,
            json!({"kind":"nativeOutputSnapshots","outputs":outputs(&mut inputs.iter().copied()),
                "timeoutMs":10000,"maxReaders":4}),
        )?;
    }

    // PostgreSQL source and its bootstrap provider.
    let mut configuration = postgres_settings(health)?;
    configuration["stream"] = json!(stream(PG)?);
    configuration["persistent"] = json!(false);
    let mut source = source_factory.specification(ids[PG].clone(), configuration)?;
    source
        .configuration
        .insert("password".into(), reference("env:IGNITE_POSTGRES_PASSWORD")?);
    source
        .dependencies
        .insert("consumer_progress".into(), vec![resource("pg-consumers")?]);
    builder = builder.component(ComponentName::try_new(PG)?, source, source_factory);
    // The source relays each query's bootstrap session to this provider, which
    // records progress for every query.
    let provider = ComponentSpecification {
        descriptor: NativeBootstrapNodeFactory::component_descriptor(ids["postgres-bootstrap"].clone())?,
        role: ComponentRole::BootstrapProvider,
        completion: None,
        implementation: bootstrap.descriptor().implementation.clone(),
        configuration_version: bootstrap.descriptor().configuration_version,
        configuration: bootstrap_configuration(&ids[PG], health)?,
        dependencies: BTreeMap::from([(
            "source_progress".into(),
            fed_by_postgres
                .iter()
                .map(|name| resource(&format!("{name}-progress")))
                .collect::<Result<Vec<_>>>()?,
        )]),
    };
    builder = builder.component(ComponentName::try_new("postgres-bootstrap")?, provider, bootstrap);

    // OpenTelemetry source: the health probes, from the Collector. No bootstrap: its state
    // is volatile and the probes arrive every second.
    if let Some(otel) = &otel {
        let factory = find_factory(otel, "drasi.network/otel-source")?;
        let specification = factory.specification(ids[OTEL].clone(), otel_configuration()?)?;
        builder = builder.component(ComponentName::try_new(OTEL)?, specification, factory);
    }

    // Queries.
    for query in &queries {
        let result = match query.label {
            Some(label) => json!({"label":label}),
            None => json!({}),
        };
        let mut specification = query_factory.specification_from_config(
            ids[query.name].clone(),
            json!({
                "query":query.text, "stream":stream(query.name)?, "outbox_capacity":128,
                "execution":{"joins":joins(&query.text, health)?, "result":result},
            }),
        )?;
        specification.dependencies = BTreeMap::from([
            ("indexes".into(), vec![resource("indexes")?]),
            ("catalog".into(), vec![resource("catalog")?]),
        ]);
        if query.inputs.contains(&PG) {
            specification.dependencies.insert(
                "source_progress".into(),
                vec![resource(&format!("{}-progress", query.name))?],
            );
        }
        builder = builder.component(
            ComponentName::try_new(query.name)?,
            specification,
            query_factory.clone(),
        );
    }

    // Drasi's Rego policy (drasi.standard/policy) decides each `policy-input` row; the
    // module and the input and output mappings are its configuration (drasi/policy).
    let mut rego: Value = serde_json::from_str(include_str!("../../../drasi/policy/policy.json"))?;
    rego["modules"][0]["source"] = json!(include_str!("../../../drasi/policy/ignite.rego"));
    rego["stream"] = json!(stream(ids["policy"].as_str())?);
    let factory = find_factory(&policy, "drasi.standard/policy")?;
    let mut specification = factory.specification(ids["policy"].clone(), rego)?;
    // A query keeps its bootstrap to itself: the policy starts from `policy-input`'s snapshot.
    specification
        .dependencies
        .insert("output_snapshots".into(), vec![resource("policy-snapshots")?]);
    builder = builder.component(ComponentName::try_new("policy")?, specification, factory);
    // Drasi's optimization (drasi.standard/optimization): the placement model, its
    // single-loss what-if and its preconditions are its configuration (drasi/optimization).
    let mut model: Value =
        serde_json::from_str(include_str!("../../../drasi/optimization/optimization.json"))?;
    model["stream"] = json!(stream(ids["plan"].as_str())?);
    let factory = find_factory(&optimization, "drasi.standard/optimization")?;
    let mut specification = factory.specification(ids["plan"].clone(), model)?;
    // It starts from its input queries' snapshots.
    specification
        .dependencies
        .insert("output_snapshots".into(), vec![resource("plan-snapshots")?]);
    builder = builder.component(ComponentName::try_new("plan")?, specification, factory);

    // Prometheus reaction: every tile query's rows as gauges.
    let mut metrics: Value =
        serde_json::from_str(include_str!("../../../plugins/prometheus/examples/ignite.json"))?;
    metrics["listen"] = json!(format!("127.0.0.1:{}", settings.port("DRASI_METRICS")?));
    metrics["outputs"] = Value::Object(
        TILES
            .iter()
            .map(|(alias, query)| (alias.to_string(), json!({"component":ids[query],"port":"out"})))
            .collect(),
    );
    let factory = find_factory(&prometheus, "ignite/prometheus-exporter")?;
    let mut specification = factory.specification(ids["metrics"].clone(), metrics)?;
    specification
        .dependencies
        .insert("output_snapshots".into(), vec![resource("metrics-snapshots")?]);
    builder = builder.component(ComponentName::try_new("metrics")?, specification, factory);

    // SSE reaction: the desk's answers on one event stream, each output's snapshot at
    // /snapshot/{alias}.
    let ui = settings.port("UI")?;
    let factory = find_factory(&sse, "drasi.network/sse-sink")?;
    let mut specification = factory.specification(
        ids["desk-stream"].clone(),
        json!({
            "host":"127.0.0.1", "port":settings.port("DRASI_SSE")?, "ssePath":"/events",
            "heartbeatIntervalMs":10000,
            "corsAllowedOrigins":[format!("http://127.0.0.1:{ui}"), format!("http://localhost:{ui}")],
            "outputs":Value::Object(DESK.iter()
                .map(|query| (query.to_string(), json!({"component":ids[query],"port":"out"})))
                .collect()),
        }),
    )?;
    specification
        .dependencies
        .insert("output_snapshots".into(), vec![resource("desk-stream-snapshots")?]);
    builder = builder.component(ComponentName::try_new("desk-stream")?, specification, factory);

    // HTTP reactions: the gateway configuration API, Grafana annotations, Langfuse scores,
    // and the incident cards in Mattermost's #incident-ai-support.
    let workload = settings.url("WORKLOAD")?;
    let grafana = format!("http://127.0.0.1:{}", settings.port("GRAFANA")?);
    let langfuse = format!("http://127.0.0.1:{}", settings.port("LANGFUSE")?);
    let mattermost = format!("http://127.0.0.1:{}", settings.port("MATTERMOST")?);
    let channel = mattermost_channel()?;
    let annotation = r#"{"time":{{after.at_ms}},"tags":["ignite-decision","{{after.id}}"],"text":{{json after.text}}}"#;
    let plan_annotation = r#"{"tags":["ignite-decision","{{after.id}}"],"text":{{json after.text}}}"#;
    let reaction_configs: BTreeMap<&str, (Value, Vec<(&str, ConfigurationValue)>)> = BTreeMap::from([
        ("gateway-reaction", (json!({
            "baseUrl":workload, "onPermanentFailure":"stall", "maxRetries":10, "retryBudgetMs":30000, "timeoutMs":5000,
            "outputTemplates":{"routes":{
                "approve":http_route("POST", "/routes/apply",
                    r#"{"approval":{{json after.approval}},"candidate_id":{{json after.candidate_id}},"basis":{{json after.basis}},"expected_revision":{{json after.expected_revision}},"assignments":{{{after.assignments}}}}"#, false),
                "reject":http_route("POST", "/approvals/{{after.approval}}/reject", r#"{"reason":{{json after.reason}}}"#, false),
                "verified":http_route("POST", "/approvals/{{after.approval}}/verified", r#"{"verified_ms":{{json after.verified_ms}}}"#, false),
            }},
        }), vec![])),
        ("grafana-annotations", (json!({
            "baseUrl":grafana, "onPermanentFailure":"skip", "maxRetries":3, "timeoutMs":5000,
            "outputTemplates":{"routes":{
                "approval-events":http_route("POST", "/api/annotations", annotation, true),
                "plan-events":http_route("POST", "/api/annotations", plan_annotation, true),
            }},
        }), vec![("token", reference("env:IGNITE_GRAFANA_TOKEN")?)])),
        ("langfuse-scores", (json!({
            "baseUrl":langfuse, "onPermanentFailure":"skip", "maxRetries":3, "timeoutMs":5000,
            "outputTemplates":{"routes":{
                "scores":http_route("POST", "/api/public/scores",
                    r#"{"id":{{json after.id}},"traceId":{{json after.trace_id}},"name":"drasi_recovery","value":1,"dataType":"NUMERIC","comment":{{json after.comment}}}"#, false),
            }},
        }), vec![("secretHeaders", reference("env-json:IGNITE_LANGFUSE_HEADERS")?)])),
        ("incident-channel", (json!({
            "baseUrl":mattermost, "onPermanentFailure":"skip", "maxRetries":3, "timeoutMs":5000,
            // Below Mattermost's 60 s keep-alive idle timeout, so a post never goes out on a
            // connection Mattermost is closing.
            "poolIdleTimeoutMs":30000,
            "successStatusCodes":[201],
            "outputTemplates":{"routes":incident_routes(&channel, settings.port("GRAFANA")?, settings.port("WORKLOAD")?)},
        }), vec![("token", reference("env:IGNITE_MATTERMOST_BOT_TOKEN")?)])),
    ]);
    let sink_factory = find_factory(&http_sink, "drasi.network/http-sink")?;
    for (name, aliases) in REACTIONS {
        let (mut configuration, secrets) = reaction_configs[name].clone();
        if configuration.get("successStatusCodes").is_none() {
            configuration["successStatusCodes"] = json!([200]);
        }
        // Each route lists the operations it acts on; other changes send nothing.
        configuration["unroutedChanges"] = json!("ignore");
        configuration["outputs"] = Value::Object(
            aliases
                .iter()
                .map(|(alias, query)| (alias.to_string(), json!({"component":ids[query],"port":"out"})))
                .collect(),
        );
        let mut specification = sink_factory.specification(ids[name].clone(), configuration)?;
        for (key, value) in secrets {
            specification.configuration.insert(key.into(), value);
        }
        builder = builder.component(ComponentName::try_new(*name)?, specification, sink_factory.clone());
    }

    // Every query's results through Server's query API (attach, snapshot).
    let factory = Arc::new(QueryResultsOutletFactory::default());
    let mut outlet = factory.specification_from_config(ids["query-results"].clone(), json!({}))?;
    outlet
        .dependencies
        .insert("catalog".into(), vec![resource("catalog")?]);
    builder = builder.component(ComponentName::try_new("query-results")?, outlet, factory);

    // Edges.
    let pipe = |capacity| Box::new(BoundedPipeConfig { capacity });
    let requires_running = || RelationshipPolicy {
        activation: ActivationCoupling::RequiresRunning,
        ..RelationshipPolicy::default()
    };
    for query in &queries {
        for input in &query.inputs {
            let edge = EdgeDefinition::new(endpoint(input, "out")?, endpoint(query.name, "in")?);
            builder = builder.connect(edge.clone(), pipe(32));
            if *input != PG && *input != OTEL && !plugins.contains(input) {
                builder = builder.relationship_policy(edge, requires_running());
            }
        }
        builder = builder.connect(
            EdgeDefinition::new(endpoint(query.name, "out")?, endpoint("query-results", "in")?),
            pipe(32),
        );
    }
    for (from, to, port) in PLUGIN_INPUTS {
        let edge = EdgeDefinition::new(endpoint(from, "out")?, endpoint(to, port)?);
        builder = builder
            .connect(edge.clone(), pipe(32))
            .relationship_policy(edge, requires_running());
    }
    for (_, query) in TILES {
        builder = builder.connect(
            EdgeDefinition::new(endpoint(query, "out")?, endpoint("metrics", "in")?),
            pipe(32),
        );
    }
    for query in DESK {
        builder = builder.connect(
            EdgeDefinition::new(endpoint(query, "out")?, endpoint("desk-stream", "in")?),
            pipe(32),
        );
    }
    for (name, aliases) in REACTIONS {
        for (_, query) in *aliases {
            builder = builder.connect(
                EdgeDefinition::new(endpoint(query, "out")?, endpoint(name, "in")?),
                pipe(32),
            );
        }
    }
    // Queries reading PostgreSQL bootstrap through the source, which relays to its provider.
    for (from, to) in std::iter::once(("postgres-bootstrap", PG))
        .chain(fed_by_postgres.iter().map(|name| (PG, *name)))
    {
        builder = builder.connect(
            EdgeDefinition::new(endpoint(from, "snapshot")?, endpoint(to, "bootstrap")?),
            pipe(8),
        );
    }
    for name in std::iter::once(PG)
        .chain(otel.as_ref().map(|_| OTEL))
        .chain(queries.iter().map(|q| q.name))
    {
        builder = builder.bind_stream(endpoint(name, "out")?, stream(name)?);
    }
    // The plugins name their output stream after their component ID.
    for name in plugins {
        builder = builder.bind_stream(endpoint(name, "out")?, stream(ids[name].as_str())?);
    }
    let graph = builder.build()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "apiVersion":"drasi.io/v1","host":"127.0.0.1","port":settings.port("DRASI_API")?,
            "persistConfig":false,"enableUi":true,"verifyPlugins":false,"autoInstallPlugins":false,
            "instances":[{"id":"ignite",
                // Each PostgreSQL query snapshots the tables in turn; with a
                // long run's history a restart takes minutes.
                "activationTimeoutSecs":1200,
                "computation":{"definition":graph.definition}}]
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders the sink's `{{json after.x}}` placeholders, and `{{after.x}}` (a string column
    /// placed inside a JSON string, as in `pending_post_id`).
    fn render(template: &str, row: &Value) -> Value {
        let mut out = String::new();
        let mut rest = template;
        while let Some(start) = rest.find("{{") {
            out.push_str(&rest[..start]);
            let end = rest[start..].find("}}").expect("closed placeholder") + start;
            let placeholder = &rest[start + 2..end];
            if let Some(column) = placeholder.strip_prefix("json after.") {
                out.push_str(&row.get(column).unwrap_or_else(|| panic!("no column {column}")).to_string());
            } else {
                let column = placeholder
                    .strip_prefix("after.")
                    .unwrap_or_else(|| panic!("only {{{{json after.…}}}} and {{{{after.…}}}} are used: {}", &rest[start..end + 2]));
                out.push_str(row.get(column).and_then(Value::as_str).unwrap_or_else(|| panic!("no string column {column}")));
            }
            rest = &rest[end + 2..];
        }
        out.push_str(rest);
        serde_json::from_str(&out).expect("a card renders to JSON")
    }

    fn row(extra: Value) -> Value {
        let mut row = json!({"id":"card-1","run_epoch":"run-1","root_id":"","title":"Title","text":"Text."});
        for (k, v) in extra.as_object().unwrap() {
            row[k] = v.clone();
        }
        row
    }

    fn cards() -> Value {
        incident_routes("channel-1", 14000, 19500)
    }

    fn attachment(card: &str, extra: Value) -> (Value, Value) {
        let route = &cards()[card]["added"];
        assert_eq!(route["method"], "POST");
        assert_eq!(route["url"], "/api/v4/posts");
        let body = render(route["template"].as_str().unwrap(), &row(extra));
        let attachment = body["props"]["attachments"][0].clone();
        (body, attachment)
    }

    fn field_titles(attachment: &Value) -> Vec<String> {
        attachment["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["title"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn every_card_is_a_post_in_the_channel_with_its_query_footer() {
        for (alias, query) in REACTIONS.iter().find(|(name, _)| *name == "incident-channel").unwrap().1 {
            let route = &cards()[alias];
            assert!(route.get("updated").is_none() && route.get("deleted").is_none(), "{alias}: only additions post");
            let template = route["added"]["template"].as_str().unwrap();
            let text = queries(Health::Postgres).into_iter().find(|q| q.name == *query).unwrap().text;
            for column in template.split("{{json after.").skip(1).map(|r| r.split("}}").next().unwrap()) {
                assert!(
                    text.contains(&format!(" AS {column}")) || text.contains(&format!(" {column},")) || text.contains(&format!(" {column}\n")) || text.trim_end().ends_with(column),
                    "{query} returns no column {column}"
                );
            }
            let row = row(json!({"region":"EU","capacity":"100k tokens/min","runs":"support","overdue":"3","deadline":"15 s",
                "why":"EU-A is down","peak":"80%","policy":"p","based_on":"revision 1 · plan c","candidate_id":"c","basis":"b",
                "revision":1,"change":"Move summaries to US-C","was":"w","now":"n","color":"#2fa66a","approved_by":"@oncall",
                "detail_title":"Change","detail":"d","lost":"EU-B","affects":"support"}));
            let body = render(template, &row);
            assert_eq!(body["channel_id"], "channel-1");
            assert_eq!(body["message"], "");
            assert_eq!(body["props"]["ignite_card"], *alias);
            assert_eq!(body["props"]["ignite_id"], "card-1");
            assert_eq!(body["props"]["ignite_run_epoch"], "run-1");
            assert_eq!(body["pending_post_id"], "run-1/card-1", "{alias}: a repeated delivery is the same post");
            let attachment = &body["props"]["attachments"][0];
            assert_eq!(attachment["fallback"], "Title");
            assert_eq!(attachment["title"], "Title");
            assert_eq!(attachment["text"], "Text.");
            assert_eq!(attachment["footer"], format!("Drasi · {query}"));
        }
    }

    #[test]
    fn cards_have_the_contract_colours_and_fields() {
        let (_, a) = attachment("failed", json!({"region":"EU","capacity":"100k tokens/min","runs":"support"}));
        assert_eq!((a["color"].as_str().unwrap(), field_titles(&a)), (RED, vec!["Region".into(), "Capacity".into(), "Runs here".into()]));
        assert_eq!(a["fields"][2], json!({"title":"Runs here","value":"support","short":true}));
        let (_, a) = attachment("overdue", json!({"overdue":"3","deadline":"15 s"}));
        assert_eq!(a["color"], RED);
        assert_eq!(field_titles(&a), ["Overdue when posted", "Deadline", "Dashboard"]);
        assert_eq!(a["fields"][2]["value"], "[Operations › Support overdue](http://localhost:14000/d/ignite-workload)");
        let (_, a) = attachment("superseded", json!({"was":"Move summaries to US-C","now":"No valid plan"}));
        assert_eq!((a["color"].as_str().unwrap(), field_titles(&a)), (GREY, vec!["Was".into(), "Now".into()]));
        assert_eq!(a["fields"][0]["short"], false);
        let (_, a) = attachment("at-risk", json!({"lost":"EU-B","affects":"support"}));
        assert_eq!((a["color"].as_str().unwrap(), field_titles(&a)), (ORANGE, vec!["Would fail".into(), "Affects".into()]));
        let (_, a) = attachment("restored", json!({}));
        assert_eq!((a["color"].as_str().unwrap(), a["fields"].clone()), (GREEN, json!([])));
        assert!(a.get("actions").is_none());
    }

    #[test]
    fn outcomes_reply_in_the_approved_cards_thread() {
        let (body, a) = attachment("outcome", json!({"root_id":"post-7","color":"#3d8bd9","approved_by":"@oncall",
            "detail_title":"Routes","detail":"support → EU-B · summaries → US-C"}));
        assert_eq!(body["root_id"], "post-7");
        assert_eq!(a["color"], "#3d8bd9");
        assert_eq!(a["fields"], json!([{"title":"Approved by","value":"@oncall","short":true},
            {"title":"Routes","value":"support → EU-B · summaries → US-C","short":false}]));
    }

    #[test]
    fn the_proposal_card_has_approve_and_reject_buttons_for_its_plan() {
        let (body, a) = attachment("proposal", json!({"why":"EU-A is down","peak":"80%","policy":"p","capacity":"c",
            "based_on":"revision 7 · plan abcdef12","candidate_id":"cand","basis":"bas","revision":7,
            "change":"Move summaries to US-C and support to EU-B"}));
        assert_eq!(body["root_id"], "");
        assert_eq!(a["color"], ORANGE);
        assert_eq!(field_titles(&a), ["Why", "Peak use after", "Policy", "Capacity", "Based on"]);
        let shorts: Vec<bool> = a["fields"].as_array().unwrap().iter().map(|f| f["short"].as_bool().unwrap()).collect();
        assert_eq!(shorts, [true, true, false, false, true]);
        let context = json!({"candidate_id":"cand","basis":"bas","revision":7,"run_epoch":"run-1",
            "change":"Move summaries to US-C and support to EU-B"});
        for (i, (id, name, style)) in [("approve", "Approve", "primary"), ("decline", "Reject", "danger")].into_iter().enumerate() {
            let button = &a["actions"][i];
            assert_eq!((button["id"].as_str(), button["name"].as_str(), button["type"].as_str(), button["style"].as_str()),
                (Some(id), Some(name), Some("button"), Some(style)));
            assert_eq!(button["integration"]["url"], "http://host.containers.internal:19500/mattermost/actions");
            let mut expected = context.clone();
            expected["action"] = json!(id);
            assert_eq!(button["integration"]["context"], expected);
        }
    }

    #[test]
    fn every_query_uses_declared_joins_or_source_relations() {
        for health in [Health::Postgres, Health::Otel] {
            for query in queries(health) {
                joins(&query.text, health).unwrap_or_else(|e| panic!("{health:?} {}: {e}", query.name));
            }
        }
        let adapter = queries(Health::Otel).into_iter().find(|q| q.name == HEALTH).unwrap();
        assert!(joins(&adapter.text, Health::Otel).unwrap().is_empty(), "REPORTS comes from the OTel source");
    }

    #[test]
    fn card_queries_and_their_helpers_use_declared_joins() {
        for health in [Health::Postgres, Health::Otel] {
            for query in queries(health).iter().filter(|q| q.name.starts_with("incident-")) {
                joins(&query.text, health).unwrap_or_else(|e| panic!("{}: {e}", query.name));
            }
        }
    }

    const HEALTH_QUERIES: [&str; 7] = [
        "plan-bins", "recovery-observation-context", "tile-deployments", "tile-workloads", "tile-capacity",
        "incident-deployments", "incident-failed",
    ];

    #[test]
    fn health_from_postgres_is_the_graph_of_today() {
        let queries = queries(Health::Postgres);
        assert!(queries.iter().all(|q| q.name != HEALTH && !q.inputs.contains(&HEALTH) && !q.inputs.contains(&OTEL)));
        assert!(queries.iter().all(|q| matches!(q.text, std::borrow::Cow::Borrowed(_))), "query files unchanged");
        let readers: Vec<_> = queries.iter().filter(|q| q.text.contains("ignite_reports")).map(|q| q.name).collect();
        assert_eq!(readers.len(), HEALTH_QUERIES.len());
        assert!(HEALTH_QUERIES.iter().all(|name| readers.contains(name)), "{readers:?}");
        assert_eq!(Health::Postgres.tables(), TABLES);
        let report = joins("(d)-[:DEPLOYMENT_REPORT]->(r)", Health::Postgres).unwrap();
        assert_eq!(report[0]["keys"][1]["label"], "ignite_reports");
    }

    #[test]
    fn health_from_telemetry_reads_probe_rows_instead_of_reports() {
        let queries = queries(Health::Otel);
        let adapter = queries.iter().find(|q| q.name == HEALTH).expect("deployment-health");
        assert_eq!((adapter.inputs.as_slice(), adapter.label), (&[OTEL][..], Some(HEALTH_LABEL)));
        let readers: Vec<_> = queries.iter().filter(|q| q.name != HEALTH && q.text.contains("ignite_reports")).map(|q| q.name).collect();
        assert!(readers.is_empty(), "no query reads reports: {readers:?}");
        for name in HEALTH_QUERIES {
            let query = queries.iter().find(|q| q.name == name).unwrap();
            assert!(query.text.contains(HEALTH_LABEL), "{name}");
            let from_postgres = all_queries().into_iter().find(|q| q.name == name).unwrap().inputs;
            assert_eq!(query.inputs, [from_postgres, vec![HEALTH]].concat(), "{name}: its inputs and the health rows");
        }
        let others = queries.iter().filter(|q| q.name != HEALTH && !HEALTH_QUERIES.contains(&q.name));
        assert!(others.into_iter().all(|q| !q.inputs.contains(&HEALTH) && !q.inputs.contains(&OTEL)));
        assert!(!Health::Otel.tables().contains(&"ignite_reports"));
        assert_eq!(Health::Otel.tables().len(), TABLES.len() - 1);
        for (text, join) in [("(d)-[:DEPLOYMENT_REPORT]->(r)", "DEPLOYMENT_REPORT"), ("(rt)-[:ROUTE_REPORT]->(r)", "ROUTE_REPORT")] {
            let keys = &joins(text, Health::Otel).unwrap()[0];
            assert_eq!((keys["id"].as_str(), keys["keys"][1]["label"].as_str()), (Some(join), Some(HEALTH_LABEL)));
        }
    }

    /// Fields documented for drasi.network/otel-source configuration version 1
    /// (drasi-core PR 1030, CONFIGURATION.md); the source refuses unknown keys.
    #[test]
    fn otel_source_configuration_uses_documented_fields() {
        const FIELDS: &[&str] = &[
            "stream", "host", "protocols", "grpcPort", "httpPort", "timeoutMs", "ingressCapacity",
            "maxMessageBytes", "maxConnections", "headerReadTimeoutMs", "authToken", "authTokenSecondary",
            "allowUnauthenticated", "rateLimitPerSecond", "rateLimitBurst", "tlsCertificatePem",
            "tlsPrivateKeyPem", "tlsClientCaPem", "metricAllowlist", "metricIdentityAttributes",
            "destinationAttributes", "spanKinds", "heartbeatMetric", "heartbeatEventName", "logMinSeverity",
            "logEventNameAllowlist", "logEventTtlSecs", "dependencyTtlSecs", "maxServices", "maxMetrics",
            "maxDependencies", "maxLogEvents", "rejectDerived",
        ];
        let configuration = otel_configuration().unwrap();
        for key in configuration.as_object().unwrap().keys() {
            assert!(FIELDS.contains(&key.as_str()), "undocumented field {key}");
        }
        // Loopback needs no token; the Collector reaches it through host.containers.internal.
        assert_eq!(configuration["host"], "127.0.0.1");
        assert_eq!(configuration["metricIdentityAttributes"], json!(["deployment", "run_epoch"]));
    }
}

#[cfg(test)]
mod incident_tests;
