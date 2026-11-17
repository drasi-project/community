//! `ignite/rego-policy`: a native transformer that evaluates a Rego rule for
//! every subject × target pair and emits one `PolicyDecision` per pair.
//!
//! Contract: `plan/contracts.md` (Rego policy plugin, element contracts).
//!
//! Layout: `config`, `evaluate` and `decide` are plain Rust with host-free unit
//! tests; this file is the thin SDK wrapper: factory, input bindings per port,
//! snapshot bootstrap, per-upstream cursors, and encoding of changed decisions.
use anyhow::{bail, ensure, Context, Result};
use async_trait::async_trait;
use drasi_computation_contracts::*;
use drasi_computation_plugin_sdk::lifecycle::{
    nothing_to_do, DisposeContext, InitializeContext, LifecycleResult, PauseContext, ResumeContext,
    StartContext, StopContext,
};
use drasi_computation_plugin_sdk::{
    Capabilities, Component, ConfigSchema, ConfigType, ControlSender, CreateRequest, CreatedComponent,
    Factory, FactoryMetadata, NativeComponent, NativeOutputSnapshots, NativeTransformer,
    PluginDefinition,
};
use drasi_core_types::models::{Element, ElementMetadata, ElementPropertyMap, ElementReference, SourceChange};
use futures_util::StreamExt;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock, Weak},
    time::{SystemTime, UNIX_EPOCH},
};

pub mod config;
pub mod decide;
pub mod evaluate;
pub use config::Config;
use decide::{Change, Policy, Tables};

pub const IMPLEMENTATION: &str = "ignite/rego-policy";
/// Input ports: one per role. Each may have any number of incoming edges.
pub const SUBJECTS: &str = "subjects";
pub const TARGETS: &str = "targets";
pub const PARAMETERS: &str = "parameters";
pub const OUT: &str = "out";
/// Label of every emitted element.
pub const LABEL: &str = "PolicyDecision";

pub struct PolicyFactory;
impl Factory for PolicyFactory {
    /// Stateless: outputs carry a new volatile producer identity on every construction,
    /// and a new instance rebuilds its state from its upstreams' snapshots.
    fn describe(
        &self,
        request: &drasi_computation_plugin_sdk::wire::DescribeRequest,
    ) -> Result<drasi_computation_plugin_sdk::ComponentDescription> {
        Ok(request.volatile_transformer())
    }
    fn metadata(&self) -> FactoryMetadata {
        FactoryMetadata {
            implementation: ImplementationIdentity::try_new(IMPLEMENTATION, "1").unwrap(),
            role: ComponentRole::Transformer,
            configuration_version: 1,
            configuration: ConfigSchema::from_fields([
                ("module", ConfigType::String, true),
                ("rule", ConfigType::String, true),
                ("subjects", ConfigType::Object, true),
                ("targets", ConfigType::Object, true),
                ("parameters", ConfigType::Object, true),
                ("epochProperty", ConfigType::String, false),
                ("timeoutMs", ConfigType::Integer, false),
            ]),
            ports: ports(&[SUBJECTS, TARGETS, PARAMETERS], OUT),
            completion: None,
            capabilities: Capabilities {
                continuations: true,
                startup_continuation: true,
                input_binding_updates: true,
                output_snapshots: vec![PortId::try_new(OUT).unwrap()],
                ..Capabilities::default()
            },
        }
    }
    fn create(&self, _: &CreateRequest, _: ControlSender) -> Result<CreatedComponent> {
        bail!("rego-policy requires native upstream snapshots (an output_snapshots dependency)")
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
        let snapshots = snapshots.context("rego-policy requires an output_snapshots dependency")?;
        Ok(Component::Transformer(Box::new(Transformer::new(request, Arc::new(snapshots))?)).into())
    }
}

/// Reads one upstream output's snapshot: its cursor and full-image records.
#[async_trait]
pub(crate) trait SnapshotSource: Send + Sync {
    async fn read(&self, output: &OutputEndpoint) -> Result<(OutputCursor, Vec<Record>)>;
}

