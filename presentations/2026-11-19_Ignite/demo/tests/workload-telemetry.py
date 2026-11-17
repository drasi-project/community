#!/usr/bin/env python3
"""Require actual workload metrics, correlated Langfuse spans and new Grafana transitions."""
import base64
import importlib.util
import json
import time
import urllib.parse
from pathlib import Path

DEMO = Path(__file__).resolve().parents[1]


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, DEMO / "tests" / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


work = load("workload", "workload-integration.py")
obs = load("observability", "observability-smoke.py")


def main():
    env = obs.environment()
    auth = base64.b64encode(f"admin:{env['GRAFANA_ADMIN_PASSWORD']}".encode()).decode()
    dashboard = obs.get("http://127.0.0.1:13000/api/dashboards/uid/ignite-workload", auth)
    assert {"Support errors", "Support queue", "Support overdue",
            "Recovery"} <= {panel["title"] for panel in dashboard["dashboard"]["panels"]}
    metric_url = "http://127.0.0.1:13000/api/datasources/proxy/uid/ignite-prometheus/api/v1/query?" + urllib.parse.urlencode({
        "query": 'sum(ignite_workload_requests_total{workload="support",endpoint="EU-A",status="503"})'})

    def metric():
        rows = obs.get(metric_url, auth)["data"]["result"]
        return float(rows[0]["value"][1]) if rows else 0

    # Finish a successful request before the new measured incident, so previous alert state resolves.
    work.control("restore")
    warm, _ = work.submit()
    work.wait(lambda: work.api("/tasks/" + warm["id"])["task"]["completed_ms"])
    time.sleep(20)
    alerts = work.native.Attachment("alert-context")
    evidence = {"role": "actual-support-workload", "dashboard_uid": "ignite-workload"}
    try:
        baseline = metric()
        work.control("fail")
        started = int(time.time() * 1000)
        body, trace = work.submit()
        evidence.update({"task_id": body["id"], "trace": trace, "started_ms": started})
        _, evidence["metric_wait_seconds"] = obs.wait_for("new measured workload HTTP503", lambda: metric() > baseline)
        firing = alerts.until(lambda rows: any(
            r["status"] == "firing" and r["event_ms"] >= started and "Model calls failing" in r["title"]
            for r in rows.values()), timeout=100)
        row = next(r for r in firing.values() if r["status"]=="firing" and r["event_ms"]>=started and "Model calls failing" in r["title"])
        evidence["firing"] = row
        firing_payload = json.loads(work.native.runtime.sql(
            f"SELECT payload FROM ignite_alerts WHERE id='{row['id']}';",tuples=True))
        evidence["firing_wait_seconds"] = round((time.time()*1000-started)/1000,3)
        query = urllib.parse.urlencode({"traceId":trace["trace_id"],"fields":"core,basic,metadata","limit":100})
        url = "http://127.0.0.1:13001/api/public/v2/observations?" + query

        def correlated():
            return [r for r in obs.get(url,env["LANGFUSE_AUTH_BASE64"]).get("data",[])
                    if r.get("traceId")==trace["trace_id"] and r.get("metadata",{}).get("task_id")==body["id"]]

        evidence["observations"], evidence["trace_wait_seconds"] = obs.wait_for("exact Langfuse task/trace correlation",correlated)
        assert any(r["name"].startswith("attempt · ") for r in evidence["observations"])
        assert any(r["name"]=="request received" and r["id"]==trace["root_span_id"] and r["isRootObservation"] for r in evidence["observations"])
        work.control("restore")
        work.wait(lambda: work.api("/tasks/" + body["id"])["task"]["completed_ms"])
        alerts.until(lambda rows: rows.get(row["id"],{}).get("status")=="resolved"
                     and rows[row["id"]]["event_ms"]>row["event_ms"],timeout=100)
        evidence["resolved"] = alerts.rows[row["id"]]
        # Replay the actual captured older notification, twice; no fabricated alert payload.
        work.api("/alerts/grafana",firing_payload)
        work.api("/alerts/grafana",firing_payload)
        replayed = work.native.Attachment("alert-context")
        try:
            assert replayed.rows[row["id"]]["status"]=="resolved"
            assert replayed.rows[row["id"]]["event_ms"]==evidence["resolved"]["event_ms"]
        finally:
            replayed.close()
        evidence["older_duplicate_firing_rejected"] = True
        evidence["query_events"] = alerts.history
        path = DEMO / ".native/evidence/workload-telemetry.json"
        path.write_text(json.dumps(evidence,indent=2))
        print(json.dumps({"passed":"real measured workload -> Grafana metric/firing/resolved -> durable Drasi CQ; exact Langfuse task trace",
                          "task_id":body["id"], "trace_id":trace["trace_id"],"evidence":str(path)},indent=2))
    finally:
        alerts.close()
        work.control("restore")


if __name__ == "__main__":
    main()
