#!/usr/bin/env python3
"""Live gate for the telemetry around Drasi.

The application's own telemetry (request metrics, Langfuse traces) is the AI service's job; the
overdue count is Drasi's (its Prometheus reaction, plan/contracts.md section 9). Grafana alerts on
both; the alerts stay in Grafana. Leaves EU-A restored; needs the running services."""
import json
import time
import urllib.parse

import drasi_outputs as d
from drasi_outputs import settings

FAILING = "Model calls failing"
OVERDUE = "Support requests overdue (Drasi)"


def firing(title):
    """Grafana's own alert instances for one rule that are firing ("Alerting")."""
    alerts = d.get(settings.url("GRAFANA") + "/api/prometheus/grafana/api/v1/alerts", d.grafana_auth())
    return [a for a in alerts["data"]["alerts"] if a["labels"].get("alertname") == title and a["state"] == "Alerting"]


def main():
    auth = d.grafana_auth()
    dashboard = d.get(settings.url("GRAFANA") + "/api/dashboards/uid/ignite-workload", auth)
    assert {"Support errors", "Support queue", "Support overdue", "Change status"} <= {
        panel["title"] for panel in dashboard["dashboard"]["panels"]}
    errors = settings.url("GRAFANA") + "/api/datasources/proxy/uid/ignite-prometheus/api/v1/query?" + \
        urllib.parse.urlencode({"query": 'sum(ignite_workload_requests_total{workload="support",endpoint="EU-A",status="503"})'})

    def measured():
        rows = d.get(errors, auth)["data"]["result"]
        return float(rows[0]["value"][1]) if rows else 0

    d.reset()
    evidence = {"dashboard_uid": "ignite-workload"}
    try:
        # Start from resolved alerts so the firing below is new.
        warm, _ = d.submit()
        d.wait(lambda: d.task(warm["id"])["task"]["completed_ms"], 20, "a warm-up answer")
        d.wait(lambda: not firing(FAILING) and not firing(OVERDUE), 120, "no alerts firing", 2)
        baseline = measured()
        d.control("fail")
        started = time.time()
        body, trace = d.submit()
        evidence.update({"task_id": body["id"], "trace": trace})
        d.wait(lambda: measured() > baseline, 60, "the application's 503 metric in Grafana's Prometheus")
        evidence["failing"] = d.wait(lambda: firing(FAILING), 100, "Grafana fires 'Model calls failing'", 2)
        evidence["failing_seconds"] = round(time.time() - started, 1)

        d.metric_until("ignite_drasi_overdue_requests", lambda v: v and v >= 1, 40, workload="support")
        scraped = d.wait(lambda: [r for r in d.prometheus('ignite_drasi_overdue_requests{workload="support"} >= 1')],
                         20, "Prometheus scraped Drasi's overdue count")
        assert scraped[0]["metric"].get("job") == "drasi", scraped
        evidence["overdue"] = d.wait(lambda: firing(OVERDUE), 100, "Grafana fires the overdue alert", 2)
        evidence["overdue_seconds"] = round(time.time() - started, 1)

        d.control("restore")
        d.wait(lambda: d.task(body["id"])["task"]["completed_ms"], 30, "the request completes on EU-A")
        d.metric_until("ignite_drasi_overdue_requests", lambda v: not v, 20, workload="support")
        url = settings.url("LANGFUSE") + "/api/public/v2/observations?" + urllib.parse.urlencode(
            {"traceId": trace["trace_id"], "fields": "core,basic,metadata", "limit": 100})

        def correlated():
            rows = [r for r in d.get(url, d.ENV["LANGFUSE_AUTH_BASE64"]).get("data", [])
                    if r.get("traceId") == trace["trace_id"]]
            attempts = [r for r in rows if r["name"].startswith("attempt · ")
                        and r.get("metadata", {}).get("task_id") == body["id"]]
            root = [r for r in rows if r["name"] == "request received" and r["id"] == trace["root_span_id"]
                    and r["isRootObservation"]]
            return rows if attempts and root else None

        evidence["observations"] = d.wait(correlated, 90, "Langfuse attempts and root span of the request", 2)
        d.wait(lambda: not firing(FAILING) and not firing(OVERDUE), 150, "both alerts resolve", 2)
        path = d.save("workload-telemetry.json", evidence)
        print(json.dumps({"passed": [
            "the application's measured 503s fire Grafana's 'Model calls failing'",
            "Drasi's overdue count (Prometheus reaction, scraped as job drasi) fires Grafana's 'Support "
            "requests overdue (Drasi)'",
            "the request's Langfuse trace has its attempts and root span",
            "restoring EU-A completes the request; Drasi's count returns to 0 and both alerts resolve"],
            "task_id": body["id"], "evidence": str(path)}, indent=2))
    finally:
        d.control("restore")


if __name__ == "__main__":
    main()
