//! The native transformer: keeps the current rows of its three input ports,
//! coalesces changes, solves on one worker thread off the async runtime and
//! emits only the elements that changed.
//!
//! Flow: `start` reads every bound upstream output's snapshot and schedules a
//! solve. `transform` applies row changes and schedules a solve `coalesceMs`
//! after the first unsolved change. The host calls `on_wakeup` when the
//! [`Wake`] source fires: it collects a finished result (discarding it if the
//! inputs moved on), launches the next solve when due, and emits the diff. A
//! solve that overruns its time limit is answered `unknown` by a watchdog.
use crate::{
    config::{Config, Mode},
    model::{self, Elements, Inputs},
    BINS, ELIGIBILITY, ITEMS, OUT,
};
use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use drasi_computation_contracts::*;
use drasi_computation_plugin_sdk::lifecycle::{
    nothing_to_do, DisposeContext, InitializeContext, LifecycleResult, PauseContext, ResumeContext,
    StartContext, StopContext,
};
use drasi_computation_plugin_sdk::{
    CreateRequest, NativeComponent, NativeOutputSnapshots, NativeTransformer,
};
use drasi_core_types::models::{
    Element, ElementMetadata, ElementPropertyMap, ElementReference, SourceChange,
};
use futures_util::StreamExt;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Notify;

/// Extra time after the solver's own limit before the watchdog answers `unknown`.
const WATCHDOG_GRACE: Duration = Duration::from_millis(500);

/// Reads an upstream output's snapshot: its position and full-image records.
#[async_trait]
pub trait SnapshotReader: Send + Sync {
    async fn read(&self, endpoint: &OutputEndpoint) -> Result<(OutputCursor, Vec<Record>)>;
}
#[async_trait]
impl SnapshotReader for NativeOutputSnapshots {
    async fn read(&self, endpoint: &OutputEndpoint) -> Result<(OutputCursor, Vec<Record>)> {
        let mut snapshot = self.snapshot(endpoint, &GraphChangeCodec::schema()).await?;
        let mut records = Vec::new();
        while let Some(record) = snapshot.records.next().await {
            records.push(record?);
        }
        Ok((snapshot.cursor, records))
    }
}

/// The transformer's wakeup source, shared with its worker thread.
#[derive(Default)]
pub struct Wake {
    /// When the component next needs the host to call `on_wakeup`.
    due: Mutex<Option<Instant>>,
    /// A worker result is waiting in `result`.
    ready: AtomicBool,
    result: Mutex<Option<Done>>,
    notify: Notify,
}
struct Done {
    generation: u64,
    elements: Elements,
}
#[async_trait]
impl WakeupSource for Wake {
    async fn wait(&self) -> Result<()> {
        loop {
            if self.ready.load(Ordering::Acquire) {
                return Ok(());
            }
            let due = *self.due.lock().unwrap();
            match due {
                Some(due) if due <= Instant::now() => return Ok(()),
                Some(due) => {
                    let _ = tokio::time::timeout_at(due.into(), self.notify.notified()).await;
                }
                None => self.notify.notified().await,
            }
        }
    }
    async fn has_pending(&self) -> Result<bool> {
        Ok(self.ready.load(Ordering::Acquire) || self.due.lock().unwrap().is_some())
    }
}
impl Wake {
    fn set_due(&self, due: Option<Instant>) {
        *self.due.lock().unwrap() = due;
        self.notify.notify_one();
    }
}

struct Job {
    generation: u64,
    deadline: Instant,
    inputs: Inputs,
}
/// One long-lived solver thread per component; jobs are submitted one at a time.
struct Worker {
    jobs: mpsc::SyncSender<Job>,
}
impl Worker {
    fn spawn(mode: Mode, config: Arc<Config>, wake: Arc<Wake>) -> Result<Self> {
        let (jobs, receiver) = mpsc::sync_channel::<Job>(1);
        std::thread::Builder::new()
            .name(format!("{}-solver", mode.implementation()))
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    let elements = catch_unwind(AssertUnwindSafe(|| {
                        model::elements(mode, &config, &job.inputs, job.deadline, now_ms())
                    }))
                    .unwrap_or_else(|_| {
                        model::unknown(
                            mode,
                            &config,
                            &job.inputs,
                            &model::Context::default(),
                            "The solver failed".into(),
                            now_ms(),
                        )
                    });
                    *wake.result.lock().unwrap() = Some(Done {
                        generation: job.generation,
                        elements,
                    });
                    wake.ready.store(true, Ordering::Release);
                    wake.notify.notify_one();
                }
            })?;
        Ok(Self { jobs })
    }
}
struct Active {
    generation: u64,
    deadline: Instant,
    answered: bool,
}

