use anyhow::{Context, Result};
use drasi_host_sdk::computation::{NativeBootstrapNodeFactory, NativePlugin};
use drasi_lib::computation::v1::*;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path, sync::Arc};

fn resource(value: &str) -> Result<ResourceId> {
    Ok(ResourceId::try_new(value)?)
}
fn stream(value: &str) -> Result<StreamId> {
    Ok(StreamId::try_new(format!("{value}/out"))?)
}
fn literal(value: impl serde::Serialize) -> Result<ConfigurationValue> {
    Ok(ConfigurationValue::Literal(serde_json::to_value(value)?))
}
const TABLES: [&str; 8] = [
    "ignite_endpoints",
    "ignite_model",
    "ignite_tasks",
    "ignite_attempts",
    "ignite_reports",
    "ignite_breaches",
    "ignite_alerts",
    "ignite_commands",
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
fn password() -> Result<ConfigurationValue> {
    Ok(ConfigurationValue::Reference {
        resource: resource("configuration")?,
        key: "env:IGNITE_POSTGRES_PASSWORD".into(),
        secret: true,
    })
}
fn bootstrap_configuration(source: &ComponentId) -> Result<BTreeMap<Arc<str>, ConfigurationValue>> {
    let mut configuration = BTreeMap::from([
        ("source".into(), literal(source.as_str())?),
        ("stream".into(), literal(stream("postgres")?)?),
        ("password".into(), password()?),
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

fn main() -> Result<()> {
    let directory = std::env::args()
        .nth(1)
        .context("usage: ignite-config <plugin-directory>")?;
    let directory = Path::new(&directory);
    let pg = load(directory, "drasi_computation_source_postgres")?;
    let bootstrap = load(directory, "drasi_computation_bootstrap_postgres")?;
    let queries = load(directory, "drasi_computation_transformer_query")?;
    let capacity = load(directory, "drasi_computation_ignite_capacity")?;
    let decisions = load(directory, "drasi_computation_ignite_decisions")?;
    let http = load(directory, "drasi_computation_source_http")?;
    let source_factory = pg
        .factories()
        .iter()
        .find(|f| f.metadata().implementation.name.as_ref() == "drasi.postgres/shared-changes")
        .context("PostgreSQL shared-row factory")?
        .clone();
    let bootstrap = bootstrap
        .bootstrap_factories()
        .iter()
        .find(|f| {
            f.metadata().implementation.name.as_ref() == "drasi.postgres/shared-row-bootstrap"
        })
        .cloned()
        .context("PostgreSQL shared-row bootstrap")?;
    let bootstrap = Arc::new(NativeBootstrapNodeFactory::new(bootstrap));
    let query_factory = queries.query_factory()?;
    let context_queries = [
        (
            "decision-context",
            include_str!("../../../drasi/decision-context.cypher"),
        ),
        (
            "recovery-task-context",
            include_str!("../../../drasi/recovery-task-context.cypher"),
        ),
        (
            "recovery-observation-context",
            include_str!("../../../drasi/recovery-observation-context.cypher"),
        ),
        (
            "model-context",
            include_str!("../../../drasi/model-context.cypher"),
        ),
        (
            "task-context",
            include_str!("../../../drasi/task-context.cypher"),
        ),
        (
            "overdue-tasks",
            include_str!("../../../drasi/overdue-tasks.cypher"),
        ),
        (
            "deadline-history",
            include_str!("../../../drasi/deadline-history.cypher"),
        ),
        (
            "endpoint-freshness",
            include_str!("../../../drasi/endpoint-freshness.cypher"),
        ),
        (
            "attempt-context",
            include_str!("../../../drasi/attempt-context.cypher"),
        ),
        (
            "breach-history",
            include_str!("../../../drasi/breach-history.cypher"),
        ),
        (
            "alert-context",
            include_str!("../../../drasi/alert-context.cypher"),
        ),
    ];
    let ids: BTreeMap<_, _> = [
        "postgres",
        "postgres-bootstrap",
        "endpoint-context",
        "capacity",
        "capacity-results",
        "grafana-input",
        "grafana-alerts",
        "query-results",
        "policy",
        "allocation",
        "resilience",
        "policy-results",
        "allocation-results",
        "resilience-results",
        "recovery-results",
    ]
    .into_iter()
    .chain(context_queries.iter().map(|(name, _)| *name))
    .map(|name| (name, ComponentId::generate()))
    .collect();
    let endpoint = |name: &str, port: &str| -> Result<Endpoint> {
        Ok(Endpoint::new(ids[name].clone(), PortId::try_new(port)?))
    };
    let mut builder = ComponentBatch::builder();
    let mut consumers = serde_json::Map::new();
    consumers.insert(
        ids["endpoint-context"].as_str().into(),
        json!("context-progress"),
    );
    for (name, _) in &context_queries {
        consumers.insert(ids[name].as_str().into(), json!(format!("{name}-progress")));
    }
    for (name, role, recipe) in [
        (
            "configuration",
            ResourceRole::SecretStore,
            json!({"kind":"configuration"}),
        ),
        (
            "indexes",
            ResourceRole::IndexBackend,
            json!({"kind":"memoryIndexes"}),
        ),
        (
            "catalog",
            ResourceRole::QueryCatalog,
            json!({"kind":"queryCatalog"}),
        ),
        (
            "context-progress",
            ResourceRole::Checkpoint,
            json!({"kind":"sourceProgress","component":ids["endpoint-context"]}),
        ),
        (
            "context-snapshot",
            ResourceRole::OutputCatalogue,
            json!({"kind":"nativeOutputSnapshots",
                "outputs":[{"component":ids["endpoint-context"],"port":"out"}],
                "timeoutMs":10000,"maxReaders":4}),
        ),
        (
            "pg-consumers",
            ResourceRole::Checkpoint,
            json!({
                "kind":"consumerProgress","producer":ids["postgres"],
                "consumers":consumers
            }),
        ),
    ] {
        builder = declare(builder, name, role, recipe)?;
    }
    for (name, upstream) in [
        ("policy", "decision-context"),
        ("allocation", "policy-results"),
        ("resilience", "policy-results"),
    ] {
        builder = declare(
            builder,
            &format!("{name}-snapshot"),
            ResourceRole::OutputCatalogue,
            json!({"kind":"nativeOutputSnapshots","outputs":[{"component":ids[upstream],"port":"out"}],
                "timeoutMs":10000,"maxReaders":4}),
        )?;
    }
    builder = builder.resource_dependency(
        resource("pg-consumers")?,
        resource("context-progress")?,
        ResourceRole::Checkpoint,
    )?;
    for (name, _) in &context_queries {
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
    let mut configuration = postgres_settings()?;
    configuration["stream"] = json!(stream("postgres")?);
    configuration["persistent"] = json!(false);
    let mut source = source_factory.specification(ids["postgres"].clone(), configuration)?;
    source.configuration.insert("password".into(), password()?);
    source
        .dependencies
        .insert("consumer_progress".into(), vec![resource("pg-consumers")?]);
    builder = builder.component(ComponentName::try_new("postgres")?, source, source_factory);
    // The source's visible bootstrap provider. The source relays each query's
    // bootstrap session to it; the provider records progress for every query.
    let progress = std::iter::once("context-progress".to_string())
        .chain(context_queries.iter().map(|(name, _)| format!("{name}-progress")))
        .map(|name| resource(&name))
        .collect::<Result<Vec<_>>>()?;
    let provider = ComponentSpecification {
        descriptor: NativeBootstrapNodeFactory::component_descriptor(ids["postgres-bootstrap"].clone())?,
        role: ComponentRole::BootstrapProvider,
        completion: None,
        implementation: bootstrap.descriptor().implementation.clone(),
        configuration_version: bootstrap.descriptor().configuration_version,
        configuration: bootstrap_configuration(&ids["postgres"])?,
        dependencies: BTreeMap::from([("source_progress".into(), progress)]),
    };
    builder = builder.component(ComponentName::try_new("postgres-bootstrap")?, provider, bootstrap);

    for (name, query) in [
        ("endpoint-context", include_str!("../../../drasi/endpoint-context.cypher")),
        ("capacity-results", include_str!("../../../drasi/capacity-results.cypher")),
        ("policy-results", include_str!("../../../drasi/decision-results.cypher")),
        ("allocation-results", include_str!("../../../drasi/decision-results.cypher")),
        ("resilience-results", include_str!("../../../drasi/decision-results.cypher")),
        ("grafana-alerts", "MATCH (a:GrafanaAlert) RETURN a.id AS id, a.status AS status, a.title AS title, a.message AS message"),
    ].into_iter().chain(context_queries) {
        let epoch_joins: &[(&str, &str, &str)] = match name {
            "task-context" | "overdue-tasks" | "recovery-task-context" => &[("TASK_MODEL", "ignite_tasks", "ignite_model")],
            "attempt-context" => &[("ATTEMPT_MODEL", "ignite_attempts", "ignite_model")],
            "decision-context" | "endpoint-freshness" => &[("REPORT_MODEL", "ignite_reports", "ignite_model")],
            "recovery-observation-context" => &[
                ("REPORT_MODEL", "ignite_reports", "ignite_model"),
                ("MODEL_COMMAND", "ignite_model", "ignite_commands"),
            ],
            _ => &[],
        };
        let joins: Vec<_> = epoch_joins.iter().map(|(id, left, right)| json!({
            "id":id, "keys":[{"label":left,"property":"run_epoch"},{"label":right,"property":"run_epoch"}]
        })).collect();
        let result=match name {
            "recovery-task-context"=>json!({"label":"RecoveryTasks"}),
            "recovery-observation-context"=>json!({"label":"RecoveryObservation"}),
            _=>json!({}),
        };
        let mut specification = query_factory.specification_from_config(ids[name].clone(), json!({
            "query":query, "stream":stream(name)?, "outbox_capacity":128, "execution":{"joins":joins,"result":result},
        }))?;
        specification.dependencies = BTreeMap::from([
            ("indexes".into(), vec![resource("indexes")?]),
            ("catalog".into(), vec![resource("catalog")?]),
        ]);
        if name == "endpoint-context" {
            specification.dependencies.insert("source_progress".into(), vec![resource("context-progress")?]);
        }
        if context_queries.iter().any(|(context, _)| *context == name) {
            specification.dependencies.insert("source_progress".into(),vec![resource(&format!("{name}-progress"))?]);
        }
        builder = builder.component(ComponentName::try_new(name)?, specification, query_factory.clone());
    }
    let factory = capacity
        .factories()
        .first()
        .context("capacity factory")?
        .clone();
    let mut specification = factory.specification(ids["capacity"].clone(), json!({}))?;
    specification.dependencies.insert(
        "output_snapshots".into(),
        vec![resource("context-snapshot")?],
    );
    builder = builder.component(ComponentName::try_new("capacity")?, specification, factory);
    for name in ["policy", "allocation", "resilience"] {
        let factory = decisions
            .factories()
            .iter()
            .find(|f| f.metadata().implementation.name.as_ref() == format!("ignite/{name}"))
            .context("decision factory")?
            .clone();
        let mut specification = factory.specification(ids[name].clone(), json!({}))?;
        specification.dependencies.insert(
            "output_snapshots".into(),
            vec![resource(&format!("{name}-snapshot"))?],
        );
        builder = builder.component(ComponentName::try_new(name)?, specification, factory);
    }
    // Temporary: an ordinary query fed by both upstream outputs. The intended
    // snapshot-bootstrapped volatile query cannot bind a result catalog yet.
    let mut specification = query_factory.specification_from_config(
        ids["recovery-results"].clone(),
        json!({
            "query":include_str!("../../../drasi/recovery-results.cypher"),
            "stream":stream("recovery-results")?, "outbox_capacity":128,
            "execution":{"joins":[{"id":"RECOVERY_SUMMARY","keys":[
                {"label":"RecoveryObservation","property":"run_epoch"},
                {"label":"RecoveryTasks","property":"run_epoch"}]}]},
        }),
    )?;
    specification.dependencies = BTreeMap::from([
        ("indexes".into(), vec![resource("indexes")?]),
        ("catalog".into(), vec![resource("catalog")?]),
    ]);
    builder = builder.component(
        ComponentName::try_new("recovery-results")?,
        specification,
        query_factory.clone(),
    );
    let factory = http
        .factories()
        .first()
        .context("HTTP source factory")?
        .clone();
    builder = builder.component(
        ComponentName::try_new("grafana-input")?,
        factory.specification(
            ids["grafana-input"].clone(),
            json!({
                "stream":stream("grafana-input")?,
                "host":"127.0.0.1", "port":ignite_settings::get().port("DRASI_ALERT")?,
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
    let factory = Arc::new(QueryResultsOutletFactory::default());
    let mut outlet = factory.specification_from_config(ids["query-results"].clone(), json!({}))?;
    outlet
        .dependencies
        .insert("catalog".into(), vec![resource("catalog")?]);
    builder = builder.component(ComponentName::try_new("query-results")?, outlet, factory);
    for (from, to) in [
        ("postgres", "endpoint-context"),
        ("endpoint-context", "capacity"),
        ("capacity", "capacity-results"),
        ("grafana-input", "grafana-alerts"),
        ("endpoint-context", "query-results"),
        ("capacity-results", "query-results"),
        ("grafana-alerts", "query-results"),
        ("decision-context", "policy"),
        ("policy", "policy-results"),
        ("policy-results", "allocation"),
        ("policy-results", "resilience"),
        ("allocation", "allocation-results"),
        ("resilience", "resilience-results"),
        ("policy-results", "query-results"),
        ("allocation-results", "query-results"),
        ("resilience-results", "query-results"),
        ("recovery-task-context", "recovery-results"),
        ("recovery-observation-context", "recovery-results"),
        ("recovery-results", "query-results"),
    ] {
        builder = builder.connect(
            EdgeDefinition::new(endpoint(from, "out")?, endpoint(to, "in")?),
            Box::new(BoundedPipeConfig { capacity: 32 }),
        );
    }
    // Queries reading PostgreSQL bootstrap through the source, which relays to its provider.
    for (from, to) in std::iter::once(("postgres-bootstrap", "postgres"))
        .chain(std::iter::once(("postgres", "endpoint-context")))
        .chain(context_queries.iter().map(|(name, _)| ("postgres", *name)))
    {
        builder = builder.connect(
            EdgeDefinition::new(endpoint(from, "snapshot")?, endpoint(to, "bootstrap")?),
            Box::new(BoundedPipeConfig { capacity: 8 }),
        );
    }
    for name in ids
        .keys()
        .filter(|name| !matches!(**name, "query-results" | "postgres-bootstrap"))
    {
        builder = builder.bind_stream(endpoint(name, "out")?, stream(name)?);
    }
    for (name, _) in context_queries {
        for (from, to) in [("postgres", name), (name, "query-results")] {
            builder = builder.connect(
                EdgeDefinition::new(endpoint(from, "out")?, endpoint(to, "in")?),
                Box::new(BoundedPipeConfig { capacity: 32 }),
            );
        }
    }
    builder = builder.relationship_policy(
        EdgeDefinition::new(
            endpoint("endpoint-context", "out")?,
            endpoint("capacity", "in")?,
        ),
        RelationshipPolicy {
            activation: ActivationCoupling::RequiresRunning,
            ..RelationshipPolicy::default()
        },
    );
    for (from, to) in [
        ("decision-context", "policy"),
        ("policy-results", "allocation"),
        ("policy-results", "resilience"),
        ("recovery-task-context", "recovery-results"),
        ("recovery-observation-context", "recovery-results"),
    ] {
        builder = builder.relationship_policy(
            EdgeDefinition::new(endpoint(from, "out")?, endpoint(to, "in")?),
            RelationshipPolicy {
                activation: ActivationCoupling::RequiresRunning,
                ..RelationshipPolicy::default()
            },
        );
    }
    let graph = builder.build()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "apiVersion":"drasi.io/v1","host":"127.0.0.1","port":ignite_settings::get().port("DRASI_API")?,
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
