use anyhow::Result;
use opentelemetry::{
    metrics::{Counter, Gauge, Histogram},
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
use std::time::Duration;

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
    pub failure: Gauge<u64>,
    /// The health monitor's readings, exported as `deployment_health_probe_*` gauges.
    pub probes: crate::probe::Probes,
    _probe_gauges: crate::probe::Gauges,
}

impl Telemetry {
    pub fn new() -> Result<Self> {
        let resource = Resource::builder()
            .with_service_name("ai-gateway")
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
        let meter = metrics.meter("ai.workload");
        let probes = crate::probe::Probes::default();
        let probe_gauges = probes.register(&meter);
        Ok(Self {
            tracer: traces.tracer("ai.workload"),
            requests: meter.u64_counter("ai_gateway_model_calls").build(),
            retries: meter.u64_counter("ai_gateway_retries").build(),
            tokens: meter.u64_counter("ai_gateway_tokens").build(),
            completed: meter.u64_counter("ai_gateway_requests_completed").build(),
            duration: meter
                .f64_histogram("ai_gateway_model_call_duration")
                .with_unit("s")
                .with_boundaries(vec![
                    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0,
                ])
                .build(),
            answer: meter
                .f64_histogram("ai_gateway_answer_time")
                .with_description("Time from a request being received to its answer")
                .with_unit("s")
                .with_boundaries(vec![
                    0.1, 0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0, 45.0, 60.0,
                    90.0, 120.0,
                ])
                .build(),
            queue: meter.u64_gauge("ai_gateway_queue_depth").build(),
            failure: meter
                .u64_gauge("ai_gateway_last_request_failed")
                .build(),
            probes,
            _probe_gauges: probe_gauges,
            traces,
            metrics,
        })
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
            KeyValue::new("langfuse.trace.metadata.request_id", task.to_owned()),
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