/// Rows of one input stream, keyed by record identity.
type RowKey = (String, Vec<u8>);

pub struct Optimizer {
    mode: Mode,
    config: Arc<Config>,
    configuration: Value,
    descriptor: ComponentDescriptor,
    producer: GraphProducerIdentity,
    stream: StreamId,
    sequence: u64,
    snapshots: Arc<dyn SnapshotReader>,
    bindings: Vec<InputBinding>,
    /// Upstream stream -> (port, position read so far).
    cursors: BTreeMap<String, (String, OutputCursor)>,
    rows: BTreeMap<String, BTreeMap<RowKey, Value>>,
    loaded: bool,
    running: bool,
    /// Increases on every input change; a result is current only for its generation.
    generation: u64,
    /// Generation of the last emitted answer.
    answered: u64,
    /// When the coalescing window for unanswered changes closes.
    due: Option<Instant>,
    wake: Arc<Wake>,
    worker: Option<Worker>,
    job: Option<Active>,
    published: Elements,
}

impl Optimizer {
    /// A component for `request`, reading upstream snapshots through `snapshots`.
    pub fn new(
        mode: Mode,
        descriptor: ComponentDescriptor,
        request: &CreateRequest,
        snapshots: Arc<dyn SnapshotReader>,
    ) -> Result<Self> {
        let config = Config::parse(&request.configuration)?;
        let scope = request.scope.as_ref().context("hosted scope required")?;
        let stream = StreamId::try_new(format!("{}/{OUT}", request.id))?;
        Ok(Self {
            mode,
            config: Arc::new(config),
            configuration: request.configuration.clone(),
            descriptor,
            producer: GraphProducerIdentity::volatile(
                scope.instance_id.clone(),
                scope.graph_id.clone(),
                request.id.clone(),
                stream.clone(),
            )?,
            stream,
            sequence: 0,
            snapshots,
            bindings: request.inputs.clone(),
            cursors: BTreeMap::new(),
            rows: BTreeMap::new(),
            loaded: false,
            running: false,
            generation: 0,
            answered: 0,
            due: None,
            wake: Arc::new(Wake::default()),
            worker: None,
            job: None,
            published: Elements::new(),
        })
    }

    fn inputs(&self) -> Inputs {
        let rows = |port: &str| -> Vec<Value> {
            self.rows
                .iter()
                .filter(|(stream, _)| {
                    self.cursors.get(*stream).is_some_and(|(p, _)| p == port)
                })
                .flat_map(|(_, rows)| rows.values().cloned())
                .collect()
        };
        Inputs {
            items: rows(ITEMS),
            bins: rows(BINS),
            eligibility: rows(ELIGIBILITY),
            unbound: [ITEMS, BINS, ELIGIBILITY]
                .into_iter()
                .filter(|port| !self.bindings.iter().any(|b| b.input_port.as_str() == *port))
                .map(String::from)
                .collect(),
        }
    }

    fn changed(&mut self) {
        self.generation += 1;
        if self.due.is_none() {
            self.due = Some(Instant::now() + Duration::from_millis(self.config.coalesce_ms));
        }
    }

    /// Start the next solve if one is due and the worker is free.
    fn launch(&mut self) -> Result<()> {
        if !self.running
            || self.job.is_some()
            || self.answered == self.generation
            || self.due.is_some_and(|due| Instant::now() < due)
        {
            return Ok(());
        }
        if self.worker.is_none() {
            self.worker = Some(Worker::spawn(
                self.mode,
                self.config.clone(),
                self.wake.clone(),
            )?);
        }
        let inputs = self.inputs();
        let deadline = Instant::now() + Duration::from_millis(self.config.time_limit_ms);
        self.worker
            .as_ref()
            .context("solver worker")?
            .jobs
            .try_send(Job {
                generation: self.generation,
                deadline,
                inputs,
            })
            .map_err(|e| anyhow::anyhow!("solver worker unavailable: {e}"))?;
        self.job = Some(Active {
            generation: self.generation,
            deadline,
            answered: false,
        });
        self.due = None;
        Ok(())
    }

    /// Tell the wakeup source when the host next needs to call `on_wakeup`.
    fn schedule(&self) {
        let due = match &self.job {
            Some(job) if !job.answered => Some(job.deadline + WATCHDOG_GRACE),
            Some(_) => None,
            None if self.running && self.answered != self.generation => {
                Some(self.due.unwrap_or_else(Instant::now))
            }
            None => None,
        };
        self.wake.set_due(due);
    }