#[async_trait]
impl SnapshotSource for NativeOutputSnapshots {
    async fn read(&self, output: &OutputEndpoint) -> Result<(OutputCursor, Vec<Record>)> {
        let mut snapshot = self.snapshot(output, &GraphChangeCodec::schema()).await?;
        let mut records = Vec::new();
        while let Some(record) = snapshot.records.next().await {
            records.push(record?);
        }
        Ok((snapshot.cursor, records))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Subjects,
    Targets,
    Parameters,
}

impl Role {
    fn of(port: &PortId) -> Result<Self> {
        Ok(match port.as_str() {
            SUBJECTS => Self::Subjects,
            TARGETS => Self::Targets,
            PARAMETERS => Self::Parameters,
            other => bail!("{IMPLEMENTATION} has no input port '{other}'"),
        })
    }
}

/// One incoming edge: its rows (by record identity) and the position they reflect.
struct Upstream {
    binding: InputBinding,
    role: Role,
    cursor: Option<OutputCursor>,
    rows: HashMap<RecordId, Result<Value, String>>,
}

impl Upstream {
    fn new(binding: InputBinding) -> Result<Self> {
        Ok(Self { role: Role::of(&binding.input_port)?, binding, cursor: None, rows: HashMap::new() })
    }

    fn matches(&self, port: &PortId, producer: &GraphProducerIdentity) -> bool {
        &self.binding.input_port == port
            && &self.binding.output.component == producer.component_id()
            && &self.binding.stream == producer.stream()
    }

    /// Replaces this edge's rows with its upstream's current snapshot.
    async fn load(&mut self, source: &dyn SnapshotSource) -> Result<()> {
        let (cursor, records) = source.read(&self.binding.output).await?;
        ensure!(
            cursor.producer().stream() == &self.binding.stream,
            "snapshot of {} is for another stream",
            self.binding.output
        );
        self.rows = records.iter().map(|record| (record.identity().clone(), decode(record))).collect();
        self.cursor = Some(cursor);
        Ok(())
    }

    fn apply(&mut self, envelope: &ChangeEnvelope) {
        for operation in envelope.changes().operations() {
            match operation {
                ChangeOperation::Added { after, .. } => {
                    self.rows.insert(after.identity().clone(), decode(after));
                }
                ChangeOperation::Updated { before, after, .. } => {
                    if let Some(before) = before.as_ref().filter(|b| b.identity() != after.identity()) {
                        self.rows.remove(before.identity());
                    }
                    self.rows.insert(after.identity().clone(), decode(after));
                }
                ChangeOperation::Deleted { identity, .. } => {
                    self.rows.remove(identity.identity());
                }
            }
        }
    }
}

/// A query result row as JSON; an unreadable row becomes a fault (and every
/// decision `unknown`) rather than a failure of the component.
fn decode(record: &Record) -> Result<Value, String> {
    QueryChangeCodec::decode_row(record)
        .map(|row| QueryChangeCodec::row_values_to_json(&row.values))
        .map_err(|error| format!("unreadable input row: {error}"))
}

pub(crate) struct Transformer {
    descriptor: ComponentDescriptor,
    configuration: Value,
    policy: Policy,
    producer: GraphProducerIdentity,
    stream: StreamId,
    sequence: u64,
    upstreams: Vec<Upstream>,
    started: bool,
    source: Arc<dyn SnapshotSource>,
    pending: Vec<OutputEnvelope>,
    published: Arc<RwLock<Published>>,
}

impl Transformer {
    pub(crate) fn new(request: &CreateRequest, source: Arc<dyn SnapshotSource>) -> Result<Self> {
        let factory = PolicyFactory;
        factory.metadata().configuration.validate(&request.configuration)?;
        let config = Config::parse(&request.configuration)?;
        let policy = Policy::new(config).with_context(|| format!("{IMPLEMENTATION}: invalid module"))?;
        let scope = request.scope.as_ref().context("hosted scope required")?;
        let stream = StreamId::try_new(format!("{}/{OUT}", request.id))?;
        let producer = GraphProducerIdentity::volatile(
            scope.instance_id.clone(),
            scope.graph_id.clone(),
            request.id.clone(),
            stream.clone(),
        )?;
        let published = Arc::new(RwLock::new(Published {
            cursor: OutputCursor::try_new(producer.clone(), 0)?,
            records: Vec::new(),
            ready: false,
            running: false,
        }));
        let mut upstreams = Vec::new();
        for binding in &request.inputs {
            upstreams.push(Upstream::new(binding.clone())?);
        }
        Ok(Self {
            descriptor: factory.metadata().descriptor(request.id.clone())?,
            configuration: request.configuration.clone(),
            policy,
            producer,
            stream,
            sequence: 0,
            upstreams,
            started: false,
            source,
            pending: Vec::new(),
            published,
        })
    }

