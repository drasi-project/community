//! The native sink: lifecycle, input handling, snapshot loading and the
//! `/metrics` listener. All row logic lives in [`crate::state`].
use crate::config::Config;
use crate::state::{Applied, Exporter, Loaded, Row, RowChange};
use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use drasi_computation_contracts::*;
use drasi_computation_plugin_sdk::lifecycle::{
    DisposeContext, InitializeContext, LifecycleError, LifecycleResult, PauseContext,
    ResumeContext, StartContext, StopContext, StopMode,
};
use drasi_computation_plugin_sdk::{NativeComponent, NativeOutputSnapshots, NativeSink};
use futures_util::StreamExt;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};
use tokio::{net::TcpListener, sync::Notify, task::JoinHandle, time::Instant};

pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";
const DEFAULT_STOP_BUDGET: Duration = Duration::from_secs(10);
const RETRY_MIN: Duration = Duration::from_millis(200);
const RETRY_MAX: Duration = Duration::from_secs(2);

macro_rules! warn {
    ($($arg:tt)+) => { eprintln!("ignite.prometheus WARN: {}", format_args!($($arg)+)) };
}

/// Shared between the sink, its loaders and the HTTP handler.
pub(crate) struct Shared {
    exporter: Mutex<Exporter>,
    /// Envelopes accepted by `handle` since creation, across restarts.
    pub(crate) handled: std::sync::atomic::AtomicU64,
}

impl Shared {
    pub(crate) fn new(exporter: Exporter) -> Arc<Self> {
        Arc::new(Self {
            exporter: Mutex::new(exporter),
            handled: std::sync::atomic::AtomicU64::new(0),
        })
    }
    pub(crate) fn lock(&self) -> MutexGuard<'_, Exporter> {
        match self.exporter.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

/// One configured output with an incoming edge.
#[derive(Clone, Debug)]
pub(crate) struct Binding {
    pub(crate) alias: String,
    pub(crate) endpoint: OutputEndpoint,
    pub(crate) stream: StreamId,
}

/// Match configured aliases to the graph's incoming edges. Aliases without an
/// edge stay unattached (they export only `drasi_reaction_input_ready 0`).
pub(crate) fn bind(config: &Config, inputs: &[InputBinding]) -> Result<Vec<Binding>> {
    let mut bindings = Vec::new();
    for (alias, output) in &config.outputs {
        let mut matching = inputs.iter().filter(|input| {
            input.output.component.as_str() == output.component
                && input.output.port.as_str() == output.port
        });
        if let Some(input) = matching.next() {
            anyhow::ensure!(
                matching.all(|other| other.stream == input.stream),
                "output {alias} is declared with more than one stream"
            );
            bindings.push(Binding {
                alias: alias.clone(),
                endpoint: input.output.clone(),
                stream: input.stream.clone(),
            });
        }
    }
    Ok(bindings)
}

/// One graph change as a row change keyed by element identity.
pub(crate) fn row_change(change: &GraphChange) -> Result<RowChange> {
    let reference = change.reference();
    let key = (
        reference.namespace.to_string(),
        reference.element_id.to_string(),
    );
    let after = || -> Result<Row> {
        match change.after_json()? {
            Some(Value::Object(row)) => Ok(row),
            Some(_) => anyhow::bail!("graph change properties are not an object"),
            None => anyhow::bail!("graph change has no after image"),
        }
    };
    Ok(match change.kind() {
        GraphChangeKind::Add | GraphChangeKind::Replace => RowChange::Upsert(key, after()?),
        GraphChangeKind::Patch => RowChange::Patch {
            key,
            set: after()?,
            removed: change.removed_properties().iter().cloned().collect(),
        },
        GraphChangeKind::Delete => RowChange::Delete(key),
    })
}

struct Loader {
    binding: Binding,
    reload: Arc<Notify>,
}

pub(crate) struct Running {
    cancel: Arc<Notify>,
    stopped: Arc<std::sync::atomic::AtomicBool>,
    server: Option<JoinHandle<()>>,
    loaders: Vec<JoinHandle<()>>,
    reloads: BTreeMap<String, Arc<Notify>>,
}

impl Running {
    fn cancel(&self) {
        self.stopped
            .store(true, std::sync::atomic::Ordering::Release);
        self.cancel.notify_waiters();
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.cancel();
        if let Some(server) = &self.server {
            server.abort();
        }
        for loader in &self.loaders {
            loader.abort();
        }
    }
}

pub(crate) struct ExporterSink {
    descriptor: ComponentDescriptor,
    config: Config,
    bindings: Vec<Binding>,
    snapshots: Option<NativeOutputSnapshots>,
    pub(crate) shared: Arc<Shared>,
    pub(crate) running: Option<Running>,
    paused: bool,
}

impl ExporterSink {
    pub(crate) fn new(
        descriptor: ComponentDescriptor,
        config: Config,
        inputs: &[InputBinding],
        snapshots: Option<NativeOutputSnapshots>,
    ) -> Result<Self> {
        let bindings = bind(&config, inputs)?;
        let shared = Shared::new(Exporter::new(
            &config,
            bindings.iter().map(|binding| binding.alias.as_str()),
        ));
        Ok(Self {
            descriptor,
            config,
            bindings,
            snapshots,
            shared,
            running: None,
            paused: false,
        })
    }

