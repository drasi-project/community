#!/usr/bin/env python3
"""Verify real HTTP -> OTel -> Grafana/Langfuse and Grafana alert transitions."""
import argparse
import base64
from datetime import datetime, timezone
import json
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ops"))
import settings  # noqa: E402
import shlex
import subprocess
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def environment():
    result = {}
    for line in (ROOT / ".env").read_text().splitlines():
        if line.strip() and not line.startswith("#"):
            key, value = line.split("=", 1)
            tokens = shlex.split(value)
            result[key] = tokens[0] if tokens else ""
    return result


def get(url, auth=None, expected=200):
    headers = {"Authorization": f"Basic {auth}"} if auth else {}
    try:
        response = urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=10)
    except urllib.error.HTTPError as error:
        if error.code != expected:
            raise
        response = error
    with response:
        assert response.code == expected, (url, response.code, expected)
        return json.load(response)


def wait_for(label, check, timeout=120):
    start = time.monotonic()
    last_error = None
    while time.monotonic() - start < timeout:
        try:
            value = check()
            if value:
                return value, round(time.monotonic() - start, 3)
        except (urllib.error.URLError, TimeoutError, ConnectionError) as error:
            last_error = error
        time.sleep(1)
    raise AssertionError(f"{label} not observed in {timeout}s; last transport error: {last_error}")


