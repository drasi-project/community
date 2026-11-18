//! The health monitor's probe readings as OpenTelemetry metrics, for Drasi's OTel source.
//!
//! Each reading is exported as observable gauges with the attributes `deployment` and
//! `run_epoch`; the Drasi source's `metricIdentityAttributes: [deployment, run_epoch]`
//! makes one Metric node per deployment and run. Observable gauges export with delta
//! temporality (only what the callback observed), so a deployment whose reporting is
//! paused stops being exported and Drasi sees the silence. A synchronous gauge would
//! re-export its last value forever.
use opentelemetry::{metrics::Meter, KeyValue};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Gauge names; the Drasi source allowlists `deployment_health_probe_*`.
pub const STATUS: &str = "deployment_health_probe_status";
pub const UP: &str = "deployment_health_probe_up";
pub const LATENCY: &str = "deployment_health_probe_latency_ms";

/// A reading is exported while it is younger than this: the monitor probes every second
/// and the reader exports every second, so each reading is exported once or twice.
const FRESH: Duration = Duration::from_millis(1_500);

/// `deployment_health_probe_status` values, matching the tile's status codes.
pub fn status_code(status: &str) -> Option<i64> {
    match status {
        "ok" => Some(0),
        "throttled" => Some(1),
        "failed" => Some(2),
        "unknown" => Some(3),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Reading {
    run_epoch: String,
    status: i64,
    latency_ms: f64,
    at: Instant,
}

/// The latest probe reading per deployment.
#[derive(Clone, Default)]
pub struct Probes(Arc<Mutex<BTreeMap<String, Reading>>>);

impl Probes {
    pub fn record(&self, deployment: &str, run_epoch: &str, status: i64, latency: Duration) {
        self.record_at(deployment, run_epoch, status, latency, Instant::now());
    }

    fn record_at(
        &self,
        deployment: &str,
        run_epoch: &str,
        status: i64,
        latency: Duration,
        at: Instant,
    ) {
        self.0.lock().expect("probe readings").insert(
            deployment.into(),
            Reading {
                run_epoch: run_epoch.into(),
                status,
                latency_ms: latency.as_secs_f64() * 1000.0,
                at,
            },
        );
    }

    /// Readings fresh at `now`: (attributes, status, latency in ms).
    fn fresh(&self, now: Instant) -> Vec<([KeyValue; 2], i64, f64)> {
        self.0
            .lock()
            .expect("probe readings")
            .iter()
            .filter(|(_, r)| now.saturating_duration_since(r.at) < FRESH)
            .map(|(deployment, r)| {
                (
                    [
                        KeyValue::new("deployment", deployment.clone()),
                        KeyValue::new("run_epoch", r.run_epoch.clone()),
                    ],
                    r.status,
                    r.latency_ms,
                )
            })
            .collect()
    }

    /// Registers the probe gauges on `meter`. The returned instruments must be kept alive.
    pub fn register(&self, meter: &Meter) -> Gauges {
        let (status, up, latency) = (self.clone(), self.clone(), self.clone());
        Gauges {
            _status: meter
                .i64_observable_gauge(STATUS)
                .with_description("Health probe status: 0 ok, 1 throttled, 2 failed, 3 unknown")
                .with_callback(move |o| {
                    for (attributes, code, _) in status.fresh(Instant::now()) {
                        o.observe(code, &attributes);
                    }
                })
                .build(),
            _up: meter
                .i64_observable_gauge(UP)
                .with_description("1 when the health probe found the deployment ok")
                .with_callback(move |o| {
                    for (attributes, code, _) in up.fresh(Instant::now()) {
                        o.observe(i64::from(code == 0), &attributes);
                    }
                })
                .build(),
            _latency: meter
                .f64_observable_gauge(LATENCY)
                .with_unit("ms")
                .with_description("Health probe round trip")
                .with_callback(move |o| {
                    for (attributes, _, ms) in latency.fresh(Instant::now()) {
                        o.observe(ms, &attributes);
                    }
                })
                .build(),
        }
    }
}

pub struct Gauges {
    _status: opentelemetry::metrics::ObservableGauge<i64>,
    _up: opentelemetry::metrics::ObservableGauge<i64>,
    _latency: opentelemetry::metrics::ObservableGauge<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::metrics::MeterProvider as _;
    use opentelemetry_sdk::{
        error::OTelSdkResult,
        metrics::{
            data::{AggregatedMetrics, MetricData, ResourceMetrics},
            exporter::PushMetricExporter,
            PeriodicReader, SdkMeterProvider, Temporality,
        },
    };

    type Point = (String, String, String, f64);

    /// Captures each export's points: (metric, deployment, run_epoch, value). Cumulative
    /// temporality, as the OTLP exporter uses by default.
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<Vec<Point>>>>);
    impl PushMetricExporter for Capture {
        async fn export(&self, rm: &ResourceMetrics) -> OTelSdkResult {
            let mut points = Vec::new();
            for scope in rm.scope_metrics() {
                for metric in scope.metrics() {
                    let mut push = |attributes: Vec<&KeyValue>, value: f64| {
                        let get = |key: &str| {
                            attributes
                                .iter()
                                .find(|kv| kv.key.as_str() == key)
                                .map(|kv| kv.value.to_string())
                                .unwrap_or_default()
                        };
                        points.push((
                            metric.name().to_string(),
                            get("deployment"),
                            get("run_epoch"),
                            value,
                        ));
                    };
                    match metric.data() {
                        AggregatedMetrics::I64(MetricData::Gauge(g)) => {
                            for p in g.data_points() {
                                push(p.attributes().collect(), p.value() as f64);
                            }
                        }
                        AggregatedMetrics::F64(MetricData::Gauge(g)) => {
                            for p in g.data_points() {
                                push(p.attributes().collect(), p.value());
                            }
                        }
                        other => panic!("unexpected aggregation {other:?}"),
                    }
                }
            }
            points.sort_by(|a, b| a.partial_cmp(b).unwrap());
            self.0.lock().unwrap().push(points);
            Ok(())
        }
        fn force_flush(&self) -> OTelSdkResult {
            Ok(())
        }
        fn shutdown_with_timeout(&self, _: Duration) -> OTelSdkResult {
            Ok(())
        }
        fn temporality(&self) -> Temporality {
            Temporality::Cumulative
        }
    }

    struct Harness {
        probes: Probes,
        capture: Capture,
        provider: SdkMeterProvider,
        _gauges: Gauges,
    }
    impl Harness {
        /// One collection and export, as the periodic reader does every second.
        fn collect(&self) -> Vec<Point> {
            self.provider.force_flush().unwrap();
            self.capture.0.lock().unwrap().pop().unwrap_or_default()
        }
    }

    fn setup() -> Harness {
        let capture = Capture::default();
        let reader = PeriodicReader::builder(capture.clone())
            .with_interval(Duration::from_secs(3600))
            .build();
        let provider = SdkMeterProvider::builder().with_reader(reader).build();
        let probes = Probes::default();
        let gauges = probes.register(&provider.meter("test"));
        Harness {
            probes,
            capture,
            provider,
            _gauges: gauges,
        }
    }

    #[test]
    fn status_codes_cover_every_report_status() {
        let codes: Vec<_> = ["ok", "throttled", "failed", "unknown"]
            .iter()
            .map(|s| status_code(s))
            .collect();
        assert_eq!(codes, [Some(0), Some(1), Some(2), Some(3)]);
        assert_eq!(status_code("exploded"), None);
    }

    #[test]
    fn exports_one_series_per_deployment_with_its_run() {
        let h = setup();
        let probes = &h.probes;
        probes.record("EU-A", "run-1", 2, Duration::from_millis(12));
        probes.record("EU-B", "run-1", 0, Duration::from_millis(5));
        let points = h.collect();
        let status: Vec<_> = points.iter().filter(|p| p.0 == STATUS).collect();
        assert_eq!(
            status,
            [
                &(STATUS.to_string(), "EU-A".into(), "run-1".into(), 2.0),
                &(STATUS.to_string(), "EU-B".into(), "run-1".into(), 0.0)
            ]
        );
        let up: Vec<_> = points
            .iter()
            .filter(|p| p.0 == UP)
            .map(|p| (p.1.as_str(), p.3))
            .collect();
        assert_eq!(up, [("EU-A", 0.0), ("EU-B", 1.0)]);
        let latency: Vec<_> = points
            .iter()
            .filter(|p| p.0 == LATENCY)
            .map(|p| (p.1.as_str(), p.3))
            .collect();
        assert_eq!(latency, [("EU-A", 12.0), ("EU-B", 5.0)]);
    }

    #[test]
    fn a_silent_deployment_stops_being_exported() {
        let h = setup();
        let probes = &h.probes;
        let stale = Instant::now() - FRESH - Duration::from_millis(1);
        probes.record_at("EU-A", "run-1", 0, Duration::ZERO, stale);
        probes.record("EU-B", "run-1", 0, Duration::ZERO);
        let deployments: Vec<_> = h.collect().into_iter().map(|p| p.1).collect();
        assert!(deployments.iter().all(|d| d == "EU-B"), "{deployments:?}");
        // Delta temporality: nothing new observed, nothing re-exported.
        probes.0.lock().unwrap().clear();
        assert!(h.collect().is_empty());
    }

    #[test]
    fn a_new_reading_replaces_the_last_and_a_new_run_is_a_new_series() {
        let h = setup();
        let probes = &h.probes;
        probes.record("EU-A", "run-1", 0, Duration::ZERO);
        probes.record("EU-A", "run-1", 2, Duration::ZERO);
        let status: Vec<_> = h.collect().into_iter().filter(|p| p.0 == STATUS).collect();
        assert_eq!(
            status,
            [(STATUS.to_string(), "EU-A".into(), "run-1".into(), 2.0)]
        );
        probes.record("EU-A", "run-2", 0, Duration::ZERO);
        let status: Vec<_> = h.collect().into_iter().filter(|p| p.0 == STATUS).collect();
        assert_eq!(
            status,
            [(STATUS.to_string(), "EU-A".into(), "run-2".into(), 0.0)]
        );
    }
}
