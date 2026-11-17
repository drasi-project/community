use anyhow::Result;
use opentelemetry::{
    metrics::{Counter, Gauge, Histogram, ObservableGauge},
    trace::{
        Span, SpanContext, SpanId, SpanKind, TraceContextExt, TraceFlags, TraceId, TraceState,
        Tracer, TracerProvider,
    },
    Context, KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::{SdkTracer, SdkTracerProvider},
    Resource,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

/// Latest dashboard projection: (view, workload, endpoint, value, valid until in seconds).
type Projection = Arc<Mutex<Vec<(&'static str, String, String, i64, i64)>>>;

pub struct Telemetry {
    traces: SdkTracerProvider,
    metrics: SdkMeterProvider,
    pub tracer: SdkTracer,
    pub requests: Counter<u64>,
    pub retries: Counter<u64>,
    pub tokens: Counter<u64>,
    pub completed: Counter<u64>,
    pub duration: Histogram<f64>,
    pub answer: Histogram<f64>,
    pub queue: Gauge<u64>,
    pub capacity: Gauge<u64>,
    pub failure: Gauge<u64>,
    pub assessment: Gauge<u64>,
    projection: Projection,
    _dashboard: ObservableGauge<i64>,
    pub dashboard_timestamp: Gauge<i64>,
    _dashboard_valid_until: ObservableGauge<i64>,
}

impl Telemetry {
    pub fn new() -> Result<Self> {
        let resource = Resource::builder()
            .with_service_name("ignite-workload")
            .build();
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(format!("{}/v1/traces", ignite_settings::get().url("OTLP")?))
            .build()?;
        let traces = SdkTracerProvider::builder()
            .with_resource(resource.clone())
            .with_batch_exporter(exporter)
            .build();
        let exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_endpoint(format!("{}/v1/metrics", ignite_settings::get().url("OTLP")?))
            .build()?;
        let reader = PeriodicReader::builder(exporter)
            .with_interval(Duration::from_secs(1))
            .build();
        let metrics = SdkMeterProvider::builder()
            .with_resource(resource)
            .with_reader(reader)
            .build();
        use opentelemetry::metrics::MeterProvider;
        let meter = metrics.meter("ignite.workload");
        // Observable gauges report only the latest projection, so a label set that is no longer
        // current (such as the deployment named by an infeasible resilience result) disappears.
        let projection = Projection::default();
        let current = projection.clone();
        let dashboard = meter
            .i64_observable_gauge("ignite_drasi_dashboard")
            .with_callback(move |observer| {
                for (view, workload, endpoint, value, _) in current.lock().unwrap().iter() {
                    observer.observe(
                        *value,
                        &[
                            KeyValue::new("view", *view),
                            KeyValue::new("workload", workload.clone()),
                            KeyValue::new("endpoint", endpoint.clone()),
                        ],
                    );
                }
            })
            .build();
        let current = projection.clone();
        let dashboard_valid_until = meter
            .i64_observable_gauge("ignite_drasi_valid_until_seconds")
            .with_callback(move |observer| {
                let views: BTreeMap<_, _> = current
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(view, _, _, _, until)| (*view, *until))
                    .collect();
                for (view, until) in views {
                    observer.observe(until, &[KeyValue::new("view", view)]);
                }
            })
            .build();
        Ok(Self {
            tracer: traces.tracer("ignite.workload"),
            requests: meter.u64_counter("ignite_workload_requests").build(),
            retries: meter.u64_counter("ignite_workload_retries").build(),
            tokens: meter.u64_counter("ignite_workload_tokens").build(),
            completed: meter.u64_counter("ignite_workload_completed").build(),
            duration: meter
                .f64_histogram("ignite_workload_call_duration")
                .with_unit("s")
                .with_boundaries(vec![
                    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0,
                ])
                .build(),
            answer: meter
                .f64_histogram("ignite_workload_answer")
                .with_description("Time from a request being received to its answer")
                .with_unit("s")
                .with_boundaries(vec![
                    0.1, 0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0, 45.0, 60.0,
                    90.0, 120.0,
                ])
                .build(),
            queue: meter.u64_gauge("ignite_workload_queue_depth").build(),
            capacity: meter.u64_gauge("ignite_workload_capacity_tpm").build(),
            failure: meter
                .u64_gauge("ignite_workload_last_request_failed")
                .build(),
            assessment: meter.u64_gauge("ignite_drasi_assessment").build(),
            projection,
            _dashboard: dashboard,
            dashboard_timestamp: meter
                .i64_gauge("ignite_drasi_projection_timestamp_seconds")
                .build(),
            _dashboard_valid_until: dashboard_valid_until,
            traces,
            metrics,
        })
    }

    pub fn project(&self, samples: Vec<crate::dashboard::Sample>) {
        *self.projection.lock().unwrap() = samples
            .into_iter()
            .map(|s| {
                (
                    s.view,
                    s.workload,
                    s.endpoint,
                    s.value,
                    s.valid_until_ms / 1000,
                )
            })
            .collect();
    }

    pub fn span(
        &self,
        name: impl Into<std::borrow::Cow<'static, str>>,
        trace: &str,
        root_span: &str,
        task: &str,
        workload: &str,
    ) -> Result<opentelemetry_sdk::trace::Span> {
        let parent = Context::new().with_remote_span_context(SpanContext::new(
            TraceId::from_hex(trace)?,
            SpanId::from_hex(root_span)?,
            TraceFlags::SAMPLED,
            true,
            TraceState::default(),
        ));
        let mut span = self
            .tracer
            .span_builder(name)
            .with_kind(SpanKind::Internal)
            .start_with_context(&self.tracer, &parent);
        Self::decorate(&mut span, task, workload);
        Ok(span)
    }

    pub fn root(&self, task: &str, workload: &str) -> opentelemetry_sdk::trace::Span {
        let mut span = self
            .tracer
            .span_builder("request received")
            .with_kind(SpanKind::Internal)
            .start_with_context(&self.tracer, &Context::new());
        Self::decorate(&mut span, task, workload);
        span
    }

    fn decorate(span: &mut impl Span, task: &str, workload: &str) {
        span.set_attributes([
            KeyValue::new("task.id", task.to_owned()),
            KeyValue::new("workload.id", workload.to_owned()),
            KeyValue::new(
                "langfuse.trace.name",
                if workload == "support" {
                    "Customer support request"
                } else {
                    "Document summary request"
                },
            ),
            KeyValue::new("langfuse.trace.metadata.task_id", task.to_owned()),
            KeyValue::new("langfuse.trace.metadata.workload", workload.to_owned()),
            KeyValue::new("langfuse.trace.metadata.synthetic", true),
        ]);
    }

    pub fn shutdown(&self) -> Result<()> {
        self.traces.shutdown()?;
        self.metrics.shutdown()?;
        Ok(())
    }
}
