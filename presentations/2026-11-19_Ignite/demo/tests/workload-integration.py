#!/usr/bin/env python3
"""Live Phase 2-3 gate. Explicitly resets this demo; requires running owned services."""
import importlib.util
import json
import subprocess
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ops"))
import settings  # noqa: E402

DEMO = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("native", DEMO / "tests/native-integration.py")
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)
EVIDENCE = DEMO / ".native/evidence/workload.json"


def api(path, body=None, expected=200):
    request = urllib.request.Request(settings.url("WORKLOAD") + path,
        data=None if body is None else json.dumps(body).encode(),
        headers={"Content-Type": "application/json"})
    try:
        response = urllib.request.urlopen(request, timeout=8)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        value = json.load(response)
        assert response.status == expected, (response.status, value)
        return value


def wait(predicate, timeout=30):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        value = predicate()
        if value:
            return value
        time.sleep(0.2)
    raise AssertionError("bounded condition did not become true")


def control(action, endpoint="EU-A"):
    state = api("/health")
    return api("/control", {"run_epoch": state["run_epoch"], "revision": state["revision"],
                            "endpoint": endpoint, "action": action})


def submit(workload="support"):
    epoch = api("/health")["run_epoch"]
    body = {"id": f"{epoch}-{uuid.uuid4()}", "run_epoch": epoch, "workload": workload,
            "content": "SYNTHETIC RESTRICTED support text" if workload == "support" else "PUBLIC DOCUMENT text"}
    return body, api("/tasks", body)


def main():
    native.runtime.sql((DEMO / "database/reset.sql").read_text())
    attachments = {}
    checks = []
    try:
        for name in ["model-context", "task-context", "overdue-tasks", "deadline-history",
                     "endpoint-freshness", "attempt-context", "breach-history", "alert-context"]:
            attachments[name] = native.Attachment(name)
        epoch = api("/health")["run_epoch"]
        attachments["model-context"].until(lambda rows: rows.get("active", {}).get("run_epoch") == epoch)
        fresh = attachments["endpoint-freshness"]
        fresh.until(lambda rows: len(rows) == 3 and all(r["run_epoch"] == epoch and r["status"] == "ok" for r in rows.values()))
        checks.append("snapshot-first coherent model and fresh endpoint observations")
        wait(lambda: api("/health")["continuous_context"] == "ready")
        for workload in ["support", "batch"]:
            body, result = submit(workload)
            wait(lambda: api("/tasks/" + body["id"])["task"]["completed_ms"])
            task = api("/tasks/" + body["id"])
            assert task["task"]["endpoint"] == ("EU-A" if workload == "support" else "EU-B")
            assert task["task"]["trace_id"] == result["trace_id"]
            duplicate = api("/tasks", body)
            assert duplicate["duplicate"] and duplicate["trace_id"] == result["trace_id"]
            api("/tasks", {**body, "content": "different"}, expected=409)
        checks.append("real support/public batch HTTP traffic, durable outcomes and submission idempotency")
        rejected = {"id": f"{epoch}-prohibited", "run_epoch": epoch, "workload": "support",
                    "content": "SYNTHETIC RESTRICTED never transmitted", "destination": "US-C"}
        api("/tasks", rejected, expected=409)
        assert "\n     0\n" in native.runtime.sql(f"SELECT count(*) FROM ignite_tasks WHERE id='{rejected['id']}';")
        checks.append("gateway rejects requested US-C support destination before enqueue or content transmission")

        control("throttle", "EU-B")
        throttled, _ = submit("batch")
        wait(lambda: any(a["status"]==429 for a in api("/tasks/"+throttled["id"])["attempts"]))
        control("restore", "EU-B")
        wait(lambda: api("/tasks/"+throttled["id"])["task"]["completed_ms"])
        checks.append("real HTTP429 throttling queues and retries public batch")

        basis = api("/health")
        control("fail")
        api("/control", {"run_epoch": basis["run_epoch"], "revision": basis["revision"],
                         "endpoint": "EU-A", "action": "restore"}, expected=409)
        body, result = submit()
        before = api("/tasks/" + body["id"])["task"]
        wait(lambda: any(a["status"] == 503 for a in api("/tasks/" + body["id"])["attempts"]))
        attempts = api("/tasks/" + body["id"])
        assert attempts["task"] == before
        attachments["overdue-tasks"].until(lambda rows: body["id"] in rows, timeout=25)
        after = api("/tasks/" + body["id"])["task"]
        assert before == after, (before, after)
        attachments["breach-history"].until(lambda rows: body["id"] in rows)
        checks.append("actual 503 plus unchanged task becomes overdue via native Drasi timer; durable breach")

        control("pause-reports", "EU-B")
        fresh.until(lambda rows: rows.get("EU-B", {}).get("run_epoch") == epoch)
        # Drain to the last report before testing expiry; SQL confirms no further report update.
        report_before = native.runtime.sql("SELECT row_to_json(r) FROM ignite_reports r WHERE id='EU-B';")
        fresh.until(lambda rows: rows.get("EU-B", {}).get("status") == "unknown", timeout=15)
        assert report_before == native.runtime.sql("SELECT row_to_json(r) FROM ignite_reports r WHERE id='EU-B';")
        wait(lambda: api("/health")["continuous_context"]=="unknown")
        for sql in [
            "UPDATE ignite_reports SET sequence=sequence-1 WHERE id='EU-B';",
            "UPDATE ignite_reports SET sequence=sequence+1,observed_ms=observed_ms-1 WHERE id='EU-B';",
            "UPDATE ignite_model SET revision=revision-1 WHERE id='active';",
        ]:
            try:
                native.runtime.sql(sql)
                raise AssertionError("out-of-order input accepted")
            except subprocess.CalledProcessError as error:
                assert "non-monotonic" in error.stderr
        assert report_before == native.runtime.sql("SELECT row_to_json(r) FROM ignite_reports r WHERE id='EU-B';")
        checks.append("unchanged paused report expires to unknown through native Drasi timer")
        checks.append("duplicate/out-of-order report and model versions rejected, not healthy defaults")

        control("restore")
        wait(lambda: api("/tasks/" + body["id"])["task"]["completed_ms"])
        attachments["overdue-tasks"].until(lambda rows: body["id"] not in rows)
        assert body["id"] in attachments["breach-history"].rows
        reconnect = native.Attachment("breach-history")
        assert body["id"] in reconnect.rows
        reconnect.close()
        control("resume-reports", "EU-B")
        fresh.until(lambda rows: rows.get("EU-B", {}).get("status") == "ok")
        checks.append("manual restore completes affected task and clears current impact, retaining breach on reattach")

        receipts = native.runtime.sql("SELECT count(*) FROM ignite_receipts WHERE workload='support' AND endpoint='US-C';")
        assert "\n     0\n" in receipts, receipts
        checks.append("no restricted support receipts at US-C")
        EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
        EVIDENCE.write_text(json.dumps({"checks": checks, "incident_task":body, "trace":result,
            "history":{name:a.history for name,a in attachments.items()}}, indent=2))
        print(json.dumps({"passed":checks,"evidence":str(EVIDENCE)},indent=2))
    finally:
        for a in attachments.values():
            a.close()
        for endpoint in ["EU-A","EU-B","US-C"]:
            control("restore",endpoint)


if __name__ == "__main__":
    main()