    /// Every port's rows, once each port has at least one edge and every edge
    /// has loaded its snapshot; `None` until then.
    fn tables(&self) -> Option<Tables> {
        let mut tables = Tables::default();
        for role in [Role::Subjects, Role::Targets, Role::Parameters] {
            let edges: Vec<_> = self.upstreams.iter().filter(|u| u.role == role).collect();
            if edges.is_empty() || edges.iter().any(|u| u.cursor.is_none()) {
                return None;
            }
            let rows = match role {
                Role::Subjects => &mut tables.subjects,
                Role::Targets => &mut tables.targets,
                Role::Parameters => &mut tables.parameters,
            };
            for row in edges.iter().flat_map(|u| u.rows.values()) {
                match row {
                    Ok(row) => rows.push(row.clone()),
                    Err(fault) => tables.faults.push(fault.clone()),
                }
            }
        }
        Some(tables)
    }

    /// Re-decides every pair and encodes only the decisions that changed.
    fn emit(&mut self, input: Option<&ChangeEnvelope>) -> Result<Vec<OutputEnvelope>> {
        let Some(tables) = self.tables() else {
            return Ok(vec![]);
        };
        let changes = self.policy.update(&tables);
        let first = !self.published.read().map_err(poisoned)?.ready;
        if changes.is_empty() {
            if first {
                self.publish()?;
            }
            return Ok(vec![]);
        }
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        let changes: Vec<SourceChange> = changes
            .into_iter()
            .map(|change| match change {
                Change::Upsert { id, properties, replaces } => {
                    let element = self.element(&id, properties, now);
                    if replaces {
                        SourceChange::Replace { element }
                    } else {
                        SourceChange::Insert { element }
                    }
                }
                Change::Delete { id } => SourceChange::Delete { metadata: self.metadata(&id, now) },
            })
            .collect();
        self.sequence = self.sequence.checked_add(1).context("sequence exhausted")?;
        let mut envelope = match input {
            Some(input) => GraphChangeCodec::derive_changes(input, &changes, self.stream.clone(), self.sequence)?,
            None => GraphChangeCodec::encode_changes(&changes, self.stream.clone(), self.sequence, None)?,
        };
        GraphProducerProgress::annotate(&mut envelope, &self.producer, self.sequence)?;
        self.publish()?;
        Ok(vec![OutputEnvelope { port: PortId::try_new(OUT)?, envelope }])
    }

    fn metadata(&self, id: &str, now: u64) -> ElementMetadata {
        ElementMetadata {
            reference: ElementReference::new(self.stream.as_str(), id),
            labels: Arc::from([Arc::from(LABEL)]),
            effective_from: now,
        }
    }

    fn element(&self, id: &str, properties: Value, now: u64) -> Element {
        Element::Node { metadata: self.metadata(id, now), properties: ElementPropertyMap::from(properties) }
    }

