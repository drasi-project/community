//! SDK-level tests: factory, description, lifecycle conformance, input
//! handling through `NativeSink::handle`, and concurrent scrapes over HTTP.
use super::*;
use crate::component::{router, ExporterSink, Shared, CONTENT_TYPE};
use crate::state::{Exporter, RowChange};
use anyhow::Context;
use async_trait::async_trait;
use drasi_computation_plugin_sdk::testing::{contexts, create_sink, SinkConformance, SinkProbe};
use drasi_core_types::models::{
    Element, ElementMetadata, ElementPropertyMap, ElementReference, SourceChange,
};
use serde_json::{json, Value};
use std::{future::IntoFuture, sync::Mutex, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ID: &str = "268e23f4-4a61-59ee-8d8f-0ab7a4b9f5c1";
/// Producing components (IDs; the config generator maps tile names to IDs).
const OVERDUE: &str = "76b8e87b-4275-567c-a37f-d48582112ba1";
const ACTION: &str = "8a0f0b4e-5b7c-5d55-9a3e-0f7f5e3e4a10";
const STRAY: &str = "c1d2e3f4-0000-5000-8000-000000000001";
const WAIT: Duration = Duration::from_secs(5);

fn configuration() -> Value {
    json!({
        "listen": "127.0.0.1:0",
        "outputs": {
            "overdue": {"component": OVERDUE, "port": "out"},
            "action": {"component": ACTION}
        },
        "metrics": {
            "overdue": [{"metric": "drasi_requests_overdue", "help": "Overdue.", "value": "value", "labels": {"workload": "workload"}}],
            "action": [{"metric": "drasi_action_needed", "help": "Action.", "value": "value"}]
        }
    })
}

/// Only `tile-overdue` is attached; `action` has no edge, and `stray` is an
/// edge no alias names.
fn inputs() -> Vec<InputBinding> {
    [(OVERDUE, "tile-overdue/out"), (STRAY, "stray/out")]
        .into_iter()
        .map(|(component, stream)| InputBinding {
            input_port: PortId::try_new(IN).unwrap(),
            output: OutputEndpoint::new(
                ComponentId::try_new(component).unwrap(),
                PortId::try_new("out").unwrap(),
            ),
            stream: StreamId::try_new(stream).unwrap(),
        })
        .collect()
}

fn envelope(stream: &str, sequence: u64, change: SourceChange) -> InputEnvelope {
    InputEnvelope {
        port: PortId::try_new(IN).unwrap(),
        envelope: GraphChangeCodec::encode_change(
            change,
            StreamId::try_new(stream).unwrap(),
            sequence,
            None,
        )
        .unwrap(),
    }
}

fn node(id: &str, properties: Value) -> Element {
    Element::Node {
        metadata: ElementMetadata {
            reference: ElementReference::new(OVERDUE, id),
            labels: Arc::from([Arc::from("Row")]),
            effective_from: 1,
        },
        properties: ElementPropertyMap::from(properties),
    }
}

async fn scrape(address: std::net::SocketAddr, path: &str) -> anyhow::Result<(String, String)> {
    let mut stream = tokio::net::TcpStream::connect(address).await?;
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .await?;
    let mut response = String::new();
    tokio::time::timeout(WAIT, stream.read_to_string(&mut response)).await??;
    let (head, body) = response.split_once("\r\n\r\n").context("no header end")?;
    Ok((head.to_string(), body.to_string()))
}

#[test]
fn example_configuration_is_valid() {
    let example: Value = serde_json::from_str(include_str!("../examples/ignite.json")).unwrap();
    ExporterFactory
        .metadata()
        .configuration
        .validate(&example)
        .unwrap();
    let config = Config::parse(&example).unwrap();
    assert_eq!(config.outputs.len(), config.metrics.len());
    let mut bad = example.clone();
    bad["metrics"]["overdue"][0]["metric"] = json!("ignite-drasi");
    assert!(Config::parse(&bad).is_err());
    let mut bad = example.clone();
    bad["metrics"]["overdue"][0]["constLabels"] = json!({"workload": "x"});
    assert!(
        Config::parse(&bad).is_err(),
        "a label may not be both mapped and constant"
    );
    let mut bad = example.clone();
    bad["outputs"]["demand"] = json!({"component": "tile-capacity", "port": "out"});
    assert!(
        Config::parse(&bad).is_err(),
        "two aliases may not name one output"
    );
    let mut bad = example;
    bad["metrics"]["nope"] = json!([{"metric": "x", "help": "x", "value": "v"}]);
    assert!(Config::parse(&bad).is_err());
    // The configuration round-trips through serialization.
    let config = Config::parse(&configuration()).unwrap();
    let again = Config::parse(&serde_json::to_value(&config).unwrap()).unwrap();
    assert_eq!(format!("{config:?}"), format!("{again:?}"));
}

#[test]
fn factory_metadata_and_description() {
    plugin().unwrap();
    let metadata = ExporterFactory.metadata();
    assert_eq!(
        metadata.implementation,
        ImplementationIdentity::try_new(IMPLEMENTATION, "1").unwrap()
    );
    assert_eq!(metadata.role, ComponentRole::Sink);
    assert_eq!(metadata.completion, Some(SinkCompletion::Accepted));
    assert!(metadata.capabilities.binds_inputs);
    assert!(ExporterFactory.supports_output_snapshots());
    assert_eq!(metadata.ports.len(), 1);
}

/// Wraps the factory so the probe can observe the sink's handled count.
struct Observed {
    shared: Mutex<Option<Arc<Shared>>>,
}

impl Factory for Observed {
    fn describe(
        &self,
        request: &drasi_computation_plugin_sdk::wire::DescribeRequest,
    ) -> Result<drasi_computation_plugin_sdk::ComponentDescription> {
        ExporterFactory.describe(request)
    }
    fn metadata(&self) -> FactoryMetadata {
        ExporterFactory.metadata()
    }
    fn create(&self, request: &CreateRequest, _: ControlSender) -> Result<CreatedComponent> {
        let sink = ExporterSink::new(
            self.metadata().descriptor(request.id.clone())?,
            Config::parse(&request.configuration)?,
            &request.inputs,
            None,
        )?;
        *self.shared.lock().unwrap() = Some(sink.shared.clone());
        Ok(Component::Sink(Box::new(sink)).into())
    }
}

#[async_trait]
impl SinkProbe for Observed {
    fn input(&self, sequence: u64) -> Result<InputEnvelope> {
        Ok(envelope(
            "tile-overdue/out",
            sequence,
            SourceChange::Insert {
                element: node(
                    &format!("r{sequence}"),
                    json!({"workload": "support", "value": 1}),
                ),
            },
        ))
    }
    async fn expect_delivered(&self, _configuration: &Value, count: usize) -> Result<()> {
        let shared = self.shared.lock().unwrap().clone().context("not created")?;
        let deadline = tokio::time::Instant::now() + WAIT;
        while shared.handled.load(std::sync::atomic::Ordering::Acquire) < count as u64 {
            anyhow::ensure!(tokio::time::Instant::now() < deadline, "not delivered");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lifecycle_conformance() -> Result<()> {
    let factory = Observed {
        shared: Mutex::new(None),
    };
    SinkConformance::new(&factory, ID, inputs())?
        .lifecycle(configuration(), &factory)
        .await?;
    SinkConformance::new(&ExporterFactory, ID, inputs())?
        .erase(configuration())
        .await
}

/// Through the SDK path without a snapshot capability: outputs are never
/// bootstrapped, so only readiness 0 is exported, whatever input arrives.
#[tokio::test(flavor = "current_thread")]
async fn without_snapshots_outputs_stay_not_ready() -> Result<()> {
    let port = free_port();
    let mut config = configuration();
    config["listen"] = json!(format!("127.0.0.1:{port}"));
    let address: std::net::SocketAddr = format!("127.0.0.1:{port}").parse()?;
    let mut sink = create_sink(
        &ExporterFactory,
        &ComponentId::try_new(ID)?,
        config,
        &inputs(),
    )?;
    sink.initialize(&contexts::initialize()).await?;
    assert!(
        sink.handle(envelope(
            "tile-overdue/out",
            1,
            SourceChange::Insert {
                element: node("a", json!({"value": 1}))
            }
        ))
        .await
        .is_err(),
        "refused before start"
    );
    sink.start(&mut contexts::start()).await?;
    sink.handle(envelope(
        "tile-overdue/out",
        1,
        SourceChange::Insert {
            element: node("a", json!({"workload": "support", "value": 3})),
        },
    ))
    .await?;
    // An edge no alias names is accepted and ignored.
    sink.handle(envelope(
        "stray/out",
        1,
        SourceChange::Insert {
            element: node("b", json!({"value": 1})),
        },
    ))
    .await?;
    let (head, body) = scrape(address, "/metrics").await?;
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(
        head.to_ascii_lowercase()
            .contains(&format!("content-type: {CONTENT_TYPE}")),
        "{head}"
    );
    assert_eq!(
        body,
        "# HELP drasi_reaction_input_ready 1 once the output's snapshot is loaded and its changes are applied; 0 otherwise.\n\
         # TYPE drasi_reaction_input_ready gauge\n\
         drasi_reaction_input_ready{output=\"action\"} 0\n\
         drasi_reaction_input_ready{output=\"overdue\"} 0\n"
    );
    let (head, _) = scrape(address, "/other").await?;
    assert!(head.starts_with("HTTP/1.1 404"), "{head}");
    sink.stop(&contexts::stop(
        drasi_computation_plugin_sdk::lifecycle::StopMode::Drain,
    ))
    .await?;
    assert!(
        scrape(address, "/metrics").await.is_err(),
        "listener closed after stop"
    );
    Ok(())
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("free port")
}

/// Change decoding from real graph envelopes: insert, update (patch),
/// replace and delete, applied to a ready output.
#[test]
fn graph_changes_decode_to_row_changes() -> Result<()> {
    let config = Config::parse(&configuration())?;
    let mut exporter = Exporter::new(&config, ["overdue"]);
    let generation = exporter.loading("overdue").context("loading")?;
    exporter.complete_load("overdue", generation, 0, []);
    let changes = [
        SourceChange::Insert {
            element: node("a", json!({"workload": "support", "value": 1})),
        },
        SourceChange::update_with_removals(
            node("a", json!({"value": 5})),
            ["workload".to_string()].into(),
        )?,
        SourceChange::Replace {
            element: node("b", json!({"workload": "batch", "value": 2})),
        },
        SourceChange::Delete {
            metadata: ElementMetadata {
                reference: ElementReference::new(OVERDUE, "b"),
                labels: Arc::from([]),
                effective_from: 2,
            },
        },
    ];
    let mut kinds = Vec::new();
    for (sequence, change) in changes.into_iter().enumerate() {
        let input = envelope("tile-overdue/out", sequence as u64 + 1, change);
        let batch = GraphChangeBatch::from_envelope(&input.envelope)?;
        let rows = batch
            .changes()
            .iter()
            .map(component::row_change)
            .collect::<Result<Vec<_>>>()?;
        kinds.extend(rows.iter().map(|row| match row {
            RowChange::Upsert(..) => "upsert",
            RowChange::Patch { .. } => "patch",
            RowChange::Delete(..) => "delete",
        }));
        exporter.apply("overdue", sequence as u64 + 1, rows);
        if sequence == 2 {
            assert!(exporter
                .render()
                .contains("drasi_requests_overdue{workload=\"batch\"} 2\n"));
        }
    }
    assert_eq!(kinds, ["upsert", "patch", "upsert", "delete"]);
    let rows = exporter.rows("overdue").context("rows")?;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows.values().next(),
        Some(json!({"value": 5}).as_object().context("object")?)
    );
    assert!(exporter
        .render()
        .contains("drasi_requests_overdue{workload=\"\"} 5\n"));
    Ok(())
}

/// Many clients scrape while changes are applied: every response is a
/// complete, well-formed exposition with a consistent value.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_scrapes_under_load() -> Result<()> {
    let config = Config::parse(&configuration())?;
    let mut exporter = Exporter::new(&config, ["overdue", "action"]);
    for alias in ["overdue", "action"] {
        let generation = exporter.loading(alias).context("loading")?;
        exporter.complete_load(alias, generation, 0, []);
    }
    let shared = Shared::new(exporter);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server =
        tokio::spawn(axum::serve(listener, router("/metrics", shared.clone())).into_future());

    let writer = {
        let shared = shared.clone();
        tokio::spawn(async move {
            for sequence in 1..=500u64 {
                let rows = (0..20)
                    .map(|index| {
                        RowChange::Upsert(
                            ("q".into(), format!("w{index:02}")),
                            json!({"workload": format!("w{index:02}"), "value": sequence})
                                .as_object()
                                .unwrap()
                                .clone(),
                        )
                    })
                    .collect();
                shared.lock().apply("overdue", sequence, rows);
                shared.lock().apply(
                    "action",
                    sequence,
                    vec![RowChange::Upsert(
                        ("q".into(), "a".into()),
                        json!({"value": sequence % 4}).as_object().unwrap().clone(),
                    )],
                );
                tokio::task::yield_now().await;
            }
        })
    };
    let mut clients = Vec::new();
    for _ in 0..16 {
        clients.push(tokio::spawn(async move {
            for _ in 0..25 {
                let (head, body) = scrape(address, "/metrics").await?;
                anyhow::ensure!(head.starts_with("HTTP/1.1 200"), "{head}");
                anyhow::ensure!(
                    body.ends_with("drasi_reaction_input_ready{output=\"overdue\"} 1\n"),
                    "{body}"
                );
                let samples: Vec<&str> = body
                    .lines()
                    .filter(|line| line.starts_with("drasi_requests_overdue{"))
                    .collect();
                anyhow::ensure!(
                    samples.is_empty() || samples.len() == 20,
                    "partial family: {}",
                    samples.len()
                );
                // One envelope is applied atomically: all 20 rows agree.
                let values: std::collections::BTreeSet<&str> = samples
                    .iter()
                    .filter_map(|line| line.rsplit(' ').next())
                    .collect();
                anyhow::ensure!(values.len() <= 1, "torn update: {values:?}");
                for line in body.lines() {
                    if line.starts_with('#') {
                        anyhow::ensure!(
                            line.starts_with("# HELP ") || line.starts_with("# TYPE "),
                            "{line}"
                        );
                    } else {
                        let (_, value) = line.rsplit_once(' ').context("sample")?;
                        value
                            .parse::<f64>()
                            .with_context(|| format!("value in {line}"))?;
                    }
                }
            }
            anyhow::Ok(())
        }));
    }
    for client in clients {
        client.await??;
    }
    writer.await?;
    let (_, body) = scrape(address, "/metrics").await?;
    assert!(
        body.contains("drasi_requests_overdue{workload=\"w19\"} 500\n"),
        "{body}"
    );
    assert!(body.contains("drasi_action_needed 0\n"), "{body}");
    server.abort();
    Ok(())
}
