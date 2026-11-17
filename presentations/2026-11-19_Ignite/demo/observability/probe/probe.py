"""Small HTTP integration probe; not the Ignite workload or gateway."""
import hashlib
import json
import logging
import os
from pathlib import Path
import signal
import sqlite3
import threading
import time
import urllib.parse
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from opentelemetry import metrics, trace
from opentelemetry.exporter.otlp.proto.http.metric_exporter import OTLPMetricExporter
from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter
from opentelemetry.metrics import Observation
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.trace import SpanKind, Status, StatusCode

logging.basicConfig(level=logging.INFO)
RESOURCE = Resource.create({"service.name": "ignite-integration-probe", "service.version": "phase-1"})
TRACES = TracerProvider(resource=RESOURCE)
TRACES.add_span_processor(BatchSpanProcessor(OTLPSpanExporter(), schedule_delay_millis=500))
trace.set_tracer_provider(TRACES)
METRICS = MeterProvider(
    resource=RESOURCE,
    metric_readers=[PeriodicExportingMetricReader(OTLPMetricExporter(), export_interval_millis=1000)],
)
metrics.set_meter_provider(METRICS)
TRACER = trace.get_tracer("ignite.integration-probe")
METER = metrics.get_meter("ignite.integration-probe")
REQUESTS = METER.create_counter("ignite_probe_http_requests", unit="{request}")
DURATION = METER.create_histogram("ignite_probe_http_duration", unit="s")
LOCK = threading.Lock()
WEBHOOK_LOCK = threading.Lock()
LAST_FAILED = None
STATUS_COUNTS = {200: 0, 503: 0}


def last_failure(_options):
    with LOCK:
        return [] if LAST_FAILED is None else [Observation(LAST_FAILED)]


METER.create_observable_gauge("ignite_probe_last_request_failed", callbacks=[last_failure])
DATABASE = Path(os.environ.get("PROBE_DATABASE", "/data/webhooks.sqlite"))
DATABASE.parent.mkdir(parents=True, exist_ok=True)
with sqlite3.connect(DATABASE) as db:
    db.execute("""CREATE TABLE IF NOT EXISTS webhooks (
        id INTEGER PRIMARY KEY, digest TEXT UNIQUE, received_at REAL,
        payload TEXT NOT NULL, forwarded_at REAL, delivery_error TEXT)""")


def record_webhook(payload):
    """Record one Grafana notification (deduplicated by content) for the smoke test."""
    encoded = json.dumps(payload, sort_keys=True)
    digest = hashlib.sha256(encoded.encode()).hexdigest()
    with WEBHOOK_LOCK, sqlite3.connect(DATABASE) as db:
        db.execute(
            "INSERT OR IGNORE INTO webhooks(digest, received_at, payload) VALUES (?, ?, ?)",
            (digest, time.time(), encoded),
        )
        return db.execute("SELECT id FROM webhooks WHERE digest=?", (digest,)).fetchone()[0]


class Handler(BaseHTTPRequestHandler):
    def reply(self, status, body):
        encoded = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def do_GET(self):
        global LAST_FAILED
        url = urllib.parse.urlsplit(self.path)
        if url.path == "/health":
            return self.reply(200, {"status": "ok", "role": "integration-probe"})
        if url.path == "/events":
            with sqlite3.connect(DATABASE) as db:
                db.row_factory = sqlite3.Row
                rows = db.execute("SELECT * FROM webhooks ORDER BY id DESC LIMIT 100").fetchall()
            return self.reply(200, [
                {**dict(row), "payload": json.loads(row["payload"])} for row in rows
            ])
        if url.path != "/probe":
            return self.reply(404, {"error": "not found"})
        outcome = urllib.parse.parse_qs(url.query).get("outcome", ["ok"])[0]
        if outcome not in ("ok", "fail"):
            return self.reply(400, {"error": "outcome must be ok or fail"})
        request_id = str(uuid.uuid4())
        status = 200 if outcome == "ok" else 503
        started = time.perf_counter()
        with TRACER.start_as_current_span("GET /probe", kind=SpanKind.SERVER) as span:
            span.set_attributes({
                "http.request.method": "GET",
                "http.route": "/probe",
                "http.response.status_code": status,
                "langfuse.trace.name": "Ignite integration probe",
                "langfuse.trace.metadata.request_id": request_id,
                "langfuse.trace.metadata.synthetic": True,
            })
            # Perform actual bounded local work; no fabricated latency or tokens.
            digest = hashlib.pbkdf2_hmac("sha256", b"synthetic-probe", b"ignite", 10000).hex()
            if status == 503:
                span.set_status(Status(StatusCode.ERROR, "Deliberate integration-probe HTTP failure"))
            duration = time.perf_counter() - started
            labels = {"http.response.status_code": status, "http.route": "/probe"}
            REQUESTS.add(1, labels)
            DURATION.record(duration, labels)
            with LOCK:
                LAST_FAILED = int(status == 503)
                STATUS_COUNTS[status] += 1
                status_request_count = STATUS_COUNTS[status]
            trace_id = f"{span.get_span_context().trace_id:032x}"
        self.reply(status, {
            "role": "integration-probe", "request_id": request_id, "trace_id": trace_id,
            "status": status, "duration_seconds": duration, "work_digest": digest,
            "status_request_count": status_request_count,
            # LANGFUSE_PUBLIC_URL is set by compose.yaml from ports.env.
            "trace_url": f"{os.environ.get('LANGFUSE_PUBLIC_URL', '')}/project/ignite-probe/traces/{trace_id}",
        })

    def do_POST(self):
        if self.path != "/alerts/grafana":
            return self.reply(404, {"error": "not found"})
        try:
            length = int(self.headers.get("Content-Length", "0"))
            if length <= 0 or length > 1024 * 1024:
                return self.reply(413, {"error": "expected JSON body under 1 MiB"})
            payload = json.loads(self.rfile.read(length))
            if not isinstance(payload, dict) or payload.get("status") not in ("firing", "resolved") or not payload.get("groupKey"):
                return self.reply(400, {"error": "expected native Grafana webhook status and groupKey"})
        except (ValueError, json.JSONDecodeError):
            return self.reply(400, {"error": "invalid JSON or content length"})
        self.reply(200, {"event_id": record_webhook(payload)})


if __name__ == "__main__":
    server = ThreadingHTTPServer(("0.0.0.0", 8888), Handler)
    signal.signal(signal.SIGTERM, lambda *_: threading.Thread(target=server.shutdown).start())
    try:
        server.serve_forever()
    finally:
        server.server_close()
        METRICS.shutdown()
        TRACES.shutdown()