    /// Makes the current decisions, at the current sequence, this output's snapshot.
    fn publish(&self) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        let records = self
            .policy
            .current()
            .iter()
            .map(|(id, properties)| {
                let image = ElementImage::try_from(self.element(id, properties.clone(), now))?;
                Ok(GraphChangeCodec::element_record(&image)?)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut published = self.published.write().map_err(poisoned)?;
        published.cursor = published.cursor.at_sequence(self.sequence);
        published.records = records;
        published.ready = true;
        Ok(())
    }

    /// Loads every edge that has no position yet.
    async fn load(&mut self) -> Result<()> {
        let source = self.source.clone();
        for upstream in self.upstreams.iter_mut().filter(|u| u.cursor.is_none()) {
            upstream.load(source.as_ref()).await?;
        }
        Ok(())
    }
}

fn poisoned<T>(_: T) -> anyhow::Error {
    anyhow::anyhow!("{IMPLEMENTATION} output state poisoned")
}

#[async_trait]
impl NativeComponent for Transformer {
    fn descriptor(&self) -> &ComponentDescriptor {
        &self.descriptor
    }
    fn configuration(&self) -> Result<Value> {
        Ok(self.configuration.clone())
    }
    fn supports_input_binding_updates(&self) -> bool {
        true
    }
    fn output_readers(&self) -> OutputReaders {
        OutputReaders::new().with_snapshot(
            PortId::try_new(OUT).expect("constant port"),
            Arc::new(Snapshots(Arc::downgrade(&self.published))),
        )
    }

    /// Keeps the rows of edges that remain; a new edge after start is loaded
    /// from its snapshot at once and the decisions it changes are queued.
    async fn bind_inputs(&mut self, bindings: &[InputBinding]) -> Result<()> {
        let mut next = Vec::with_capacity(bindings.len());
        for binding in bindings {
            match self.upstreams.iter().position(|u| &u.binding == binding) {
                Some(index) => next.push(self.upstreams.swap_remove(index)),
                None => next.push(Upstream::new(binding.clone())?),
            }
        }
        self.upstreams = next;
        if self.started {
            self.load().await?;
            let emitted = self.emit(None)?;
            self.pending.extend(emitted);
        }
        Ok(())
    }
    async fn initialize(&mut self, _ctx: &InitializeContext) -> LifecycleResult {
        nothing_to_do()
    }

    /// Snapshot bootstrap: reads every upstream output once, then emits the
    /// first decisions as a continuation. Later starts resume from the cursors.
    async fn start(&mut self, _ctx: &mut StartContext) -> LifecycleResult {
        if !self.started {
            self.load().await?;
            self.started = true;
            self.pending = self.emit(None)?;
        }
        self.published.write().map_err(poisoned)?.running = true;
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
        self.published.write().map_err(poisoned)?.running = false;
        Ok(())
    }
    // State is volatile and rebuilt from the upstream snapshots by a new instance.
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
        ensure!(self.started, "{IMPLEMENTATION} is not started");
        let progress = GraphProducerProgress::from_envelope(&input.envelope)?
            .context("producer progress required")?;
        let cursor = OutputCursor::try_new(progress.identity().clone(), progress.sequence())?;
        let source = self.source.clone();
        let upstream = self
            .upstreams
            .iter_mut()
            .find(|u| u.matches(&input.port, progress.identity()))
            .with_context(|| format!("input on '{}' from an unbound output", input.port.as_str()))?;
        // An upstream that restarted (new volatile epoch) is re-read from its snapshot.
        let same_epoch = upstream.cursor.as_ref().is_some_and(|saved| saved.ensure_same_epoch(&cursor).is_ok());
        if !same_epoch {
            upstream.load(source.as_ref()).await?;
        }
        let saved = upstream.cursor.as_ref().context("edge not loaded")?;
        if saved.ensure_same_epoch(&cursor).is_ok() && cursor.sequence() > saved.sequence() {
            upstream.apply(&input.envelope);
            upstream.cursor = Some(cursor);
        } else if same_epoch {
            return Ok(vec![]);
        }
        let mut output = std::mem::take(&mut self.pending);
        output.extend(self.emit(Some(&input.envelope))?);
        Ok(output)
    }
}

struct Published {
    cursor: OutputCursor,
    records: Vec<Record>,
    /// Set once every port has loaded and the first decisions exist.
    ready: bool,
    running: bool,
}

/// This output's snapshot: the current decisions at the last emitted sequence.
struct Snapshots(Weak<RwLock<Published>>);

#[async_trait]
impl SnapshotProvider for Snapshots {
    async fn snapshot(&self) -> Result<OutputSnapshot, OutputReadError> {
        let published = self.0.upgrade().ok_or(OutputReadError::Retired)?;
        let published = published
            .read()
            .map_err(|_| OutputReadError::Failed(anyhow::anyhow!("{IMPLEMENTATION} output state poisoned")))?;
        if !published.running || !published.ready {
            return Err(OutputReadError::NotReady(format!("{IMPLEMENTATION} has not loaded all its inputs")));
        }
        OutputSnapshot::try_new(
            published.cursor.clone(),
            published.cursor.producer().component_id().to_string(),
            GraphChangeCodec::schema().descriptor().clone(),
            published.records.iter().map(|r| (r.identity().clone(), r.clone())).collect(),
        )
    }
}

pub(crate) fn ports(inputs: &[&str], output: &str) -> Vec<PortDescriptor> {
    inputs
        .iter()
        .map(|name| (*name, PortDirection::Input))
        .chain(std::iter::once((output, PortDirection::Output)))
        .map(|(name, direction)| {
            PortDescriptor::new(
                PortId::try_new(name).unwrap(),
                direction,
                GraphChangeCodec::schema().descriptor().clone(),
                PipeRequirements::default(),
            )
        })
        .collect()
}

pub fn plugin() -> Result<PluginDefinition> {
    PluginDefinition::new(
        "ignite-policy",
        env!("CARGO_PKG_VERSION"),
        vec![Arc::new(PolicyFactory)],
        vec![GraphChangeCodec::schema()],
    )
}
#[cfg(feature = "dynamic-plugin")]
drasi_computation_plugin_sdk::export_computation_plugin!(plugin());

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::collections::BTreeMap;

