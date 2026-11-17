#!/usr/bin/env python3
"""Live gate for the telemetry loop around Drasi.

The application's own telemetry (request metrics, Langfuse traces) is the AI service's job; the
overdue count is Drasi's (its Prometheus reaction). Grafana alerts on both and its contact point
posts to Drasi's HTTP source, so each alert is read back from Drasi's `grafana-alerts` query
(plan/contracts.md sections 4 and 9). Leaves EU-A restored; needs the running services."""
import json
import time
import urllib.parse

import drasi_outputs as d
from drasi_outputs import settings

FAILING = "Model calls failing"
OVERDUE = "Support requests overdue (Drasi)"


def alert(rows, title, status):
    return next((r for r in rows.values() if r["status"] == status and title in r["title"]
                 and "DatasourceNoData" not in r["title"]), None)


def main():
    auth = d.grafana_auth()
    dashboard = d.get(settings.url("GRAFANA") + "/api/dashboards/uid/ignite-workload", auth)
    assert {"Support errors", "Support queue", "Support overdue", "Recovery"} <= {
        panel["title"] for panel in dashboard["dashboard"]["panels"]}
    errors = settings.url("GRAFANA") + "/api/datasources/proxy/uid/ignite-prometheus/api/v1/query?" + \
        urllib.parse.urlencode({"query": 'sum(ignite_workload_requests_total{workload="support",endpoint="EU-A",status="503"})'})

    def measured():
        rows = d.get(errors, auth)["data"]["result"]
        return float(rows[0]["value"][1]) if rows else 0

    d.reset()
    alerts = d.Attachment("grafana-alerts")
    evidence = {"dashboard_uid": "ignite-workload"}
    try:
        # Start from resolved alerts so the firing below is new.
        warm, _ = d.submit()
        d.wait(lambda: d.task(warm["id"])["task"]["completed_ms"], 20, "a warm-up answer")
        alerts.until(lambda rows: not alert(rows, FAILING, "firing") and not alert(rows, OVERDUE, "firing"), 120)
        baseline = measured()
        d.control("fail")
        started = time.time()
        body, trace = d.submit()
        evidence.update({"task_id": body["id"], "trace": trace})
        d.wait(lambda: measured() > baseline, 60, "the application's 503 metric in Grafana's Prometheus")
        alerts.until(lambda rows: alert(rows, FAILING, "firing"), 100)
        evidence["failing"] = alert(alerts.rows, FAILING, "firing")
        evidence["failing_seconds"] = round(time.time() - started, 1)

        d.metric_until("ignite_drasi_overdue_requests", lambda v: v and v >= 1, 40, workload="support")
        scraped = d.wait(lambda: [r for r in d.prometheus('ignite_drasi_overdue_requests{workload="support"} >= 1')],
                         20, "Prometheus scraped Drasi's overdue count")
        assert scraped[0]["metric"].get("job") == "drasi", scraped
        alerts.until(lambda rows: alert(rows, OVERDUE, "firing"), 100)
        evidence["overdue"] = alert(alerts.rows, OVERDUE, "firing")
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
        alerts.until(lambda rows: alert(rows, FAILING, "resolved") and alert(rows, OVERDUE, "resolved")
                     and not alert(rows, FAILING, "firing") and not alert(rows, OVERDUE, "firing"), 150)
        evidence["resolved"] = [alert(alerts.rows, FAILING, "resolved"), alert(alerts.rows, OVERDUE, "resolved")]
        evidence["query_events"] = alerts.history
        path = d.save("workload-telemetry.json", evidence)
        print(json.dumps({"passed": [
            "the application's measured 503s fire Grafana's 'Model calls failing'; Grafana posts it to Drasi's "
            "HTTP source and Drasi's grafana-alerts query holds it",
            "Drasi's overdue count (Prometheus reaction, scraped as job drasi) fires 'Support requests "
            "overdue (Drasi)', which also arrives back in Drasi",
            "the request's Langfuse trace has its attempts and root span",
            "restoring EU-A completes the request; Drasi's count returns to 0 and both alerts resolve in Drasi"],
            "task_id": body["id"], "evidence": str(path)}, indent=2))
    finally:
        alerts.close()
        d.control("restore")


if __name__ == "__main__":
    main()
