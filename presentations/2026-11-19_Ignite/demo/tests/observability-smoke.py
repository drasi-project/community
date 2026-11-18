#!/usr/bin/env python3
"""Verify real HTTP -> OTel -> Grafana/Langfuse and Grafana alert transitions (Grafana's own webhook)."""
import base64
from datetime import datetime, timezone
import json
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ops"))
import settings  # noqa: E402
import shlex
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


# plan/contracts.md section 16: the projector highlights panels by these keys.
PANEL_KEYS = ["panel-menus", "ai-deployments", "model-calls", "support-errors", "answer-time", "support-queue", "alerts-firing",
              "model-calls-chart", "support-waiting", "decision-support", "support-overdue", "deployment-status",
              "workload-status", "recommended-action", "failover-readiness"]


def operations_dashboard(auth):
    """One dashboard with the contract's panel keys, a description on every panel, the Recovery plan
    dashboard gone, and Grafana's alerts posting to #incident-ai-support."""
    grafana = settings.url("GRAFANA")
    panels = get(grafana + "/api/dashboards/uid/ignite-workload", auth)["dashboard"]["panels"]
    assert [p.get("key") for p in panels] == PANEL_KEYS, [p.get("key") for p in panels]
    assert all(p.get("description") for p in panels if p["type"] != "row"), "a panel has no description"
    get(grafana + "/api/dashboards/uid/ignite-decisions", auth, expected=404)
    receivers = {r["name"]: r for r in get(grafana + "/api/v1/provisioning/contact-points", auth)}
    channel = receivers.get("incident-channel")
    assert channel and channel["type"] == "slack" and channel["settings"].get("username") == "Grafana", channel
    routes = get(grafana + "/api/v1/provisioning/policies", auth).get("routes", [])
    assert any(r.get("receiver") == "incident-channel" and r.get("continue") for r in routes), routes
    return {"panels": len(panels), "contact_point": "incident-channel"}


def main():
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
    evidence["operations_dashboard"] = operations_dashboard(grafana_auth)
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
        ]
        return matching[0] if matching else None

    evidence["failure_request"] = get(f"{probe}/probe?outcome=fail", expected=503)
    evidence["firing"], evidence["firing_wait_seconds"] = wait_for("real Grafana firing webhook", lambda: notification("firing"))
    evidence["recovery_request"] = get(f"{probe}/probe?outcome=ok")
    evidence["resolved"], evidence["resolved_wait_seconds"] = wait_for("real Grafana resolved webhook", lambda: notification("resolved"))
    assert evidence["firing"]["payload"]["groupKey"] == evidence["resolved"]["payload"]["groupKey"]
    evidence["finished_at"] = datetime.now(timezone.utc).isoformat()
    out = ROOT / "observability/evidence"
    out.mkdir(exist_ok=True)
    path = out / "smoke.json"
    path.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps({
        "result": "pass",
        "trace_id": success["trace_id"],
        "grafana_metric_wait_seconds": evidence["grafana_metric_wait_seconds"],
        "langfuse_wait_seconds": evidence["langfuse_wait_seconds"],
        "firing_wait_seconds": evidence["firing_wait_seconds"],
        "resolved_wait_seconds": evidence["resolved_wait_seconds"],
        "evidence": str(path.relative_to(ROOT)),
    }, indent=2))


if __name__ == "__main__":
    main()