    fn alias_for(&self, stream: &StreamId) -> Option<&Binding> {
        self.bindings
            .iter()
            .find(|binding| &binding.stream == stream)
    }
}

async fn sleep_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

#[async_trait]
impl NativeComponent for ExporterSink {
    fn descriptor(&self) -> &ComponentDescriptor {
        &self.descriptor
    }
    fn configuration(&self) -> Result<Value> {
        Ok(serde_json::to_value(&self.config)?)
    }
    async fn initialize(&mut self, _ctx: &InitializeContext) -> LifecycleResult {
        Ok(())
    }
    /// Binds the listener, resets every attached output to Loading (the sink
    /// is volatile) and starts one snapshot loader per attached output.
    async fn start(&mut self, ctx: &mut StartContext) -> LifecycleResult {
        if self.running.is_some() {
            return Err(anyhow::anyhow!("prometheus exporter is already started").into());
        }
        if ctx.cancellation().is_cancelled() {
            return Err(LifecycleError::cancelled());
        }
        let address = self.config.listen;
        let listener = tokio::select! {
            biased;
            _ = ctx.cancellation().cancelled() => return Err(LifecycleError::cancelled()),
            _ = sleep_until(ctx.deadline()) => return Err(LifecycleError::retryable(anyhow::anyhow!(
                "metrics listener bind did not complete before the start deadline"
            ))),
            listener = TcpListener::bind(address) => listener
                .with_context(|| format!("bind prometheus exporter listener to {address}"))?,
        };
        *self.shared.lock() = Exporter::new(
            &self.config,
            self.bindings.iter().map(|binding| binding.alias.as_str()),
        );
        let cancel = Arc::new(Notify::new());
        let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let server = tokio::spawn(serve(
            listener,
            router(&self.config.path, self.shared.clone()),
            cancel.clone(),
            stopped.clone(),
        ));
        let mut reloads = BTreeMap::new();
        let mut loaders = Vec::new();
        if self.snapshots.is_none() && !self.bindings.is_empty() {
            warn!(
                "component={} has no output snapshot capability; its outputs stay not ready",
                self.descriptor.id()
            );
        }
        for binding in &self.bindings {
            let reload = Arc::new(Notify::new());
            reloads.insert(binding.alias.clone(), reload.clone());
            if let Some(snapshots) = &self.snapshots {
                loaders.push(tokio::spawn(load(
                    Loader {
                        binding: binding.clone(),
                        reload,
                    },
                    snapshots.clone(),
                    self.shared.clone(),
                    cancel.clone(),
                    stopped.clone(),
                )));
            }
        }
        self.paused = false;
        self.running = Some(Running {
            cancel,
            stopped,
            server: Some(server),
            loaders,
            reloads,
        });
        Ok(())
    }
    /// The host delivers no input while paused; scrapes keep answering with
    /// the rows as of the pause.
    async fn pause(&mut self, _ctx: &PauseContext) -> LifecycleResult {
        if self.running.is_none() {
            return Err(anyhow::anyhow!("prometheus exporter is not running").into());
        }
        self.paused = true;
        Ok(())
    }
    async fn resume(&mut self, _ctx: &ResumeContext) -> LifecycleResult {
        if self.running.is_none() {
            return Err(anyhow::anyhow!("prometheus exporter is not running").into());
        }
        self.paused = false;
        Ok(())
    }
    async fn stop(&mut self, ctx: &StopContext) -> LifecycleResult {
        let Some(running) = &mut self.running else {
            return Ok(());
        };
        running.cancel();
        if ctx.mode() == StopMode::Abort {
            if let Some(server) = &running.server {
                server.abort();
            }
        }
        for loader in &running.loaders {
            loader.abort();
        }
        let budget = ctx.deadline().map_or(DEFAULT_STOP_BUDGET, |deadline| {
            deadline.saturating_duration_since(Instant::now())
        });
        if let Some(server) = running.server.as_mut() {
            match tokio::time::timeout(budget, &mut *server).await {
                Err(_) => {
                    return Err(LifecycleError::retryable(anyhow::anyhow!(
                        "metrics listener is still draining"
                    )))
                }
                Ok(Err(error)) if !error.is_cancelled() => {
                    warn!("metrics listener task failed: {error}");
                }
                Ok(_) => {}
            }
        }
        running.server = None;
        for loader in std::mem::take(&mut running.loaders) {
            let _ = loader.await;
        }
        self.running = None;
        Ok(())
    }
    /// Volatile: the sink owns no persistent state.
    async fn dispose(&mut self, _ctx: &DisposeContext) -> LifecycleResult {
        if self.running.is_some() {
            return Err(anyhow::anyhow!("dispose requires a stopped prometheus exporter").into());
        }
        Ok(())
    }
}

#[async_trait]
impl NativeSink for ExporterSink {
    fn completion(&self) -> SinkCompletion {
        SinkCompletion::Accepted
    }
    async fn handle(&mut self, input: InputEnvelope) -> Result<()> {
        let running = self
            .running
            .as_ref()
            .context("prometheus exporter has not started")?;
        anyhow::ensure!(
            !running.stopped.load(std::sync::atomic::Ordering::Acquire),
            "prometheus exporter listener stopped; restart required"
        );
        anyhow::ensure!(!self.paused, "prometheus exporter is paused");
        anyhow::ensure!(
            input.port.as_str() == crate::IN,
            "unexpected sink input port"
        );
        self.shared
            .handled
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let system = input.envelope.system();
        let Some(binding) = self.alias_for(system.stream()) else {
            // An edge no alias names produces no metrics.
            return Ok(());
        };
        let alias = binding.alias.clone();
        let decoded = GraphChangeBatch::from_envelope(&input.envelope)
            .map_err(anyhow::Error::from)
            .and_then(|batch| {
                batch
                    .changes()
                    .iter()
                    .map(row_change)
                    .collect::<Result<Vec<_>>>()
            });
        let outcome = match decoded {
            Ok(changes) => self.shared.lock().apply(&alias, system.sequence(), changes),
            Err(error) => {
                // Export nothing for this output until its snapshot reloads,
                // rather than a series that may be wrong.
                warn!("output {alias}: input not understood, reloading: {error:#}");
                self.shared.lock().fail(&alias);
                Applied::ReloadNeeded
            }
        };
        if outcome == Applied::ReloadNeeded {
            if let Some(reload) = running.reloads.get(&alias) {
                reload.notify_one();
            }
        }
        Ok(())
    }
}

/// Reads the output's snapshot (retrying until it is available), installs it,
/// then waits for a reload request.
async fn load(
    loader: Loader,
    snapshots: NativeOutputSnapshots,
    shared: Arc<Shared>,
    cancel: Arc<Notify>,
    stopped: Arc<std::sync::atomic::AtomicBool>,
) {
    let alias = &loader.binding.alias;
    let schema = GraphChangeCodec::schema();
    let mut delay = RETRY_MIN;
    let mut warned = false;
    loop {
        if stopped.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }
        let generation = {
            let mut exporter = shared.lock();
            match exporter.loading(alias) {
                Some(generation) => generation,
                None => match exporter.begin_load(alias) {
                    Some(generation) => generation,
                    None => return,
                },
            }
        };
        let attempt = async {
            let snapshot = snapshots
                .snapshot(&loader.binding.endpoint, &schema)
                .await?;
            let sequence = snapshot.cursor.sequence();
            let mut rows = Vec::new();
            let mut records = snapshot.records;
            let mut ordinal = 0;
            while let Some(record) = records.next().await {
                let change = GraphChange::from_operation(&ChangeOperation::Added {
                    ordinal,
                    after: record?,
                })?;
                ordinal += 1;
                match row_change(&change)? {
                    RowChange::Upsert(key, row) => rows.push((key, row)),
                    _ => anyhow::bail!("snapshot record is not a full row"),
                }
            }
            anyhow::Ok((sequence, rows))
        };
        let outcome = tokio::select! {
            _ = cancel.notified() => return,
            outcome = attempt => outcome,
        };
        let loaded = match outcome {
            Ok((sequence, rows)) => shared
                .lock()
                .complete_load(alias, generation, sequence, rows),
            Err(error) => {
                if !warned {
                    warn!("output {alias}: snapshot not available yet, retrying: {error:#}");
                    warned = true;
                }
                tokio::select! {
                    _ = cancel.notified() => return,
                    _ = tokio::time::sleep(delay) => {}
                }
                delay = (delay * 2).min(RETRY_MAX);
                continue;
            }
        };
        match loaded {
            Loaded::Ready => {
                delay = RETRY_MIN;
                warned = false;
                tokio::select! {
                    _ = cancel.notified() => return,
                    _ = loader.reload.notified() => {}
                }
            }
            Loaded::Retry => {
                shared.lock().begin_load(alias);
            }
        }
    }
}

pub(crate) fn router(path: &str, shared: Arc<Shared>) -> Router {
    Router::new().route(path, get(metrics)).with_state(shared)
}

async fn metrics(State(shared): State<Arc<Shared>>) -> Response {
    let body = shared.lock().render();
    let mut response = (StatusCode::OK, body).into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE));
    response
}

async fn serve(
    listener: TcpListener,
    router: Router,
    cancel: Arc<Notify>,
    stopped: Arc<std::sync::atomic::AtomicBool>,
) {
    let shutdown = {
        let stopped = stopped.clone();
        async move {
            loop {
                let notified = cancel.notified();
                if stopped.load(std::sync::atomic::Ordering::Acquire) {
                    return;
                }
                notified.await;
            }
        }
    };
    if let Err(error) = axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
    {
        warn!("metrics listener failed: {error}");
    }
    stopped.store(true, std::sync::atomic::Ordering::Release);
}
