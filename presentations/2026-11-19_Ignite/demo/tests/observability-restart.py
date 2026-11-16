#!/usr/bin/env python3
"""Verify persisted trace/webhook/metric history and fresh delivery after restart."""
import argparse
import base64
from datetime import datetime
import json
from pathlib import Path
import runpy
import subprocess
import sys
import time
import urllib.parse

ROOT = Path(__file__).resolve().parents[1]
smoke = runpy.run_path(str(ROOT / "tests/observability-smoke.py"))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--include-postgres", action="store_true", help="Also restart the native demo's shared PostgreSQL; coordinate first")
    parser.add_argument("--require-drasi", action="store_true")
    parser.add_argument("--history-only", action="store_true", help="Leave fresh traffic to a subsequent strict smoke after the native Server is restarted")
    args = parser.parse_args()
    if args.history_only and args.require_drasi:
        parser.error("--history-only cannot claim --require-drasi")
    old = json.loads((ROOT / "observability/evidence/smoke.json").read_text())
    env = smoke["environment"]()
    services = ["redis", "minio", "clickhouse", "langfuse-web", "langfuse-worker", "collector", "prometheus", "grafana", "probe"]
    if args.include_postgres:
        services.insert(0, "postgres")
    helper = str(ROOT / "ops/observability-stack.sh")
    started = time.monotonic()
    subprocess.run([helper, "stop", *services], check=True)
    subprocess.run([helper, "up", "-d", "--no-deps", *services], check=True)
    get, wait_for = smoke["get"], smoke["wait_for"]
    wait_for("restarted Langfuse", lambda: get("http://127.0.0.1:13001/api/public/health"), timeout=240)
    wait_for("restarted Grafana", lambda: get("http://127.0.0.1:13000/api/health"))
    wait_for("restarted probe", lambda: get("http://127.0.0.1:18888/health"))
    startup_seconds = round(time.monotonic() - started, 3)
    trace_id = old["success_request"]["trace_id"]
    observations = get(
        "http://127.0.0.1:13001/api/public/v2/observations?" + urllib.parse.urlencode({"traceId": trace_id}),
        env["LANGFUSE_AUTH_BASE64"],
    )["data"]
    assert any(row["traceId"] == trace_id for row in observations), "Historical trace lost"
    events = get("http://127.0.0.1:18888/events")
    assert any(event["id"] == old["firing"]["id"] for event in events), "Historical webhook lost"
    auth = base64.b64encode(f"admin:{env['GRAFANA_ADMIN_PASSWORD']}".encode()).decode()
    params = urllib.parse.urlencode({
        "query": "ignite_probe_http_requests_total",
        "start": datetime.fromisoformat(old["started_at"]).timestamp(),
        "end": datetime.fromisoformat(old["finished_at"]).timestamp(),
        "step": "2",
    })
    history = get("http://127.0.0.1:13000/api/datasources/proxy/uid/ignite-prometheus/api/v1/query_range?" + params, auth)
    assert history["data"]["result"], "Historical metric samples lost"
    if not args.history_only:
        command = [sys.executable, str(ROOT / "tests/observability-smoke.py")]
        if args.require_drasi:
            command.append("--require-drasi")
        subprocess.run(command, check=True)
    result = {
        "result": "history-pass" if args.history_only else "pass",
        "services": services, "stop_to_ready_seconds": startup_seconds,
        "historical_trace_id": trace_id, "historical_webhook_id": old["firing"]["id"],
        "historical_metric_series": len(history["data"]["result"]),
        "fresh_delivery_verified": not args.history_only,
    }
    (ROOT / "observability/evidence/restart.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
