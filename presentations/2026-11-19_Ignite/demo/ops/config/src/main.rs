//! Generates this copy's Drasi Server configuration (plan/contracts.md section 4):
//! PostgreSQL source → context queries → Rego policy → policy-results → optimizer
//! (plan, resilience) → result queries → tile queries → Prometheus reaction, and the
//! approval, recovery, annotation and score queries → HTTP reactions.
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
const TABLES: [&str; 10] = [
    "ignite_scenario",
    "ignite_workloads",
    "ignite_deployments",
    "ignite_routes",
    "ignite_policy_destinations",
    "ignite_approvals",
    "ignite_tasks",
    "ignite_attempts",
    "ignite_reports",
    "ignite_breaches",
];

/// Synthetic relationships the queries use: name → (label, property) = (label, property).
const JOINS: &[(&str, (&str, &str), (&str, &str))] = &[
    ("WORKLOAD_SCENARIO", ("ignite_workloads", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("DEPLOYMENT_SCENARIO", ("ignite_deployments", "run_epoch"), ("ignite_scenario", "run_epoch")),
    ("DESTINATION_SCENARIO", ("ignite_policy_destinations", "run_epoch"), ("ignite_scenario", "run_epoch")),
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
];

struct Query {
    name: &'static str,
    text: &'static str,
    /// Upstream components feeding `in`; `postgres` also bootstraps the query.
    inputs: &'static [&'static str],
    /// Label of this query's result rows as seen by downstream queries.
    label: Option<&'static str>,
}

macro_rules! query {
    ($name:literal, [$($input:literal),*] $(, $label:literal)?) => {
        Query {
            name: $name,
            text: include_str!(concat!("../../../drasi/", $name, ".cypher")),
            inputs: &[$($input),*],
            label: { #[allow(unused_mut, unused_assignments)] let mut label = None; $(label = Some($label);)? label },
        }
    };
}

fn queries() -> Vec<Query> {
    vec![
        // Context queries over the scenario tables: the plugins' inputs.
        query!("policy-workloads", ["postgres"]),
        query!("policy-deployments", ["postgres"]),
        query!("policy-destinations", ["postgres"]),
        query!("plan-items", ["postgres"]),
        query!("plan-bins", ["postgres"]),
        // Plugin results.
        query!("policy-results", ["policy"], "PolicyResult"),
        query!("plan-results", ["plan"], "PlanResult"),
        query!("assignment-results", ["plan"], "AssignmentResult"),
        query!("resilience-results", ["resilience"], "ResilienceResult"),
        query!("loss-results", ["resilience"]),
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
        query!("tile-capacity", ["postgres"]),
        query!("tile-demand", ["postgres"]),
        query!("tile-recovery", ["recovery-results"]),
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
        Query {
            name: "grafana-alerts",
            text: "MATCH (a:GrafanaAlert) RETURN a.id AS id, a.status AS status, a.title AS title, a.message AS message",
            inputs: &["grafana-input"],
            label: None,
        },
    ]
}

/// Edges into the native plugins' named input ports.
const PLUGIN_INPUTS: &[(&str, &str, &str)] = &[
    ("policy-workloads", "policy", "subjects"),
    ("policy-deployments", "policy", "targets"),
    ("policy-destinations", "policy", "parameters"),
    ("plan-items", "plan", "items"),
    ("plan-bins", "plan", "bins"),
    ("policy-results", "plan", "eligibility"),
    ("plan-items", "resilience", "items"),
    ("plan-bins", "resilience", "bins"),
    ("policy-results", "resilience", "eligibility"),
];

/// Prometheus reaction aliases (plugins/prometheus/examples/ignite.json) → tile query.
const TILES: &[(&str, &str)] = &[
    ("overdue", "tile-overdue"),
    ("deployments", "tile-deployments"),
    ("capacity", "tile-capacity"),
    ("demand", "tile-demand"),
    ("recovery", "tile-recovery"),
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
];

// Shared by the live source and its bootstrap provider so they bind one slot.
// Database and user are fixed by compose.yaml and ops/observability-env.py;
// port, publication and slot come from ports.env.
fn postgres_settings() -> Result<Value> {
    let settings = ignite_settings::get();
    Ok(json!({
        "host":"127.0.0.1", "port":settings.port("POSTGRES")?, "database":"ignite", "user":"ignite",
        "sslMode":"disable", "publication":settings.get("IGNITE_PG_PUBLICATION")?, "tables":TABLES,
        "slotOwnership":"external", "slotName":settings.get("IGNITE_PG_SLOT")?,
        "maxTransactionChanges":4096, "maxTransactionBytes":8388608,
        "maxTransactionDurationMs":30000, "maxProtocolBytes":8388608,
        "ioTimeoutMs":10000, "feedbackIntervalMs":1000, "reconnectBudgetMs":60000,
    }))
}
fn bootstrap_configuration(source: &ComponentId) -> Result<BTreeMap<Arc<str>, ConfigurationValue>> {
    let mut configuration = BTreeMap::from([
        ("source".into(), literal(source.as_str())?),
        ("stream".into(), literal(stream(PG)?)?),
        ("password".into(), reference("env:IGNITE_POSTGRES_PASSWORD")?),
    ]);
    for (key, value) in postgres_settings()?.as_object().context("settings")? {
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
fn joins(text: &str) -> Result<Vec<Value>> {
    let used: BTreeSet<_> = text
        .split(":")
        .skip(1)
        .filter_map(|rest| rest.split(']').next())
        .filter(|name| name.chars().all(|c| c.is_ascii_uppercase() || c == '_') && !name.is_empty())
        .collect();
    used.into_iter()
        .map(|name| {
            let (_, (l1, p1), (l2, p2)) = JOINS
                .iter()
                .find(|(join, ..)| *join == name)
                .with_context(|| format!("unknown relationship {name}"))?;
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
    let pg = load(directory, "drasi_computation_source_postgres")?;
    let bootstrap = load(directory, "drasi_computation_bootstrap_postgres")?;
    let query_plugin = load(directory, "drasi_computation_transformer_query")?;
    let http_source = load(directory, "drasi_computation_source_http")?;
    let http_sink = load(directory, "drasi_computation_sink_http")?;
    let policy = load(directory, "drasi_computation_ignite_policy")?;
    let optimizer = load(directory, "drasi_computation_ignite_optimizer")?;
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
    let queries = queries();
    let fed_by_postgres: Vec<&str> = queries
        .iter()
        .filter(|q| q.inputs.contains(&PG))
        .map(|q| q.name)
        .collect();

    let plugins = ["policy", "plan", "resilience"];
    let ids: BTreeMap<&str, ComponentId> = [PG, "postgres-bootstrap", "grafana-input", "query-results", "metrics", "desk-stream"]
        .into_iter()
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
            json!({"kind":"sourceProgress","component":ids[name]}),
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
    let mut configuration = postgres_settings()?;
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
        configuration: bootstrap_configuration(&ids[PG])?,
        dependencies: BTreeMap::from([(
            "source_progress".into(),
            fed_by_postgres
                .iter()
                .map(|name| resource(&format!("{name}-progress")))
                .collect::<Result<Vec<_>>>()?,
        )]),
    };
    builder = builder.component(ComponentName::try_new("postgres-bootstrap")?, provider, bootstrap);

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
                "execution":{"joins":joins(query.text)?, "result":result},
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

    // Native plugins: the Rego module and the optimizer's settings are their configuration.
    let mut rego: Value = serde_json::from_str(include_str!("../../../plugins/policy/examples/ignite.json"))?;
    rego["module"] = json!(include_str!("../../../plugins/policy/ignite.rego"));
    let optimizer_config: Value =
        serde_json::from_str(include_str!("../../../plugins/optimizer/examples/ignite.json"))?;
    for (name, plugin, implementation, configuration) in [
        ("policy", &policy, "ignite/rego-policy", rego),
        ("plan", &optimizer, "ignite/assignment-plan", optimizer_config.clone()),
        ("resilience", &optimizer, "ignite/assignment-resilience", optimizer_config),
    ] {
        let factory = find_factory(plugin, implementation)?;
        let mut specification = factory.specification(ids[name].clone(), configuration)?;
        specification
            .dependencies
            .insert("output_snapshots".into(), vec![resource(&format!("{name}-snapshots"))?]);
        builder = builder.component(ComponentName::try_new(name)?, specification, factory);
    }

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

    // HTTP reactions: the gateway configuration API, Grafana annotations, Langfuse scores.
    let workload = settings.url("WORKLOAD")?;
    let grafana = format!("http://127.0.0.1:{}", settings.port("GRAFANA")?);
    let langfuse = format!("http://127.0.0.1:{}", settings.port("LANGFUSE")?);
    let annotation = r#"{"time":{{after.at_ms}},"tags":["ignite-decision","{{after.id}}"],"text":{{json after.text}}}"#;
    let plan_annotation = r#"{"tags":["ignite-decision","{{after.id}}"],"text":{{json after.text}}}"#;
    let reaction_configs: BTreeMap<&str, (Value, Vec<(&str, ConfigurationValue)>)> = BTreeMap::from([
        ("gateway-reaction", (json!({
            "baseUrl":workload, "failurePolicy":"strict", "maxRetries":10, "retryBudgetMs":30000, "timeoutMs":5000,
            "outputTemplates":{"routes":{
                "approve":http_route("POST", "/routes/apply",
                    r#"{"approval":{{json after.approval}},"candidate_id":{{json after.candidate_id}},"basis":{{json after.basis}},"expected_revision":{{json after.expected_revision}},"assignments":{{{after.assignments}}}}"#, false),
                "reject":http_route("POST", "/approvals/{{after.approval}}/reject", r#"{"reason":{{json after.reason}}}"#, false),
                "verified":http_route("POST", "/approvals/{{after.approval}}/verified", r#"{"verified_ms":{{json after.verified_ms}}}"#, false),
            }},
        }), vec![])),
        ("grafana-annotations", (json!({
            "baseUrl":grafana, "failurePolicy":"skip", "maxRetries":3, "timeoutMs":5000,
            "outputTemplates":{"routes":{
                "approval-events":http_route("POST", "/api/annotations", annotation, true),
                "plan-events":http_route("POST", "/api/annotations", plan_annotation, true),
            }},
        }), vec![("token", reference("env:IGNITE_GRAFANA_TOKEN")?)])),
        ("langfuse-scores", (json!({
            "baseUrl":langfuse, "failurePolicy":"skip", "maxRetries":3, "timeoutMs":5000,
            "outputTemplates":{"routes":{
                "scores":http_route("POST", "/api/public/scores",
                    r#"{"id":{{json after.id}},"traceId":{{json after.trace_id}},"name":"drasi_recovery","value":1,"dataType":"NUMERIC","comment":{{json after.comment}}}"#, false),
            }},
        }), vec![("secretHeaders", reference("env-json:IGNITE_LANGFUSE_HEADERS")?)])),
    ]);
    let sink_factory = find_factory(&http_sink, "drasi.network/http-sink")?;
    for (name, aliases) in REACTIONS {
        let (mut configuration, secrets) = reaction_configs[name].clone();
        configuration["successStatusCodes"] = json!([200]);
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

    // Grafana's contact point posts alerts to Drasi's HTTP source.
    let factory = http_source.factories().first().context("HTTP source factory")?.clone();
    builder = builder.component(
        ComponentName::try_new("grafana-input")?,
        factory.specification(
            ids["grafana-input"].clone(),
            json!({
                "stream":stream("grafana-input")?,
                "host":"127.0.0.1", "port":settings.port("DRASI_ALERT")?,
                "webhooks":{"errorBehavior":"reject", "routes":[{
                    "path":"/alerts/grafana","methods":["POST"],
                    "mappings":[{"operation":"update", "elementType":"node", "template":{
                        "id":"{{payload.groupKey}}", "labels":["GrafanaAlert"],
                        "properties":{
                            "id":"{{payload.groupKey}}", "status":"{{payload.status}}",
                            "title":"{{payload.title}}", "message":"{{payload.message}}"
                        }
                    }}]
                }]}
            }),
        )?,
        factory,
    );

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
        for input in query.inputs {
            let edge = EdgeDefinition::new(endpoint(input, "out")?, endpoint(query.name, "in")?);
            builder = builder.connect(edge.clone(), pipe(32));
            if *input != PG && !plugins.contains(input) && *input != "grafana-input" {
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
    for name in [PG, "grafana-input"].into_iter().chain(queries.iter().map(|q| q.name)) {
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
