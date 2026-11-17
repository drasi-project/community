//! `ignite/assignment-plan` and `ignite/assignment-resilience`: native
//! transformers that assign items to bins with `good_lp` (microlp backend).
//! The plan factory emits `Assignment` elements and one `Plan`; the resilience
//! factory removes each healthy bin in turn and emits one `LossScenario` per
//! bin plus a `Resilience` summary.
//!
//! Contract: `plan/contracts.md` (Optimizer plugin).
//!
//! Layout: `config`, `solve` (the `good_lp` model) and `model` (rows in,
//! elements out) are plain Rust with host-free unit tests; `component` is the
//! native transformer (snapshot bootstrap, coalescing, one solver thread);
//! this file is the thin SDK wrapper (factories, plugin export).
use anyhow::{Context, Result};
use drasi_computation_contracts::*;
use drasi_computation_plugin_sdk::{
    Capabilities, Component, ConfigSchema, ConfigType, ControlSender, CreateRequest,
    CreatedComponent, Factory, FactoryMetadata, NativeOutputSnapshots, PluginDefinition,
};
use std::sync::Arc;

pub mod component;
pub mod config;
pub mod model;
pub mod solve;
pub use config::{Config, Constraint, Mode, Objective, PLAN, RESILIENCE};

pub const ITEMS: &str = "items";
pub const BINS: &str = "bins";
pub const ELIGIBILITY: &str = "eligibility";
pub const OUT: &str = "out";

pub struct OptimizerFactory(pub Mode);
impl Factory for OptimizerFactory {
    fn describe(
        &self,
        request: &drasi_computation_plugin_sdk::wire::DescribeRequest,
    ) -> Result<drasi_computation_plugin_sdk::ComponentDescription> {
        Ok(request.volatile_transformer())
    }
    fn metadata(&self) -> FactoryMetadata {
        FactoryMetadata {
            implementation: ImplementationIdentity::try_new(self.0.implementation(), "1").unwrap(),
            role: ComponentRole::Transformer,
            configuration_version: 1,
            configuration: ConfigSchema::from_fields([
                ("items", ConfigType::Object, true),
                ("bins", ConfigType::Object, true),
                ("eligibility", ConfigType::Object, true),
                ("constraints", ConfigType::Array, true),
                ("objective", ConfigType::Array, true),
                ("epochProperty", ConfigType::String, false),
                ("coalesceMs", ConfigType::Integer, false),
                ("timeLimitMs", ConfigType::Integer, false),
            ]),
            ports: [ITEMS, BINS, ELIGIBILITY]
                .into_iter()
                .map(|name| (name, PortDirection::Input))
                .chain(std::iter::once((OUT, PortDirection::Output)))
                .map(|(name, direction)| {
                    PortDescriptor::new(
                        PortId::try_new(name).unwrap(),
                        direction,
                        GraphChangeCodec::schema().descriptor().clone(),
                        PipeRequirements::default(),
                    )
                })
                .collect(),
            completion: None,
            capabilities: Capabilities {
                continuations: false,
                startup_continuation: false,
                wakeups: true,
                input_binding_updates: true,
                ..Capabilities::default()
            },
        }
    }
    fn create(&self, _: &CreateRequest, _: ControlSender) -> Result<CreatedComponent> {
        anyhow::bail!("optimizer requires native upstream snapshots (an output_snapshots dependency)")
    }
    fn supports_output_snapshots(&self) -> bool {
        true
    }
    fn create_with_output_snapshots(
        &self,
        request: &CreateRequest,
        _: ControlSender,
        snapshots: Option<NativeOutputSnapshots>,
    ) -> Result<CreatedComponent> {
        let snapshots = snapshots.context("missing snapshot resource (nativeOutputSnapshots)")?;
        Ok(Component::Transformer(Box::new(self.build(request, Arc::new(snapshots))?)).into())
    }
}
impl OptimizerFactory {
    /// The component, reading upstream snapshots through `snapshots`.
    pub fn build(
        &self,
        request: &CreateRequest,
        snapshots: Arc<dyn component::SnapshotReader>,
    ) -> Result<component::Optimizer> {
        self.metadata().configuration.validate(&request.configuration)?;
        component::Optimizer::new(
            self.0,
            self.metadata().descriptor(request.id.clone())?,
            request,
            snapshots,
        )
    }
}

