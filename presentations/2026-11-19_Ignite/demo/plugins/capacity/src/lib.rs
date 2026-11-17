use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use drasi_computation_contracts::*;
use drasi_computation_plugin_sdk::lifecycle::{
    nothing_to_do, DisposeContext, InitializeContext, LifecycleResult, PauseContext, ResumeContext,
    StartContext, StopContext,
};
use drasi_computation_plugin_sdk::{
    Capabilities, Component, ConfigSchema, ControlSender, CreateRequest, CreatedComponent, Factory,
    FactoryMetadata, NativeComponent, NativeOutputSnapshots, NativeTransformer, PluginDefinition,
};
use drasi_core_types::models::{
    Element, ElementMetadata, ElementPropertyMap, ElementReference, SourceChange,
};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc};

pub struct CapacityFactory;

impl Factory for CapacityFactory {
    /// Stateless: outputs carry a new volatile producer identity on every construction.
    fn describe(
        &self,
        request: &drasi_computation_plugin_sdk::wire::DescribeRequest,
    ) -> anyhow::Result<drasi_computation_plugin_sdk::ComponentDescription> {
        Ok(request.volatile_transformer())
    }
    fn metadata(&self) -> FactoryMetadata {
        FactoryMetadata {
            implementation: ImplementationIdentity::try_new("ignite/capacity", "1").unwrap(),
            role: ComponentRole::Transformer,
            configuration_version: 1,
            configuration: ConfigSchema::default(),
            ports: [("in", PortDirection::Input), ("out", PortDirection::Output)]
                .into_iter()
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
                continuations: true,
                startup_continuation: true,
                input_binding_updates: true,
                ..Capabilities::default()
            },
        }
    }

    fn create(&self, _: &CreateRequest, _: ControlSender) -> Result<CreatedComponent> {
        anyhow::bail!("capacity requires its context output-snapshot resource")
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
        self.metadata()
            .configuration
            .validate(&request.configuration)?;
        let scope = request
            .scope
            .as_ref()
            .context("capacity requires hosted scope")?;
        let stream = StreamId::try_new("capacity/out")?;
        Ok(Component::Transformer(Box::new(Capacity {
            descriptor: self.metadata().descriptor(request.id.clone())?,
            producer: GraphProducerIdentity::volatile(
                scope.instance_id.clone().into(),
                scope.graph_id.clone().into(),
                request.id.clone(),
                stream.clone(),
            )?,
            stream,
            rows: BTreeMap::new(),
            sequence: 0,
            input_cursor: None,
            input: None,
            snapshots: snapshots.context("missing context snapshot resource")?,
            pending: vec![],
        }))
        .into())
    }
}

pub fn plugin() -> Result<PluginDefinition> {
    PluginDefinition::new(
        "ignite-capacity",
        env!("CARGO_PKG_VERSION"),
        vec![Arc::new(CapacityFactory)],
        vec![GraphChangeCodec::schema()],
    )
}

#[cfg(feature = "dynamic-plugin")]
drasi_computation_plugin_sdk::export_computation_plugin!(plugin());

struct Capacity {
    descriptor: ComponentDescriptor,
    producer: GraphProducerIdentity,
    stream: StreamId,
    rows: BTreeMap<String, Value>,
    sequence: u64,
    input_cursor: Option<OutputCursor>,
    input: Option<InputBinding>,
    snapshots: NativeOutputSnapshots,
    pending: Vec<OutputEnvelope>,
}

#[async_trait]
impl NativeComponent for Capacity {
    fn descriptor(&self) -> &ComponentDescriptor {
        &self.descriptor
    }
    fn configuration(&self) -> Result<Value> {
        Ok(json!({}))
    }

    fn supports_input_binding_updates(&self) -> bool {
        true
    }

    async fn bind_inputs(&mut self, inputs: &[InputBinding]) -> Result<()> {
        ensure!(inputs.len() <= 1, "capacity supports one context input");
        if self.input_cursor.is_some() {
            ensure!(
                inputs.first() == self.input.as_ref(),
                "context rebind requires reconstruction"
            );
        }
        self.input = inputs.first().cloned();
        Ok(())
    }

    async fn initialize(&mut self, _ctx: &InitializeContext) -> LifecycleResult {
        nothing_to_do()
    }

    // Reads the upstream snapshot once; later starts resume from the retained cursor.
    async fn start(&mut self, _ctx: &mut StartContext) -> LifecycleResult {
        if self.input_cursor.is_some() {
            return Ok(());
        }
        let input = self
            .input
            .as_ref()
            .context("capacity requires a context input")?;
        let mut snapshot = self
            .snapshots
            .snapshot(&input.output, &GraphChangeCodec::schema())
            .await?;
        if snapshot.cursor.producer().stream() != &input.stream {
            return Err(anyhow::anyhow!("context snapshot stream mismatch").into());
        }
        let mut next = BTreeMap::new();
        while let Some(record) = snapshot.records.next().await {
            let endpoint = decode_endpoint(&record?)?;
            next.insert(endpoint.id.clone(), ignite_domain::assess(&endpoint)?);
        }
        self.pending = self.replace(next, None)?;
        self.input_cursor = Some(snapshot.cursor);
        Ok(())
    }