    const MODULE: &str = include_str!("../ignite.rego");

    fn example() -> Value {
        let mut value: Value = serde_json::from_str(include_str!("../examples/ignite.json")).unwrap();
        value["module"] = json!(MODULE);
        value
    }

    #[test]
    fn example_configuration_is_valid() {
        let factory = PolicyFactory;
        factory.metadata().configuration.validate(&example()).unwrap();
        let config = Config::parse(&example()).unwrap();
        assert_eq!(config.parameters.target_key, "deployment");
        let mut wrong = example();
        wrong["unknown"] = json!(1);
        assert!(factory.metadata().configuration.validate(&wrong).is_err() || Config::parse(&wrong).is_err());
        assert!(plugin().is_ok());
        let ports: Vec<_> = factory.metadata().ports.iter().map(|p| p.id().as_str().to_owned()).collect();
        assert_eq!(ports, [SUBJECTS, TARGETS, PARAMETERS, OUT]);
    }

    // SDK-level: factory, description, and the component driven through its
    // native traits with real query-output envelopes from three upstreams.
    mod sdk {
        use super::*;
        use crate::decide::tests::{deployment, destination, workload};
        use drasi_computation_plugin_sdk::lifecycle::{CancellationToken, OperationScope};
        use drasi_computation_plugin_sdk::wire::{DescribeRequest, Scope};
        use drasi_core_types::evaluation::context::{QueryPartEvaluationContext, QueryVariables};
        use std::{
            future::Future,
            hash::{DefaultHasher, Hash, Hasher},
            sync::Mutex,
            task::{Context as TaskContext, Poll, Waker},
        };

        /// The fake source never pends, so polling to completion suffices.
        fn block_on<F: Future>(future: F) -> F::Output {
            let mut future = std::pin::pin!(future);
            let mut context = TaskContext::from_waker(Waker::noop());
            loop {
                if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                    return value;
                }
            }
        }

        /// A context query's output: identity, projector and current rows.
        struct Query {
            name: String,
            producer: GraphProducerIdentity,
            projector: ResultNodeProjector,
            sequence: u64,
            rows: Vec<Value>,
        }

