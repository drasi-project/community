#!/usr/bin/env python3
"""Live gate for the application and the simulated deployments, as Drasi sees them.

The AI service serves requests, retries and reports health; Drasi derives deployment status,
overdue requests and their history from those rows (desk-stream SSE reaction, Prometheus
reaction, Server's query results). Resets this copy's demo; needs the running services."""
import json
import subprocess

import drasi_outputs as d


def report(deployment):
    return d.sql(f"SELECT row_to_json(r) FROM ignite_reports r WHERE id='{deployment}';")


def main():
    checks = []
    d.reset()
    stream = d.DeskStream(["overdue-tasks"])
    history = d.Attachment("deadline-history")
    try:
        for deployment in ["EU-A", "EU-B", "US-C"]:
            d.metric_until("ignite_drasi_deployment_status", lambda v: v == 0, deployment=deployment)
        checks.append("every simulated deployment's health report reaches Drasi: all Healthy")

        for workload, expected in [("support", "EU-A"), ("batch", "EU-B")]:
            body, result = d.submit(workload)
            d.wait(lambda: d.task(body["id"])["task"]["completed_ms"], 20, f"{workload} answer")
            task = d.task(body["id"])["task"]
            assert task["endpoint"] == expected and task["trace_id"] == result["trace_id"], task
            duplicate = d.api("/tasks", body)
            assert duplicate["duplicate"] and duplicate["trace_id"] == result["trace_id"]
            d.api("/tasks", {**body, "content": "different"}, expected=409)
        checks.append("support and summaries answered on their configured routes; submissions are idempotent")

        epoch = d.health()["run_epoch"]
        elsewhere = {"id": f"{epoch}-elsewhere", "run_epoch": epoch, "workload": "support",
                     "content": "SYNTHETIC RESTRICTED never transmitted", "destination": "US-C"}
        d.api("/tasks", elsewhere, expected=409)
        assert d.sql(f"SELECT count(*) FROM ignite_tasks WHERE id='{elsewhere['id']}';") == "0"
        checks.append("the gateway sends only to the configured route (no per-request destination)")

        d.control("throttle", "EU-B")
        d.metric_until("ignite_drasi_deployment_status", lambda v: v == 1, deployment="EU-B")
        throttled, _ = d.submit("batch")
        d.wait(lambda: any(a["status"] == 429 for a in d.task(throttled["id"])["attempts"]), 20, "HTTP 429")
        d.control("restore", "EU-B")
        d.metric_until("ignite_drasi_deployment_status", lambda v: v == 0, deployment="EU-B")
        d.wait(lambda: d.task(throttled["id"])["task"]["completed_ms"], 30, "retried summary")
        checks.append("EU-B over quota: Drasi shows 'Over quota' from its reports; the gateway retries the 429")

        d.control("fail")
        body, _ = d.submit()
        d.wait(lambda: any(a["status"] == 503 for a in d.task(body["id"])["attempts"]), 20, "HTTP 503")
        before = d.task(body["id"])["task"]
        stream.until("overdue-tasks", lambda rows: any(r["id"] == body["id"] for r in rows), 30)
        history.until(lambda rows: body["id"] in rows)
        assert d.task(body["id"])["task"] == before
        checks.append("a request failing on EU-A becomes overdue and enters Drasi's deadline history through "
                      "Drasi's timer; the request row itself never changes")

        d.control("pause-reports", "EU-B")
        d.metric_until("ignite_drasi_deployment_status", lambda v: v == -1, 25, deployment="EU-B")
        last = report("EU-B")
        for statement in ["UPDATE ignite_reports SET sequence=sequence-1 WHERE id='EU-B';",
                          "UPDATE ignite_reports SET sequence=sequence+1,observed_ms=observed_ms-1 WHERE id='EU-B';",
                          "UPDATE ignite_scenario SET revision=revision+2 WHERE singleton;"]:
            try:
                d.sql(statement)
                raise AssertionError(f"out-of-order change accepted: {statement}")
            except subprocess.CalledProcessError as error:
                assert "non-monotonic" in error.stderr, error.stderr
        assert report("EU-B") == last
        checks.append("EU-B stops reporting: its last report is unchanged and Drasi's timer turns it Unknown; "
                      "out-of-order reports and revisions are refused by the database")

        d.control("restore")
        d.control("resume-reports", "EU-B")
        d.wait(lambda: d.task(body["id"])["task"]["completed_ms"], 30, "the restored request completes")
        stream.until("overdue-tasks", lambda rows: all(r["id"] != body["id"] for r in rows))
        d.metric_until("ignite_drasi_deployment_status", lambda v: v == 0, 25, deployment="EU-B")
        assert body["id"] in history.rows
        again = d.Attachment("deadline-history")
        assert body["id"] in again.rows
        again.close()
        checks.append("restoring EU-A completes the request: it leaves Drasi's overdue list and stays in its "
                      "deadline history, also on a new attach")

        assert d.sql("SELECT count(*) FROM ignite_receipts WHERE workload='support' AND endpoint='US-C';") == "0"
        checks.append("no restricted support request ever reached US-C")
        path = d.save("workload.json", {"checks": checks, "stream": stream.history, "history": history.history})
        print(json.dumps({"passed": checks, "evidence": str(path)}, indent=2))
    finally:
        stream.close()
        history.close()
        for deployment in ["EU-A", "EU-B", "US-C"]:
            d.control("restore", deployment)
            d.control("resume-reports", deployment)


if __name__ == "__main__":
    main()
