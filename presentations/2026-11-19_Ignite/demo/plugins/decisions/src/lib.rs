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
use ignite_domain::decision::{allocate, policy};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc};

struct DecisionFactory(&'static str);
impl Factory for DecisionFactory {
    fn metadata(&self) -> FactoryMetadata {
        FactoryMetadata {
            implementation: ImplementationIdentity::try_new(format!("ignite/{}", self.0), "1")
                .unwrap(),
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
        anyhow::bail!("decision transformer requires native upstream snapshots")
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
        let scope = request.scope.as_ref().context("hosted scope required")?;
        let stream = StreamId::try_new(format!("{}/out", self.0))?;
        Ok(Component::Transformer(Box::new(Transformer {
            kind: self.0,
            descriptor: self.metadata().descriptor(request.id.clone())?,
            producer: GraphProducerIdentity::volatile(
                scope.instance_id.clone(),
                scope.graph_id.clone(),
                request.id.clone(),
                stream.clone(),
            )?,
            stream,
            rows: BTreeMap::new(),
            output: None,
            sequence: 0,
            cursor: None,
            input: None,
            snapshots: snapshots.context("missing snapshot resource")?,
            pending: vec![],
        }))
        .into())
    }
}
pub fn plugin() -> Result<PluginDefinition> {
    PluginDefinition::new(
        "ignite-decisions",
        env!("CARGO_PKG_VERSION"),
        vec![
            Arc::new(DecisionFactory("policy")),
            Arc::new(DecisionFactory("allocation")),
            Arc::new(DecisionFactory("resilience")),
        ],
        vec![GraphChangeCodec::schema()],
    )
}
#[cfg(feature = "dynamic-plugin")]
drasi_computation_plugin_sdk::export_computation_plugin!(plugin());

struct Transformer {
    kind: &'static str,
    descriptor: ComponentDescriptor,
    producer: GraphProducerIdentity,
    stream: StreamId,
    rows: BTreeMap<String, Value>,
    output: Option<Value>,
    sequence: u64,
    cursor: Option<OutputCursor>,
    input: Option<InputBinding>,
    snapshots: NativeOutputSnapshots,
    pending: Vec<OutputEnvelope>,
}
fn decode(record: &Record) -> Result<Value> {
    Ok(QueryChangeCodec::row_values_to_json(
        &QueryChangeCodec::decode_row(record)?.values,
    ))
}
#[async_trait]
impl NativeComponent for Transformer {
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
        ensure!(inputs.len() <= 1, "one upstream query required");
        if self.cursor.is_some() {
            ensure!(
                inputs.first() == self.input.as_ref(),
                "rebind requires reconstruction"
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
        if self.cursor.is_some() {
            return Ok(());
        }
        let input = self.input.as_ref().context("upstream query required")?;
        let mut snapshot = self
            .snapshots
            .snapshot(&input.output, &GraphChangeCodec::schema())
            .await?;
        if snapshot.cursor.producer().stream() != &input.stream {
            return Err(anyhow::anyhow!("snapshot stream mismatch").into());
        }
        while let Some(record) = snapshot.records.next().await {
            let row = decode(&record?)?;
            self.rows
                .insert(row["id"].as_str().context("row id")?.into(), row);
        }
        self.pending = self.emit(None)?;
        self.cursor = Some(snapshot.cursor);
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
impl NativeTransformer for Transformer {
    fn has_pending_emissions(&self) -> bool {
        !self.pending.is_empty()
    }
    async fn continue_transform(&mut self) -> Result<Vec<OutputEnvelope>> {
        Ok(std::mem::take(&mut self.pending))
    }
    async fn transform(&mut self, input: InputEnvelope) -> Result<Vec<OutputEnvelope>> {
        ensure!(input.port.as_str() == "in", "unexpected input");
        let progress = GraphProducerProgress::from_envelope(&input.envelope)?
            .context("producer progress required")?;
        let cursor = OutputCursor::try_new(progress.identity().clone(), progress.sequence())?;
        let saved = self.cursor.as_ref().context("not initialized")?;
        saved.ensure_same_epoch(&cursor)?;
        if cursor.sequence() <= saved.sequence() {
            return Ok(vec![]);
        }
        for operation in input.envelope.changes().operations() {
            match operation {
                ChangeOperation::Added { after, .. } | ChangeOperation::Updated { after, .. } => {
                    let row = decode(after)?;
                    self.rows
                        .insert(row["id"].as_str().context("row id")?.into(), row);
                }
                ChangeOperation::Deleted { before, .. } => {
                    let row = decode(before.as_ref().context("delete before image")?)?;
                    self.rows.remove(row["id"].as_str().context("delete id")?);
                }
            }
        }
        let result = self.emit(Some(&input.envelope))?;
        self.cursor = Some(cursor);
        Ok(result)
    }
}
impl Transformer {
    fn emit(&mut self, input: Option<&ChangeEnvelope>) -> Result<Vec<OutputEnvelope>> {
        let decision = if self.kind == "policy" {
            policy(&self.rows)?
        } else {
            self.rows
                .get("active")
                .map(|row| -> Result<_> {
                    let p =
                        serde_json::from_str(row["payload"].as_str().context("policy payload")?)?;
                    allocate(
                        p,
                        if self.kind == "resilience" {
                            Some("EU-A")
                        } else {
                            None
                        },
                    )
                })
                .transpose()?
        };
        let next = decision
            .map(|d| -> Result<Value> {
                Ok(
                    json!({"id":"active","run_epoch":d.run_epoch,"revision":d.revision,
                "policy_revision":d.policy_revision,"basis":d.basis,"state":d.state,
                "reason":d.reason,"candidate_id":d.candidate_id,"valid":d.valid,
                "payload":serde_json::to_string(&d)?}),
                )
            })
            .transpose()?;
        if self.output == next {
            return Ok(vec![]);
        }
        let now = chrono::Utc::now();
        let metadata = ElementMetadata {
            reference: ElementReference::new(self.stream.as_str(), "active"),
            labels: Arc::from([Arc::from("Decision")]),
            effective_from: now.timestamp_millis() as u64,
        };
        let change = if let Some(value) = &next {
            let element = Element::Node {
                metadata,
                properties: ElementPropertyMap::from(value.clone()),
            };
            if self.output.is_some() {
                SourceChange::Replace { element }
            } else {
                SourceChange::Insert { element }
            }
        } else {
            SourceChange::Delete { metadata }
        };
        self.sequence = self.sequence.checked_add(1).context("sequence exhausted")?;
        let mut envelope = if let Some(input) = input {
            GraphChangeCodec::derive_changes(input, &[change], self.stream.clone(), self.sequence)?
        } else {
            GraphChangeCodec::encode_changes(
                &[change],
                self.stream.clone(),
                self.sequence,
                Some(now),
            )?
        };
        GraphProducerProgress::annotate(&mut envelope, &self.producer, self.sequence)?;
        self.output = next;
        Ok(vec![OutputEnvelope {
            port: PortId::try_new("out")?,
            envelope,
        }])
    }
}