        impl Query {
            fn new(name: &'static str, rows: Vec<Value>) -> Self {
                let name = uuid(name);
                let stream = StreamId::try_new(format!("{name}/out")).unwrap();
                Self {
                    producer: GraphProducerIdentity::volatile(
                        "instance".into(),
                        "graph".into(),
                        ComponentId::try_new(name.as_str()).unwrap(),
                        stream,
                    )
                    .unwrap(),
                    projector: ResultNodeProjector::try_new(&format!("{name}/out"), Some("Row")).unwrap(),
                    sequence: 3,
                    rows,
                    name,
                }
            }
            fn endpoint(&self) -> OutputEndpoint {
                OutputEndpoint::new(ComponentId::try_new(self.name.as_str()).unwrap(), PortId::try_new("out").unwrap())
            }
            fn binding(&self, port: &str) -> InputBinding {
                InputBinding {
                    input_port: PortId::try_new(port).unwrap(),
                    output: self.endpoint(),
                    stream: self.producer.stream().clone(),
                }
            }
            fn snapshot(&self) -> (OutputCursor, Vec<Record>) {
                let records = self
                    .rows
                    .iter()
                    .map(|row| QueryChangeCodec::encode_row(&self.projector, signature(row), &vars(row), 1).unwrap())
                    .collect();
                (OutputCursor::try_new(self.producer.clone(), self.sequence).unwrap(), records)
            }
            /// The next live envelope, as the query would emit it.
            fn emit(&mut self, port: &str, change: QueryPartEvaluationContext) -> InputEnvelope {
                self.sequence += 1;
                let mut envelope = QueryChangeCodec::encode_evaluation(
                    None,
                    self.producer.component_id(),
                    SystemMetadata::new(self.producer.stream().clone(), self.sequence),
                    &self.projector,
                    &[change],
                    QueryOutputMetadata {
                        query_id: self.producer.component_id().clone(),
                        input_origin: None,
                        timestamp: "2026-11-19T00:00:00Z".parse().unwrap(),
                        metadata: Default::default(),
                        profiling: None,
                    },
                )
                .unwrap()
                .unwrap();
                GraphProducerProgress::annotate(&mut envelope, &self.producer, self.sequence).unwrap();
                InputEnvelope { port: PortId::try_new(port).unwrap(), envelope }
            }
        }

        /// Component IDs are UUIDs; derive a stable one from a readable name.
        fn uuid(name: &str) -> String {
            let mut hasher = DefaultHasher::new();
            name.hash(&mut hasher);
            format!("00000000-0000-4000-8000-{:012x}", hasher.finish() & 0xffff_ffff_ffff)
        }

        fn vars(row: &Value) -> QueryVariables {
            row.as_object().unwrap().iter().map(|(k, v)| (k.as_str().into(), v.clone().into())).collect()
        }
        fn signature(row: &Value) -> u64 {
            let mut hasher = DefaultHasher::new();
            row["id"].as_str().unwrap().hash(&mut hasher);
            hasher.finish()
        }

        type Snapshot = (OutputCursor, Vec<Record>);

        #[derive(Default)]
        struct Source(Mutex<Vec<(OutputEndpoint, Snapshot)>>);
        impl Source {
            fn set(&self, query: &Query) {
                let mut all = self.0.lock().unwrap();
                all.retain(|(endpoint, _)| endpoint != &query.endpoint());
                all.push((query.endpoint(), query.snapshot()));
            }
        }
        #[async_trait]
        impl SnapshotSource for Source {
            async fn read(&self, output: &OutputEndpoint) -> Result<(OutputCursor, Vec<Record>)> {
                let all = self.0.lock().unwrap();
                let (_, snapshot) = all.iter().find(|(e, _)| e == output).context("output not ready")?;
                Ok(snapshot.clone())
            }
        }

        fn request(configuration: Value) -> CreateRequest {
            let factory = PolicyFactory;
            let metadata = factory.metadata();
            CreateRequest {
                id: ComponentId::try_new(uuid("policy")).unwrap(),
                implementation: metadata.implementation,
                configuration_version: metadata.configuration_version,
                configuration,
                scope: Some(Scope { instance_id: "instance".into(), graph_id: "graph".into(), generation: 1 }),
                inputs: Vec::new(),
                activation_credentials_bound: false,
                update_directive: None,
            }
        }

