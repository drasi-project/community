#!/usr/bin/env python3
"""Live gate for deployment health from telemetry (IGNITE_HEALTH_SOURCE=otel).

The AI service's health monitor exports probe metrics; the Collector forwards them to Drasi's
OpenTelemetry source; Drasi derives Healthy, Failed and Unknown. Nothing about health goes
through PostgreSQL. Drives only the simulated deployments (/control) and reads every answer
from Drasi. Resets this copy's demo before and after; needs the running services (SETUP.md)
switched to telemetry (plan/otel-detection-plan.md). With IGNITE_HEALTH_SOURCE=postgres it
reports that it does not apply and exits 0.
"""
import sys
import time
import uuid

import drasi_outputs as d
import otlp

STATUS = "drasi_deployment_health_status"


def main():
    if d.settings.VALUES.get("IGNITE_HEALTH_SOURCE", "postgres") != "otel":
        print("health-telemetry-integration: not applicable, IGNITE_HEALTH_SOURCE is not otel")
        return 0
    checks, evidence = [], {}
    collector = f"http://127.0.0.1:{d.settings.port('OTLP')}/v1/metrics"
    d.reset()
    health = d.Attachment("deployment-health")
    try:
        epoch = d.health()["run_epoch"]
        rows = health.until(lambda rows: all(rows.get(dep, {}).get("run_epoch") == epoch
                                             and rows[dep]["status"] == "ok" for dep in ("EU-A", "EU-B", "US-C")))
        for deployment in ("EU-A", "EU-B", "US-C"):
            d.metric_until(STATUS, lambda v: v == 0, deployment=deployment)
        time.sleep(3)
        reported = d.sql(f"SELECT max(observed_ms) FROM deployment_health_reports WHERE run_epoch='{epoch}'")
        assert reported == "0", f"the health monitor still writes deployment_health_reports: {reported}"
        evidence["baseline"] = {k: rows[k] for k in ("EU-A", "EU-B", "US-C")}
        checks.append("baseline: every deployment Healthy from probe metrics alone; deployment_health_reports is not written")

        # The path itself: a synthetic probe through the Collector reaches deployment-health.
        test_run = f"test-{uuid.uuid4()}"
        assert otlp.probe(collector, "TEST-PROBE", "failed", test_run) == 200
        row = health.until(lambda rows: rows.get("TEST-PROBE", {}).get("run_epoch") == test_run)["TEST-PROBE"]
        assert row["status"] == "failed" and row["expires_ms"] == row["observed_ms"] + 8000, row
        checks.append("a probe sent to the Collector reaches Drasi's OTel source and deployment-health")

        # Detection: the deployment fails; Drasi sees it in the next probes, and the planner reacts.
        started = time.monotonic()
        d.control("fail")
        d.metric_until(STATUS, lambda v: v == 2, timeout=10, deployment="EU-A")
        evidence["detect_failed_s"] = round(time.monotonic() - started, 1)
        with d.DeskStream(["plan-results"]) as stream:
            plan = stream.until("plan-results", lambda r: r and r[0]["state"] == "feasible" and r[0]["moves"] > 0)[0]
        evidence["failure_plan"] = plan
        checks.append(f"EU-A fails: Failed in {evidence['detect_failed_s']} s; the plan moves work off EU-A")

        # Unknown: the deployment stops reporting; its probes stop and Drasi times it out.
        d.control("restore")
        d.metric_until(STATUS, lambda v: v == 0, timeout=10, deployment="EU-A")
        started = time.monotonic()
        d.control("pause-reports")
        d.metric_until(STATUS, lambda v: v == -1, timeout=15, deployment="EU-A")
        silent = round(time.monotonic() - started, 1)
        assert silent >= 6, f"Unknown after {silent} s: before the 8-second expiry"
        evidence["unknown_after_s"] = silent
        checks.append(f"EU-A stops reporting: Unknown after {silent} s (8-second expiry)")

        # Recovery.
        started = time.monotonic()
        d.control("restore")
        d.metric_until(STATUS, lambda v: v == 0, timeout=10, deployment="EU-A")
        evidence["recover_s"] = round(time.monotonic() - started, 1)
        checks.append(f"EU-A recovers: Healthy in {evidence['recover_s']} s")
    finally:
        health.close()
        d.reset()
    path = d.save("health-telemetry.json", {"checks": checks, **evidence})
    for check in checks:
        print("ok:", check)
    print("evidence:", path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