    /// Collect a finished solve, run the watchdog, launch the next solve.
    fn step(&mut self) -> Result<Vec<OutputEnvelope>> {
        let mut next = None;
        self.wake.ready.store(false, Ordering::Release);
        if let Some(done) = self.wake.result.lock().unwrap().take() {
            if self.job.as_ref().is_some_and(|j| j.generation == done.generation) {
                self.job = None;
            }
            // A result for superseded inputs is discarded; the next solve is already due.
            if done.generation == self.generation && self.running {
                self.answered = done.generation;
                next = Some(done.elements);
            }
        }
        if let Some(job) = &mut self.job {
            if !job.answered && Instant::now() >= job.deadline + WATCHDOG_GRACE {
                job.answered = true;
                if job.generation == self.generation {
                    self.answered = job.generation;
                    next = Some(model::unknown(
                        self.mode,
                        &self.config,
                        &self.inputs(),
                        &model::Context::default(),
                        "The solver exceeded its time limit".into(),
                        now_ms(),
                    ));
                }
            }
        }
        self.launch()?;
        self.schedule();
        match next {
            Some(elements) => self.publish(elements),
            None => Ok(vec![]),
        }
    }

    /// Emit inserts, replaces and deletes that turn the published state into `next`.
    fn publish(&mut self, mut next: Elements) -> Result<Vec<OutputEnvelope>> {
        let now = now_ms();
        let mut changes = Vec::new();
        for (id, element) in &mut next {
            match self.published.get(id) {
                Some(old) if old.label == element.label && same(&old.properties, &element.properties) => {
                    element.properties = old.properties.clone();
                }
                old => {
                    let node = Element::Node {
                        metadata: metadata(&self.stream, id, element.label, now),
                        properties: ElementPropertyMap::from(element.properties.clone()),
                    };
                    match old {
                        Some(old) if old.label == element.label => {
                            changes.push(SourceChange::Replace { element: node })
                        }
                        Some(old) => {
                            changes.push(SourceChange::Delete {
                                metadata: metadata(&self.stream, id, old.label, now),
                            });
                            changes.push(SourceChange::Insert { element: node });
                        }
                        None => changes.push(SourceChange::Insert { element: node }),
                    }
                }
            }
        }
        for (id, old) in &self.published {
            if !next.contains_key(id) {
                changes.push(SourceChange::Delete {
                    metadata: metadata(&self.stream, id, old.label, now),
                });
            }
        }
        self.published = next;
        if changes.is_empty() {
            return Ok(vec![]);
        }
        self.sequence = self.sequence.checked_add(1).context("sequence exhausted")?;
        let mut envelope = GraphChangeCodec::encode_changes(
            &changes,
            self.stream.clone(),
            self.sequence,
            Some(SystemTime::now().into()),
        )?;
        GraphProducerProgress::annotate(&mut envelope, &self.producer, self.sequence)?;
        Ok(vec![OutputEnvelope {
            port: PortId::try_new(OUT)?,
            envelope,
        }])
    }

    fn port_of(&self, stream: &StreamId) -> Result<String> {
        Ok(self
            .cursors
            .get(stream.as_str())
            .with_context(|| format!("input from unbound stream {stream}"))?
            .0
            .clone())
    }
}