        fn scope() -> OperationScope {
            OperationScope::new("op", "policy", 1, None, CancellationToken::new())
        }

        /// Decoded output: (id, properties) per upsert; `None` properties for a delete.
        fn decode_output(outputs: &[OutputEnvelope]) -> Vec<Option<Value>> {
            outputs
                .iter()
                .flat_map(|output| {
                    assert_eq!(output.port.as_str(), OUT);
                    output.envelope.changes().operations().to_vec()
                })
                .map(|operation| match operation {
                    ChangeOperation::Added { after, .. } | ChangeOperation::Updated { after, .. } => {
                        let ElementImage::Node { metadata, properties } = GraphChangeCodec::decode_element(&after).unwrap()
                        else {
                            panic!("not a node")
                        };
                        assert_eq!(&*metadata.labels, [Arc::<str>::from(LABEL)]);
                        Some(properties_json(&properties).unwrap())
                    }
                    ChangeOperation::Deleted { .. } => None,
                })
                .collect()
        }

        #[test]
        fn factory_describes_a_volatile_transformer_and_validates_configuration() {
            let factory = PolicyFactory;
            let request = request(example());
            let description = factory
                .describe(&DescribeRequest {
                    request: request.clone(),
                    descriptor: factory.metadata().descriptor(request.id.clone()).unwrap(),
                    dependencies: Default::default(),
                })
                .unwrap();
            assert_eq!(description.component.progress, ProgressPersistence::Volatile);
            assert!(description.ports.contains_key(&PortId::try_new(OUT).unwrap()));
            let error = factory.create_with_output_snapshots(&request, ControlSender::default(), None);
            assert!(error.is_err(), "needs the snapshot dependency");
            let mut broken = example();
            broken["module"] = json!("package ignite\nnot rego");
            assert!(Transformer::new(&self::request(broken), Arc::new(Source::default())).is_err());
            let mut slow = example();
            slow["timeoutMs"] = json!(0);
            assert!(Transformer::new(&self::request(slow), Arc::new(Source::default())).is_err());
        }