pub fn plugin() -> Result<PluginDefinition> {
    PluginDefinition::new(
        "ignite-optimizer",
        env!("CARGO_PKG_VERSION"),
        vec![
            Arc::new(OptimizerFactory(Mode::Plan)),
            Arc::new(OptimizerFactory(Mode::Resilience)),
        ],
        vec![GraphChangeCodec::schema()],
    )
}
#[cfg(feature = "dynamic-plugin")]
drasi_computation_plugin_sdk::export_computation_plugin!(plugin());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::SnapshotReader;
    use drasi_computation_plugin_sdk::{lifecycle, wire::Scope, NativeComponent, NativeTransformer};
    use drasi_core_types::{
        evaluation::{context::QueryPartEvaluationContext, variable_value::VariableValue},
        models::SourceChange,
        query::QueryVariables,
    };
    use serde_json::{json, Value};
    use std::{collections::BTreeMap, time::Duration};

    #[test]
    fn example_configuration_is_valid_for_both_factories() {
        let example: serde_json::Value = serde_json::from_str(include_str!("../examples/ignite.json")).unwrap();
        for mode in [Mode::Plan, Mode::Resilience] {
            OptimizerFactory(mode).metadata().configuration.validate(&example).unwrap();
        }
        let config = Config::parse(&example).unwrap();
        assert_eq!(config.objective, [Objective::FewestMoves, Objective::LowestPeakUtilisation]);
        let mut shed = example.clone();
        shed["constraints"] = serde_json::json!(["binCapacity"]);
        assert!(Config::parse(&shed).is_err());
        assert!(plugin().is_ok());
    }

    const INSTANCE: &str = "test-instance";
    const GRAPH: &str = "test-graph";

    /// One upstream query output: its component, stream, projector and rows.
    struct Upstream {
        port: &'static str,
        component: ComponentId,
        stream: StreamId,
        producer: GraphProducerIdentity,
        projector: ResultNodeProjector,
        rows: BTreeMap<u64, Value>,
        sequence: u64,
    }
    impl Upstream {
        fn new(port: &'static str, component: &str, rows: Vec<Value>) -> Self {
            let component = ComponentId::try_new(component).unwrap();
            let stream = StreamId::try_new(format!("{port}-query/out")).unwrap();
            Self {
                port,
                producer: GraphProducerIdentity::volatile(
                    INSTANCE.into(),
                    GRAPH.into(),
                    component.clone(),
                    stream.clone(),
                )
                .unwrap(),
                projector: ResultNodeProjector::try_new(&format!("{port}-query"), Some("Row")).unwrap(),
                component,
                stream,
                rows: rows.into_iter().enumerate().map(|(n, r)| (n as u64 + 1, r)).collect(),
                sequence: 1,
            }
        }
        fn binding(&self) -> InputBinding {
            InputBinding {
                input_port: PortId::try_new(self.port).unwrap(),
                output: OutputEndpoint::new(self.component.clone(), PortId::try_new("out").unwrap()),
                stream: self.stream.clone(),
            }
        }
        fn snapshot(&self) -> (OutputCursor, Vec<Record>) {
            let records = self
                .rows
                .iter()
                .map(|(signature, row)| QueryChangeCodec::encode_row(&self.projector, *signature, &vars(row), 1).unwrap())
                .collect();
            (OutputCursor::try_new(self.producer.clone(), self.sequence).unwrap(), records)
        }
        /// A query output envelope updating the row whose `id` is `id`.
        fn update(&mut self, id: &str, change: impl FnOnce(&mut Value)) -> InputEnvelope {
            let (signature, row) = self.rows.iter_mut().find(|(_, r)| r["id"] == id).unwrap();
            let before = vars(row);
            change(row);
            let results = [QueryPartEvaluationContext::Updating { before, after: vars(row), row_signature: *signature }];
            self.sequence += 1;
            let metadata = QueryOutputMetadata {
                query_id: self.component.clone(),
                input_origin: None,
                timestamp: std::time::SystemTime::now().into(),
                metadata: Default::default(),
                profiling: None,
            };
            let mut envelope = QueryChangeCodec::encode_evaluation(
                None,
                &self.component,
                SystemMetadata::new(self.stream.clone(), self.sequence),
                &self.projector,
                &results,
                metadata,
            )
            .unwrap()
            .unwrap();
            GraphProducerProgress::annotate(&mut envelope, &self.producer, self.sequence).unwrap();
            InputEnvelope { port: PortId::try_new(self.port).unwrap(), envelope }
        }
    }
    fn vars(row: &Value) -> QueryVariables {
        row.as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.as_str().into(), VariableValue::from(v.clone())))
            .collect()
    }
    struct Snapshots(BTreeMap<String, (OutputCursor, Vec<Record>)>);
    #[async_trait::async_trait]
    impl SnapshotReader for Snapshots {
        async fn read(&self, endpoint: &OutputEndpoint) -> Result<(OutputCursor, Vec<Record>)> {
            self.0.get(&endpoint.component.to_string()).cloned().context("unknown output")
        }
    }

    fn upstreams() -> Vec<Upstream> {
        let seed = model::tests::seed();
        vec![
            Upstream::new(ITEMS, "00000000-0000-0000-0000-0000000000a1", seed.items),
            Upstream::new(BINS, "00000000-0000-0000-0000-0000000000a2", seed.bins),
            Upstream::new(ELIGIBILITY, "00000000-0000-0000-0000-0000000000a3", seed.eligibility),
        ]
    }

    /// (change kind, element id, label, properties) for every change in `outputs`.
    fn changes(outputs: &[OutputEnvelope]) -> Vec<(&'static str, String, String, Value)> {
        let mut out = vec![];
        for output in outputs {
            assert_eq!(output.port.as_str(), OUT);
            for change in GraphChangeCodec::decode_changes(&output.envelope).unwrap() {
                let (kind, metadata, properties) = match change {
                    SourceChange::Insert { element } => ("insert", element.get_metadata().clone(), Some(element)),
                    SourceChange::Replace { element } => ("replace", element.get_metadata().clone(), Some(element)),
                    SourceChange::Delete { metadata } => ("delete", metadata, None),
                    other => panic!("unexpected change {other:?}"),
                };
                let properties = properties
                    .map(|e| Value::Object(serde_json::Map::from(e.get_properties())))
                    .unwrap_or(Value::Null);
                out.push((kind, metadata.reference.element_id.to_string(), metadata.labels[0].to_string(), properties));
            }
        }
        out
    }

    /// Wait for the component's wakeup source and handle wakeups until it emits.
    async fn next_output(component: &mut component::Optimizer) -> Vec<OutputEnvelope> {
        let wake = component.wakeup_source().unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                wake.wait().await.unwrap();
                let outputs = component.on_wakeup().await.unwrap();
                if !outputs.is_empty() {
                    return outputs;
                }
            }
        })
        .await
        .expect("no output within 10 s")
    }

    async fn start(mode: Mode, upstreams: &[Upstream]) -> component::Optimizer {
        let factory = OptimizerFactory(mode);
        let metadata = factory.metadata();
        let ports: Vec<_> = metadata.ports.iter().map(|p| p.id().as_str().to_owned()).collect();
        assert_eq!(ports, [ITEMS, BINS, ELIGIBILITY, OUT]);
        let request = CreateRequest {
            id: ComponentId::try_new(match mode {
                Mode::Plan => "00000000-0000-0000-0000-0000000000b1",
                Mode::Resilience => "00000000-0000-0000-0000-0000000000b2",
            })
            .unwrap(),
            implementation: metadata.implementation.clone(),
            configuration_version: metadata.configuration_version,
            configuration: serde_json::from_str(include_str!("../examples/ignite.json")).unwrap(),
            scope: Some(Scope { instance_id: INSTANCE.into(), graph_id: GRAPH.into(), generation: 1 }),
            inputs: vec![],
            activation_credentials_bound: false,
            update_directive: None,
        };
        // Self-description: a volatile transformer, rebuilt from upstream snapshots.
        let description = factory
            .describe(&drasi_computation_plugin_sdk::wire::DescribeRequest {
                request: request.clone(),
                descriptor: metadata.descriptor(request.id.clone()).unwrap(),
                dependencies: Default::default(),
            })
            .unwrap();
        assert_eq!(description.component.progress, ProgressPersistence::Volatile);
        // The host must supply upstream snapshots.
        assert!(factory.create_with_output_snapshots(&request, ControlSender::default(), None).is_err());

        let snapshots = Snapshots(upstreams.iter().map(|u| (u.component.to_string(), u.snapshot())).collect());
        let mut component = factory.build(&request, Arc::new(snapshots)).unwrap();
        assert_eq!(component.descriptor().id(), &request.id);
        let bindings: Vec<_> = upstreams.iter().map(Upstream::binding).collect();
        component.bind_inputs(&bindings).await.unwrap();
        component.initialize(&lifecycle::InitializeContext::new(scope("initialize"))).await.unwrap();
        component.start(&mut lifecycle::StartContext::new(scope("start"), None)).await.unwrap();
        component
    }
    fn scope(operation: &str) -> lifecycle::OperationScope {
        lifecycle::OperationScope::new(
            format!("{operation}-1"),
            "test",
            1,
            Some(tokio::time::Instant::now() + Duration::from_secs(10)),
            lifecycle::CancellationToken::new(),
        )
    }
    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap()
    }
    fn find<'a>(changes: &'a [(&'static str, String, String, Value)], id: &str) -> &'a (&'static str, String, String, Value) {
        changes.iter().find(|c| c.1 == id).unwrap_or_else(|| panic!("no change for {id}: {changes:?}"))
    }

    #[test]
    fn plan_factory_bootstraps_from_snapshots_and_replans_on_coalesced_changes() {
        runtime().block_on(async {
            let mut upstreams = upstreams();
            let mut component = start(Mode::Plan, &upstreams).await;

            let first = changes(&next_output(&mut component).await);
            let plan = find(&first, "plan");
            assert_eq!((plan.0, plan.2.as_str()), ("insert", "Plan"));
            assert_eq!(plan.3["state"], "feasible");
            assert_eq!(plan.3["moves"], 0);
            assert_eq!(plan.3["revision"], 1);
            assert_eq!(find(&first, "support").2, "Assignment");
            assert_eq!(first.len(), 3);

            // EU-A fails, then its capacity row changes again within the window:
            // both are solved together and emitted once.
            let input = upstreams[1].update("EU-A", |r| r["status"] = json!("failed"));
            assert!(component.transform(input).await.unwrap().is_empty());
            let input = upstreams[1].update("EU-A", |r| r["capacity_tpm"] = json!(90_000));
            assert!(component.transform(input).await.unwrap().is_empty());
            let outputs = next_output(&mut component).await;
            assert_eq!(outputs.len(), 1, "one coalesced emission");
            let second = changes(&outputs);
            let plan = find(&second, "plan");
            assert_eq!(plan.0, "replace");
            assert_eq!(plan.3["moves"], 2);
            assert_eq!(plan.3["assignments"], r#"{"batch":"US-C","support":"EU-B"}"#);
            assert_eq!(find(&second, "support").3["bin"], "EU-B");
            assert_eq!(find(&second, "batch").3["bin"], "US-C");

            // EU-B's capacity changes and its solve starts; the policy edit arrives
            // before the result is collected, so that result is discarded and only
            // the answer for the latest inputs is emitted: no plan, Assignments deleted.
            let wake = component.wakeup_source().unwrap();
            let input = upstreams[1].update("EU-B", |r| r["capacity_tpm"] = json!(95_000));
            component.transform(input).await.unwrap();
            wake.wait().await.unwrap();
            assert!(component.on_wakeup().await.unwrap().is_empty(), "solve launched, nothing yet");
            let edit = upstreams[2].update("support/EU-B", |r| {
                r["state"] = json!("denied");
                r["allowed"] = json!(false);
            });
            component.transform(edit.clone()).await.unwrap();
            let third = changes(&next_output(&mut component).await);
            assert_eq!(find(&third, "plan").3["state"], "infeasible");
            assert_eq!(find(&third, "support").0, "delete");
            assert_eq!(find(&third, "batch").0, "delete");
            assert!(!wake.has_pending().await.unwrap(), "answered");

            // A replayed (already applied) position changes nothing.
            assert!(component.transform(edit).await.unwrap().is_empty());
            assert!(!wake.has_pending().await.unwrap(), "replay ignored");
            component.stop(&lifecycle::StopContext::new(scope("stop"), lifecycle::StopMode::Drain)).await.unwrap();
        });
    }

    #[test]
    fn resilience_factory_emits_loss_scenarios_and_summary() {
        runtime().block_on(async {
            let mut upstreams = upstreams();
            // After the approved recovery: support on EU-B, summaries on US-C, EU-A back at 50k.
            for (id, bin) in [("support", "EU-B"), ("batch", "US-C")] {
                upstreams[0].rows.values_mut().find(|r| r["id"] == id).unwrap()["deployment"] = json!(bin);
            }
            upstreams[1].rows.values_mut().find(|r| r["id"] == "EU-A").unwrap()["capacity_tpm"] = json!(50_000);
            let mut component = start(Mode::Resilience, &upstreams).await;

            let first = changes(&next_output(&mut component).await);
            let summary = find(&first, "resilience");
            assert_eq!(summary.2, "Resilience");
            assert_eq!(summary.3["state"], "at_risk");
            assert_eq!(summary.3["first_failure"], "EU-B");
            assert_eq!(find(&first, "EU-B").2, "LossScenario");
            assert_eq!(find(&first, "EU-B").3["state"], "infeasible");
            assert_eq!(find(&first, "EU-A").3["state"], "feasible");

            let input = upstreams[1].update("EU-A", |r| r["capacity_tpm"] = json!(100_000));
            component.transform(input).await.unwrap();
            let second = changes(&next_output(&mut component).await);
            assert_eq!(find(&second, "resilience").3["state"], "safe");
            assert_eq!(find(&second, "resilience").3["first_failure"], "");

            // A report expires: unknown, and the loss scenarios are withdrawn.
            let input = upstreams[1].update("EU-B", |r| r["status"] = json!("unknown"));
            component.transform(input).await.unwrap();
            let third = changes(&next_output(&mut component).await);
            assert_eq!(find(&third, "resilience").3["state"], "unknown");
            assert_eq!(third.iter().filter(|c| c.0 == "delete").count(), 3);
        });
    }
}