/// Equal apart from the time of the solve.
fn same(a: &Value, b: &Value) -> bool {
    let strip = |v: &Value| {
        let mut v = v.clone();
        if let Some(map) = v.as_object_mut() {
            map.remove("solved_ms");
        }
        v
    };
    strip(a) == strip(b)
}
fn metadata(stream: &StreamId, id: &str, label: &str, now: u64) -> ElementMetadata {
    ElementMetadata {
        reference: ElementReference::new(stream.as_str(), id),
        labels: Arc::from([Arc::from(label)]),
        effective_from: now,
    }
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
fn decode(record: &Record) -> Result<Value> {
    Ok(QueryChangeCodec::row_values_to_json(
        &QueryChangeCodec::decode_row(record)?.values,
    ))
}
fn key(stream: &str, identity: &RecordId) -> (String, RowKey) {
    (
        stream.to_owned(),
        (identity.namespace().to_string(), identity.value().to_vec()),
    )
}

#[async_trait]
impl NativeComponent for Optimizer {
    fn descriptor(&self) -> &ComponentDescriptor {
        &self.descriptor
    }
    fn configuration(&self) -> Result<Value> {
        Ok(self.configuration.clone())
    }
    fn supports_input_binding_updates(&self) -> bool {
        true
    }
    async fn bind_inputs(&mut self, inputs: &[InputBinding]) -> Result<()> {
        for binding in inputs {
            ensure!(
                [ITEMS, BINS, ELIGIBILITY].contains(&binding.input_port.as_str()),
                "unexpected input port {}",
                binding.input_port
            );
        }
        if self.loaded {
            let mut old = self.bindings.clone();
            let mut new = inputs.to_vec();
            old.sort_by_key(|b| b.stream.to_string());
            new.sort_by_key(|b| b.stream.to_string());
            ensure!(old == new, "changing inputs requires reconstruction");
        }
        self.bindings = inputs.to_vec();
        Ok(())
    }
    async fn initialize(&mut self, _ctx: &InitializeContext) -> LifecycleResult {
        nothing_to_do()
    }
    /// Reads every upstream snapshot once; a later start resumes with the rows held.
    async fn start(&mut self, _ctx: &mut StartContext) -> LifecycleResult {
        if !self.loaded {
            for binding in self.bindings.clone() {
                let (cursor, records) = self.snapshots.read(&binding.output).await?;
                if cursor.producer().stream() != &binding.stream {
                    return Err(anyhow::anyhow!(
                        "snapshot of {} is for another stream",
                        binding.output
                    )
                    .into());
                }
                let stream = binding.stream.to_string();
                let rows = self.rows.entry(stream.clone()).or_default();
                for record in records {
                    let row = decode(&record)?;
                    rows.insert(key(&stream, record.identity()).1, row);
                }
                self.cursors
                    .insert(stream, (binding.input_port.to_string(), cursor));
            }
            self.loaded = true;
        }
        self.running = true;
        self.answered = 0;
        self.generation += 1;
        self.due = None;
        self.schedule();
        Ok(())
    }
    // Output is produced only through host calls; the worker emits nothing itself.
    async fn pause(&mut self, _ctx: &PauseContext) -> LifecycleResult {
        nothing_to_do()
    }
    async fn resume(&mut self, _ctx: &ResumeContext) -> LifecycleResult {
        nothing_to_do()
    }
    /// The in-flight solve (bounded by the time limit) finishes on its own thread
    /// and is discarded; the next start solves again.
    async fn stop(&mut self, _ctx: &StopContext) -> LifecycleResult {
        self.running = false;
        self.job = None;
        self.worker = None;
        self.wake.result.lock().unwrap().take();
        self.wake.ready.store(false, Ordering::Release);
        self.wake.set_due(None);
        Ok(())
    }
    // Volatile: a new instance rebuilds everything from upstream snapshots.
    async fn dispose(&mut self, _ctx: &DisposeContext) -> LifecycleResult {
        nothing_to_do()
    }
}

#[async_trait]
impl NativeTransformer for Optimizer {
    fn wakeup_source(&self) -> Option<Arc<dyn WakeupSource>> {
        Some(self.wake.clone())
    }
    async fn on_wakeup(&mut self) -> Result<Vec<OutputEnvelope>> {
        self.step()
    }
    async fn transform(&mut self, input: InputEnvelope) -> Result<Vec<OutputEnvelope>> {
        let progress = GraphProducerProgress::from_envelope(&input.envelope)?
            .context("producer progress required")?;
        let stream = progress.identity().stream().clone();
        let port = self.port_of(&stream)?;
        ensure!(
            port == input.port.as_str(),
            "stream {stream} arrived on {} but is bound to {port}",
            input.port
        );
        let cursor = OutputCursor::try_new(progress.identity().clone(), progress.sequence())?;
        let saved = &self.cursors[stream.as_str()].1;
        saved.ensure_same_epoch(&cursor)?;
        if cursor.sequence() <= saved.sequence() {
            return Ok(vec![]);
        }
        let rows = self.rows.entry(stream.to_string()).or_default();
        let mut changed = false;
        for operation in input.envelope.changes().operations() {
            match operation {
                ChangeOperation::Added { after, .. } => {
                    rows.insert(key(stream.as_str(), after.identity()).1, decode(after)?);
                }
                ChangeOperation::Updated { before, after, .. } => {
                    if let Some(before) = before {
                        rows.remove(&key(stream.as_str(), before.identity()).1);
                    }
                    rows.insert(key(stream.as_str(), after.identity()).1, decode(after)?);
                }
                ChangeOperation::Deleted { identity, .. } => {
                    rows.remove(&key(stream.as_str(), identity.identity()).1);
                }
            }
            changed = true;
        }
        self.cursors.get_mut(stream.as_str()).context("cursor")?.1 = cursor;
        if changed {
            self.changed();
            self.schedule();
        }
        Ok(vec![])
    }
}