        #[test]
        fn bootstraps_from_snapshots_and_emits_only_changed_decisions() {
            let source = Arc::new(Source::default());
            let workloads = Query::new("policy-workloads", vec![workload("support"), workload("batch")]);
            let mut deployments = Query::new("policy-deployments", ["EU-A", "EU-B", "US-C"].map(deployment).to_vec());
            let mut destinations = Query::new(
                "policy-destinations",
                [("support", "EU-A"), ("support", "EU-B"), ("batch", "EU-A"), ("batch", "EU-B"), ("batch", "US-C")]
                    .map(|(w, d)| destination(w, d))
                    .to_vec(),
            );
            for query in [&workloads, &deployments, &destinations] {
                source.set(query);
            }
            let mut component = Transformer::new(&request(example()), source.clone()).unwrap();
            let reader = component.output_readers().ports()[&PortId::try_new(OUT).unwrap()].snapshot().unwrap().clone();

            // Two ports bound: nothing is emitted, and the output is not ready.
            block_on(component.bind_inputs(&[workloads.binding(SUBJECTS), deployments.binding(TARGETS)])).unwrap();
            block_on(component.start(&mut StartContext::new(scope(), None))).unwrap();
            assert!(!component.has_pending_emissions());
            assert!(matches!(block_on(reader.snapshot()), Err(OutputReadError::NotReady(_))));
            assert!(block_on(component.bind_inputs(&[workloads.binding("other")])).is_err());

            // The third port arrives: its snapshot is read and the grid emitted.
            block_on(component.bind_inputs(&[
                workloads.binding(SUBJECTS),
                deployments.binding(TARGETS),
                destinations.binding(PARAMETERS),
            ]))
            .unwrap();
            assert!(component.has_pending_emissions());
            let first = decode_output(&block_on(component.continue_transform()).unwrap());
            let first: BTreeMap<_, _> = first
                .into_iter()
                .map(|d| d.unwrap())
                .map(|d| (d["id"].as_str().unwrap().to_owned(), d))
                .collect();
            assert_eq!(first.len(), 6);
            assert_eq!(first["support/EU-B"]["state"], "allowed");
            assert_eq!(first["support/EU-B"]["allowed"], true);
            assert_eq!(first["support/US-C"]["state"], "denied");
            assert_eq!(first["batch/US-C"]["allowed"], true);
            assert_eq!(first["support/US-C"]["run_epoch"], 1);
            let snapshot = block_on(reader.snapshot()).unwrap();
            assert_eq!(snapshot.cursor().sequence(), 1);
            assert_eq!(snapshot.cursor().producer().stream().as_str(), format!("{}/out", uuid("policy")));

            // The talk's policy edit: EU-B leaves support's destinations.
            let removed = destination("support", "EU-B");
            let edit = destinations.emit(
                PARAMETERS,
                QueryPartEvaluationContext::Removing { before: vars(&removed), row_signature: signature(&removed) },
            );
            let changed = decode_output(&block_on(component.transform(edit.clone())).unwrap());
            let changed: Vec<_> = changed.into_iter().map(|d| d.unwrap()).collect();
            let ids: Vec<_> = changed.iter().map(|d| d["id"].as_str().unwrap()).collect();
            assert_eq!(ids, ["support/EU-A", "support/EU-B", "support/US-C"]);
            assert_eq!(changed[1]["state"], "denied");
            assert_eq!(changed[1]["reason"], "destination excluded by current policy");
            assert_eq!(changed[0]["state"], "allowed");
            assert_ne!(changed[0]["basis"], first["support/EU-A"]["basis"]);

            // A replayed envelope changes nothing.
            assert!(block_on(component.transform(edit)).unwrap().is_empty());

            // A target row deleted: its pairs are deleted.
            let us = deployment("US-C");
            let delete = deployments.emit(
                TARGETS,
                QueryPartEvaluationContext::Removing { before: vars(&us), row_signature: signature(&us) },
            );
            assert_eq!(decode_output(&block_on(component.transform(delete)).unwrap()), [None, None]);
            let snapshot = block_on(reader.snapshot()).unwrap();
            assert_eq!(snapshot.cursor().sequence(), 3);
            assert_eq!(snapshot.records().len(), 4);

            // An update that changes nothing the policy reads still moves the basis.
            let mut bigger = deployment("EU-A");
            bigger["capacity_tpm"] = json!(150000);
            let update = deployments.emit(
                TARGETS,
                QueryPartEvaluationContext::Updating {
                    before: vars(&deployment("EU-A")),
                    after: vars(&bigger),
                    row_signature: signature(&bigger),
                },
            );
            let updated = decode_output(&block_on(component.transform(update)).unwrap());
            assert_eq!(updated.len(), 2);
            assert!(updated.iter().all(|d| d.as_ref().unwrap()["target"] == "EU-A"));

            // The upstream restarts with a new epoch: it is re-read from its snapshot.
            let restored = destinations.rows.clone();
            destinations = Query::new("policy-destinations", restored);
            source.set(&destinations);
            let noop = destination("batch", "EU-A");
            let after_restart = destinations.emit(
                PARAMETERS,
                QueryPartEvaluationContext::Updating {
                    before: vars(&noop),
                    after: vars(&noop),
                    row_signature: signature(&noop),
                },
            );
            let restored = decode_output(&block_on(component.transform(after_restart)).unwrap());
            let restored: Vec<_> = restored.into_iter().map(|d| d.unwrap()).collect();
            assert!(restored.iter().any(|d| d["id"] == "support/EU-B" && d["state"] == "allowed"));

            // Input from an edge that is not bound is rejected.
            let mut stray = Query::new("stray", vec![]);
            let stray = stray.emit(SUBJECTS, QueryPartEvaluationContext::Adding {
                after: vars(&workload("support")),
                row_signature: 1,
            });
            assert!(block_on(component.transform(stray)).is_err());
        }
    }
}