    // Transformation is driven by the host; there are no independent workers.
    async fn pause(&mut self, _ctx: &PauseContext) -> LifecycleResult {
        nothing_to_do()
    }

    async fn resume(&mut self, _ctx: &ResumeContext) -> LifecycleResult {
        nothing_to_do()
    }

    async fn stop(&mut self, _ctx: &StopContext) -> LifecycleResult {
        Ok(())
    }

    // State is volatile and rebuilt from the upstream snapshot by a new instance.
    async fn dispose(&mut self, _ctx: &DisposeContext) -> LifecycleResult {
        nothing_to_do()
    }
}

#[async_trait]
impl NativeTransformer for Capacity {
    fn has_pending_emissions(&self) -> bool {
        !self.pending.is_empty()
    }

    async fn continue_transform(&mut self) -> Result<Vec<OutputEnvelope>> {
        Ok(std::mem::take(&mut self.pending))
    }

    async fn transform(&mut self, input: InputEnvelope) -> Result<Vec<OutputEnvelope>> {
        ensure!(
            input.port.as_str() == "in",
            "unexpected capacity input port"
        );
        let progress = GraphProducerProgress::from_envelope(&input.envelope)?
            .context("context query must provide producer progress")?;
        let cursor = OutputCursor::try_new(progress.identity().clone(), progress.sequence())?;
        let saved = self
            .input_cursor
            .as_ref()
            .context("capacity has not initialized")?;
        saved.ensure_same_epoch(&cursor)?;
        if cursor.sequence() <= saved.sequence() {
            return Ok(vec![]);
        }
        let mut next = self.rows.clone();
        for operation in input.envelope.changes().operations() {
            match operation {
                ChangeOperation::Added { after, .. } | ChangeOperation::Updated { after, .. } => {
                    let endpoint = decode_endpoint(after)?;
                    next.insert(endpoint.id.clone(), ignite_domain::assess(&endpoint)?);
                }
                ChangeOperation::Deleted { before, .. } => {
                    let before = before
                        .as_ref()
                        .context("context delete requires before image")?;
                    let row = QueryChangeCodec::decode_row(before)?;
                    let value = QueryChangeCodec::row_values_to_json(&row.values);
                    next.remove(value["id"].as_str().context("deleted endpoint ID")?);
                }
            }
        }
        let output = self.replace(next, Some(&input.envelope))?;
        self.input_cursor = Some(cursor);
        Ok(output)
    }
}

fn decode_endpoint(record: &Record) -> Result<ignite_domain::Endpoint> {
    let row = QueryChangeCodec::decode_row(record)?;
    serde_json::from_value(QueryChangeCodec::row_values_to_json(&row.values))
        .context("invalid endpoint context row")
}

impl Capacity {
    fn replace(
        &mut self,
        next: BTreeMap<String, Value>,
        input: Option<&ChangeEnvelope>,
    ) -> Result<Vec<OutputEnvelope>> {
        let now = chrono::Utc::now();
        let metadata = |id: &str| ElementMetadata {
            reference: ElementReference::new(self.stream.as_str(), id),
            labels: Arc::from([Arc::from("CapacityAssessment")]),
            effective_from: now.timestamp_millis() as u64,
        };
        let mut changes = vec![];
        for id in self.rows.keys().filter(|id| !next.contains_key(*id)) {
            changes.push(SourceChange::Delete {
                metadata: metadata(id),
            });
        }
        for (id, value) in &next {
            if self.rows.get(id) == Some(value) {
                continue;
            }
            let element = Element::Node {
                metadata: metadata(id),
                properties: ElementPropertyMap::from(value.clone()),
            };
            changes.push(if self.rows.contains_key(id) {
                SourceChange::Replace { element }
            } else {
                SourceChange::Insert { element }
            });
        }
        if changes.is_empty() {
            return Ok(vec![]);
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .context("capacity sequence exhausted")?;
        let mut envelope = if let Some(input) = input {
            GraphChangeCodec::derive_changes(input, &changes, self.stream.clone(), self.sequence)?
        } else {
            GraphChangeCodec::encode_changes(
                &changes,
                self.stream.clone(),
                self.sequence,
                Some(now),
            )?
        };
        GraphProducerProgress::annotate(&mut envelope, &self.producer, self.sequence)?;
        self.rows = next;
        Ok(vec![OutputEnvelope {
            port: PortId::try_new("out")?,
            envelope,
        }])
    }
}
