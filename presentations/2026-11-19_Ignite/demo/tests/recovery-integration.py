#!/usr/bin/env python3
"""Phase 5 live gate: real approval, queue recovery, current-policy denial and products."""
import base64
import concurrent.futures
import importlib.util
import json
import time
import urllib.parse
import uuid
from pathlib import Path

DEMO = Path(__file__).resolve().parents[1]
def load(name, file):
    spec = importlib.util.spec_from_file_location(name, DEMO / "tests" / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module
work = load("work", "workload-integration.py")
obs = load("obs", "observability-smoke.py")


def reset():
    state = work.api("/health")
    work.api("/reset", {k:state[k] for k in ["run_epoch","revision"]})
    work.wait(lambda: work.api("/decisions").get("candidate_current"))


def candidate():
    result = work.api("/decisions")
    return result["allocation"] if result["approval"] == "awaiting_approval" else None


def approve(d, command=None, expected=200):
    return work.api("/approvals", {"command_id":command or str(uuid.uuid4()),
        "candidate_id":d["candidate_id"],"basis":d["basis"]}, expected=expected)


def command(id):
    return next(c for c in work.api("/decisions")["commands"] if c["id"] == id)


def configure(edit):
    state = work.api("/health")
    edit(state["model"])
    return work.api("/configuration", {k:state[k] for k in ["run_epoch","revision","model"]})


def main():
    checks = []
    reset()
    recovery = work.native.Attachment("recovery-results")
    try:
        work.control("fail")
        d = work.wait(candidate)
        approve(work.api("/decisions")["resilience"], expected=409)
        checks.append("read-only named-failure result cannot be approved as a live allocation")
        configure(lambda m:m["policy"]["destinations"].update({"support":["EU-A"]}))
        approve(d, expected=409)
        checks.append("policy change rejects captured stale candidate and records rejection")
        configure(lambda m:m["policy"]["destinations"].update({"support":["EU-A","EU-B"]}))
        d = work.wait(candidate)
        work.control("pause-reports", "EU-B")
        work.wait(lambda: work.api("/decisions")["allocation"]["state"] == "unknown", 15)
        approve(d, expected=409)
        work.control("resume-reports", "EU-B")
        work.wait(candidate)
        body, trace = work.submit()
        work.wait(lambda:any(a["status"]==503 for a in work.api("/tasks/"+body["id"])["attempts"]))
        overdue = work.native.Attachment("overdue-tasks")
        overdue.until(lambda rows:body["id"] in rows, timeout=25)
        overdue.close()
        d = work.wait(candidate)
        id = str(uuid.uuid4())
        with concurrent.futures.ThreadPoolExecutor(2) as pool:
            results = list(pool.map(lambda _:approve(d,id), range(2)))
        assert sorted(r["duplicate"] for r in results) == [False,True]
        approve({**d,"basis":"conflicting"},id,expected=409)
        approve(d,expected=409)
        work.wait(lambda:command(id)["status"] in ["gateway_applied","recovery_confirmed"])
        work.wait(lambda:work.api("/tasks/"+body["id"])["task"]["completed_ms"])
        completed = work.api("/tasks/"+body["id"])["task"]
        assert completed["endpoint"] == "EU-B"
        assert command(id)["applied_revision"] == completed["completed_revision"]
        assert work.api("/health")["model"]["workloads"][1]["destination"] == "US-C"
        checks.append("concurrent duplicate command applies once; complete assignment adopted for queued request")
        for _ in range(3):
            work.submit()
        recovery.until(lambda rows:rows.get(id,{}).get("state")=="confirmed")
        work.wait(lambda:command(id)["status"]=="recovery_confirmed")
        c = command(id)
        assert c["recovered_ms"] >= c["applied_ms"]+5000
        assert int(work.native.runtime.sql(f"SELECT count(*) FROM ignite_breaches WHERE id='{body['id']}';",tuples=True)) == 1
        checks.append("three actual support completions at applied revision + native five-second window confirm recovery, preserving overdue history")
        recovery.until(lambda rows:rows.get(id,{}).get("state")=="unconfirmed",timeout=22)
        assert command(id)["status"]=="recovery_confirmed"
        checks.append("native success-evidence expiry retracts current recovery without erasing historical confirmation")

        configure(lambda m:m["policy"]["destinations"].update({"support":["EU-A"]}))
        blocked, blocked_trace = work.submit()
        work.wait(lambda:any(a["status"]==403 for a in work.api("/tasks/"+blocked["id"])["attempts"]))
        assert int(work.native.runtime.sql(f"SELECT count(*) FROM ignite_receipts WHERE task_id='{blocked['id']}';",tuples=True)) == 0
        assert not work.api("/tasks/"+blocked["id"])["task"]["completed_ms"]
        checks.append("gateway current Rego policy blocks previously applied destination before any HTTP content transmission")
        configure(lambda m:m["policy"]["destinations"].update({"support":["EU-A","EU-B"]}))
        work.wait(lambda:work.api("/tasks/"+blocked["id"])["task"]["completed_ms"])
        env = obs.environment()
        grafana_auth = base64.b64encode(f"admin:{env['GRAFANA_ADMIN_PASSWORD']}".encode()).decode()
        def annotation():
            return obs.get("http://127.0.0.1:13000/api/annotations?"+urllib.parse.urlencode({"tags":f"{id}-recovered"}),grafana_auth)
        annotations,_ = obs.wait_for("real recovered Grafana annotation",annotation)
        def scores(trace_id):
            return obs.get("http://127.0.0.1:13001/api/public/v3/scores?"+urllib.parse.urlencode({"traceId":trace_id,"fields":"subject,details"}),
                           env["LANGFUSE_AUTH_BASE64"])["data"]
        actual_scores,_=obs.wait_for("correlated Langfuse recovery score",lambda:[s for s in scores(trace["trace_id"]) if s["name"]=="drasi_recovery"])
        blocked_scores,_=obs.wait_for("correlated Langfuse current-policy denial",lambda:[s for s in scores(blocked_trace["trace_id"]) if s["name"]=="drasi_policy_allowed" and s["value"]==0])
        assert all(s["subject"] == {"kind":"trace","id":trace["trace_id"]} for s in actual_scores)
        assert all(s["subject"] == {"kind":"trace","id":blocked_trace["trace_id"]} for s in blocked_scores)
        metric_url="http://127.0.0.1:19090/api/v1/query?"+urllib.parse.urlencode({"query":'ignite_drasi_assessment{kind="allocation",state="feasible"}'})
        metrics,_=obs.wait_for("real derived Prometheus summary",lambda:obs.get(metric_url)["data"]["result"])
        assert all(not any(key in r["metric"] for key in ["task","task_id","trace_id"]) for r in metrics)
        checks.append("actual Grafana annotation, low-cardinality Prometheus assessment and exact Langfuse trace scores")
        path=DEMO/".native/evidence/recovery.json"
        path.write_text(json.dumps({"checks":checks,"command":c,"task":completed,"trace":trace,
            "recovery_events":recovery.history,"annotations":annotations,"scores":actual_scores,
            "blocked_scores":blocked_scores,"metrics":metrics},indent=2))
        print(json.dumps({"passed":checks,"evidence":str(path)},indent=2))
    finally:
        recovery.close()
        reset()


if __name__=="__main__":
    main()