class AlertSubscription:
    """Observe snapshot-first query deltas before producing this alert cycle."""
    def __init__(self):
        self.lock = threading.Lock()
        self.snapshot = None
        self.events = []
        self.error = None
        self.stopping = False
        self.process = None

    def start(self):
        url = settings.url("DRASI_API")+"/api/v1/instances/ignite/queries/grafana-alerts/attach"
        self.process = subprocess.Popen(
            ["curl", "--silent", "--show-error", "--fail", "--no-buffer", "--max-time", "600", url],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1,
        )
        self.thread = threading.Thread(target=self.read, daemon=True)
        self.thread.start()
        try:
            wait_for("Drasi initial query snapshot", self.ready, timeout=60)
        except (AssertionError, RuntimeError):
            self.close()
            raise

    def ready(self):
        with self.lock:
            if self.error:
                raise RuntimeError(self.error)
            return self.snapshot is not None

    def read(self):
        event, data = "message", []
        try:
            for line in self.process.stdout:
                line = line.rstrip("\r\n")
                if not line:
                    if data:
                        self.consume(event, json.loads("\n".join(data)))
                    event, data = "message", []
                elif line.startswith("event:"):
                    event = line[6:].strip()
                elif line.startswith("data:"):
                    data.append(line[5:].lstrip())
            if not self.stopping:
                with self.lock:
                    self.error = "Drasi query subscription ended before the cycle completed: " + self.process.stderr.read()
        except (ValueError, KeyError, TypeError, OSError) as exc:
            with self.lock:
                self.error = f"Invalid Drasi query stream: {exc}"

    def consume(self, event, payload):
        with self.lock:
            if event == "snapshot":
                if not isinstance(payload, list):
                    raise ValueError("snapshot must be a row array")
                self.snapshot = payload
            elif isinstance(payload, dict) and "results" in payload:
                for change in payload["results"]:
                    if change["type"] not in ("ADD", "UPDATE"):
                        continue
                    row = change.get("after", change.get("data"))
                    if not isinstance(row, dict):
                        raise ValueError("ADD/UPDATE must contain a row")
                    self.events.append({
                        "event": change["type"], "row": row,
                        "observed_at": datetime.now(timezone.utc).isoformat(),
                        "_monotonic": time.monotonic(),
                    })

    def matching(self, notification, after):
        with self.lock:
            if self.error:
                raise RuntimeError(self.error)
            for event in self.events:
                row = event["row"]
                if (
                    event["_monotonic"] >= after
                    and row.get("id") == notification["groupKey"]
                    and row.get("status") == notification["status"]
                    and row.get("title") == notification["title"]
                    and row.get("message") == notification["message"]
                ):
                    return {key: value for key, value in event.items() if key != "_monotonic"}
            return None

    def close(self):
        self.stopping = True
        if self.process is not None:
            if self.process.poll() is None:
                self.process.terminate()
            self.process.wait(timeout=10)
            self.thread.join(timeout=10)
            self.process.stdout.close()
            self.process.stderr.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--require-drasi", action="store_true", help="Require actual native acknowledgements AND query-visible firing/resolved deltas")
    args = parser.parse_args()
    env = environment()
    grafana_auth = base64.b64encode(f"admin:{env['GRAFANA_ADMIN_PASSWORD']}".encode()).decode()
    lf_auth = env["LANGFUSE_AUTH_BASE64"]
    probe = settings.url("PROBE")
    evidence = {"started_at": datetime.now(timezone.utc).isoformat(), "probe": True}
    for label, url in [
        ("probe", f"{probe}/health"),
        ("Grafana", settings.url("GRAFANA")+"/api/health"),
        ("Langfuse", settings.url("LANGFUSE")+"/api/public/health"),
    ]:
        wait_for(label, lambda url=url: get(url), timeout=240)
    dashboard = get(settings.url("GRAFANA")+"/api/dashboards/uid/ignite-integration-probe", grafana_auth)
    assert dashboard["dashboard"]["uid"] == "ignite-integration-probe"
    query = urllib.parse.urlencode({"query": "ignite_probe_http_requests_total"})
    metric_url = settings.url("GRAFANA")+f"/api/datasources/proxy/uid/ignite-prometheus/api/v1/query?{query}"
    # A probe restart resets its counters; wait out old collector/scrape samples.
    warmup = get(f"{probe}/probe?outcome=ok")
    previous_count = warmup["status_request_count"]
    def warmup_seen():
        series = get(metric_url, grafana_auth).get("data", {}).get("result", [])
        return sum(float(s["value"][1]) for s in series if s["metric"].get("http_response_status_code") == "200") == previous_count
    wait_for("fresh process counter baseline", warmup_seen)
    request_started = time.monotonic()
    success = get(f"{probe}/probe?outcome=ok")
    evidence["success_request"] = success

    def metric_seen():
        response = get(metric_url, grafana_auth)
        series = response.get("data", {}).get("result", [])
        successes = [s for s in series if s["metric"].get("http_response_status_code") == "200"]
        return successes if sum(float(s["value"][1]) for s in successes) > previous_count else None

    evidence["grafana_metric"], evidence["grafana_metric_wait_seconds"] = wait_for("Grafana real request count", metric_seen)
    evidence["request_to_grafana_observed_seconds"] = round(time.monotonic() - request_started, 3)
    trace_query = urllib.parse.urlencode({"traceId": success["trace_id"], "fields": "core,basic,metadata", "limit": 10})
    trace_url = settings.url("LANGFUSE")+f"/api/public/v2/observations?{trace_query}"

    def trace_seen():
        response = get(trace_url, lf_auth)
        return [
            row for row in response.get("data", [])
            if row.get("traceId") == success["trace_id"]
            and row.get("metadata", {}).get("request_id") == success["request_id"]
            and row.get("metadata", {}).get("synthetic") is True
        ]

    evidence["langfuse_observations"], evidence["langfuse_wait_seconds"] = wait_for("correlated Langfuse observation", trace_seen)
    evidence["request_to_langfuse_observed_seconds"] = round(time.monotonic() - request_started, 3)
    events = get(f"{probe}/events")
    baseline = max((event["id"] for event in events), default=0)

    def notification(status):
        matching = [
            event for event in get(f"{probe}/events")
            if event["id"] > baseline
            and event["payload"]["status"] == status
            and event["payload"].get("commonLabels", {}).get("alertname") == "Ignite probe HTTP failure"
            and (not args.require_drasi or event["forwarded_at"] is not None)
        ]
        return matching[0] if matching else None

    subscription = AlertSubscription() if args.require_drasi else None
    try:
        if subscription:
            subscription.start()
            evidence["drasi_initial_snapshot"] = subscription.snapshot
        failure_started = time.monotonic()
        evidence["failure_request"] = get(f"{probe}/probe?outcome=fail", expected=503)
        evidence["firing"], evidence["firing_wait_seconds"] = wait_for("real Grafana firing webhook", lambda: notification("firing"))
        if subscription:
            evidence["drasi_firing"], evidence["drasi_firing_query_wait_seconds"] = wait_for(
                "query-visible firing for this Grafana cycle",
                lambda: subscription.matching(evidence["firing"]["payload"], failure_started),
            )
        recovery_started = time.monotonic()
        evidence["recovery_request"] = get(f"{probe}/probe?outcome=ok")
        evidence["resolved"], evidence["resolved_wait_seconds"] = wait_for("real Grafana resolved webhook", lambda: notification("resolved"))
        if subscription:
            evidence["drasi_resolved"], evidence["drasi_resolved_query_wait_seconds"] = wait_for(
                "query-visible resolution for this Grafana cycle",
                lambda: subscription.matching(evidence["resolved"]["payload"], recovery_started),
            )
    finally:
        if subscription:
            subscription.close()
    assert evidence["firing"]["payload"]["groupKey"] == evidence["resolved"]["payload"]["groupKey"]
    evidence["drasi_delivery_required"] = args.require_drasi
    evidence["finished_at"] = datetime.now(timezone.utc).isoformat()
    out = ROOT / "observability/evidence"
    out.mkdir(exist_ok=True)
    path = out / ("smoke-drasi.json" if args.require_drasi else "smoke.json")
    path.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps({
        "result": "pass",
        "trace_id": success["trace_id"],
        "grafana_metric_wait_seconds": evidence["grafana_metric_wait_seconds"],
        "langfuse_wait_seconds": evidence["langfuse_wait_seconds"],
        "firing_wait_seconds": evidence["firing_wait_seconds"],
        "resolved_wait_seconds": evidence["resolved_wait_seconds"],
        "drasi_delivered": args.require_drasi,
        "drasi_query_visible": args.require_drasi,
        "evidence": str(path.relative_to(ROOT)),
    }, indent=2))


if __name__ == "__main__":
    main()
