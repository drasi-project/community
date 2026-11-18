#!/usr/bin/env python3
"""Live gate for Drasi's reactions: approvals checked by Drasi, routes changed by its gateway
reaction, recovery verified by its queries, and its Grafana annotations and Langfuse scores.

The AI service only records approvals (the approval tool), simulates the deployments and serves
requests; every decision is read from Drasi's outputs or from the effect of its reactions.
Resets this copy's demo before and after; needs the running services (SETUP.md)."""
import concurrent.futures
import json
import time

import drasi_outputs as d

ALIASES = ["plan-results", "tile-action", "tile-last-approval", "tile-recovery", "overdue-tasks"]


def latest(stream, ident, predicate=lambda row: True, timeout=30):
    return stream.until("tile-last-approval", lambda r: r and r[0]["approval"] == ident and predicate(r[0]),
                        timeout)[0]


def proposed(stream, timeout=30):
    return stream.until("plan-results", lambda r: r and r[0]["state"] == "feasible" and r[0]["moves"] > 0,
                        timeout)[0]


def stuck_request(stream):
    """A support request on the failed EU-A that Drasi reports overdue; the task itself never changes."""
    body, trace = d.submit()
    d.wait(lambda: any(a["status"] == 503 for a in d.task(body["id"])["attempts"]), 20, "a 503 from EU-A")
    before = d.task(body["id"])["task"]
    stream.until("overdue-tasks", lambda rows: any(r["id"] == body["id"] for r in rows), 30)
    d.metric_until("drasi_requests_overdue", lambda v: v and v >= 1, workload="support")
    assert d.task(body["id"])["task"] == before, "the deadline changes Drasi's answer, not the task"
    return body, trace, before


def applied(stream, ident, body):
    """Drasi's gateway reaction applied approval `ident`; the stuck request completes on EU-B."""
    row = latest(stream, ident, lambda r: r["status"] in ("applied", "verified"))
    assert d.routes() == {"support": "EU-B", "batch": "US-C"}
    stored = d.approval(ident)
    d.wait(lambda: d.task(body["id"])["task"]["completed_ms"], 30, "the stuck request completes")
    task = d.task(body["id"])["task"]
    assert task["endpoint"] == "EU-B" and task["completed_revision"] == stored["applied_revision"], (task, stored)
    stream.until("overdue-tasks", lambda rows: all(r["id"] != body["id"] for r in rows))
    return row, stored, task


def verified(stream, ident):
    """Three support answers on the new route: Drasi's recovery query confirms, its reaction records it."""
    traces = []
    for _ in range(3):
        body, trace = d.submit()
        d.wait(lambda: d.task(body["id"])["task"]["completed_ms"], 20, "a support answer")
        traces.append(trace["trace_id"])
    stream.until("tile-recovery", lambda r: r and r[0]["value"] == 1, 30)
    d.metric_until("drasi_recovery_state", lambda v: v == 1)
    latest(stream, ident, lambda r: r["status"] == "verified" and r["value"] == 3)
    d.metric_until("drasi_last_approval_state", lambda v: v == 3, approval=ident)
    return d.approval(ident), traces


def incident(stream):
    """One talk cycle from Drasi's outputs: failure, overdue, plan, approval, apply, recovery."""
    started = time.monotonic()
    epoch = d.health()["run_epoch"]
    d.control("fail")
    plan = proposed(stream)
    assessment_seconds = time.monotonic() - started
    body, trace, before = stuck_request(stream)
    ident = d.approve(plan)
    row, stored, task = applied(stream, ident, body)
    record, _ = verified(stream, ident)
    assert record["run_epoch"] == epoch and record["verified_ms"] >= stored["decided_ms"]
    return {"epoch": epoch, "input_to_plan_seconds": assessment_seconds,
            "applied_to_verified_seconds": (record["decided_ms"] - stored["decided_ms"]) / 1000,
            "elapsed_seconds": time.monotonic() - started, "plan": plan, "trace": trace,
            "unchanged_task": before, "task": task, "approval": record}


def main():
    checks, evidence = [], {}
    d.reset()
    stream = d.DeskStream(ALIASES)
    try:
        start_routes = d.routes()
        d.control("fail")
        first = proposed(stream)

        stale = d.approve(first, basis="not-the-basis-drasi-solved")
        row = latest(stream, stale, lambda r: r["status"] == "rejected")
        assert row["reason"] == "The facts changed since this plan was proposed", row
        assert d.approval(stale)["reason"] == row["reason"] and d.routes() == start_routes
        d.metric_until("drasi_last_approval_state", lambda v: v == 4, approval=stale)
        checks.append("an approval whose basis is not the one Drasi solved is rejected by Drasi's "
                      "approval-stale query through the gateway reaction, with Drasi's reason; routes unchanged")

        d.configure(destinations={"support": ["EU-A"]})
        stream.until("plan-results", lambda r: r and r[0]["state"] == "infeasible")
        denied = d.approve(first)
        row = latest(stream, denied, lambda r: r["status"] == "rejected")
        assert row["reason"].startswith("Drasi has no valid plan now"), row
        d.configure(destinations={"support": ["EU-A", "EU-B"]})
        plan = proposed(stream)
        assert plan["revision"] != first["revision"]
        old = d.approve(first)
        row = latest(stream, old, lambda r: r["status"] == "rejected")
        assert row["reason"] == "The facts changed since this plan was proposed", row
        assert d.routes() == start_routes
        checks.append("after a policy edit Drasi rejects the captured plan ('no valid plan now'), and after "
                      "restoring it rejects the superseded plan ('facts changed'); nothing applied")

        body, trace, before = stuck_request(stream)
        evidence["stuck_request"] = {"task": before, "trace": trace}
        checks.append("a support request stuck on failed EU-A becomes overdue in Drasi (desk stream and "
                      "Prometheus reaction) through Drasi's timer, without any change to the task")

        d.control("pause-reports", "EU-B")
        stream.until("plan-results", lambda r: r and r[0]["state"] == "unknown", 25)
        ident = d.approve(plan)
        with concurrent.futures.ThreadPoolExecutor(2) as pool:
            assert set(pool.map(lambda _: d.approve(plan, ident), range(2))) == {ident}
        assert d.sql(f"SELECT count(*) FROM change_approvals WHERE id='{ident}';") == "1"
        latest(stream, ident, lambda r: r["status"] == "pending")
        stream.holds("tile-last-approval", lambda r: r and r[0]["approval"] == ident and r[0]["status"] == "pending", 4)
        assert d.routes() == start_routes
        checks.append("while Drasi does not know (EU-B silent, plan Unknown) an approval waits pending; "
                      "repeated submissions of one approval ID record one approval")

        d.control("resume-reports", "EU-B")
        row, stored, task = applied(stream, ident, body)
        assert stored["applied_revision"] == d.health()["revision"]
        d.approve(plan, ident)
        assert d.approval(ident)["status"] in ("applied", "verified")
        deliveries = json.loads(d.sql("SELECT coalesce(json_agg(result),'[]') FROM gateway_deliveries "
                                      f"WHERE result->>'result'='applied' AND received_ms >= {stored['requested_ms']};"))
        assert len(deliveries) == 1, deliveries
        evidence["applied"] = {"approval": stored, "task": task, "deliveries": deliveries}
        checks.append("once Drasi knows again its approval-valid query matches, the gateway reaction "
                      "applies the plan once (compare-and-set on revision), and the overdue request "
                      "completes on EU-B at the applied revision")

        record, traces = verified(stream, ident)
        # Drasi's recovery rule waits five seconds after the route changed before it confirms.
        assert record["decided_ms"] >= stored["decided_ms"] + 5000, (record, stored)
        assert record["verified_ms"] >= stored["decided_ms"], (record, stored)
        evidence["verified"] = record
        checks.append("three support answers on the new route: Drasi's recovery query confirms and its "
                      "reaction marks the approval verified (desk stream and Prometheus reaction)")

        for tag in [f"{stale}-rejected", f"{ident}-applied", f"{ident}-verified"]:
            d.wait(lambda: d.annotations(tag), 30, f"Grafana annotation {tag}")
        d.wait(lambda: d.annotations(f"plan-{first['candidate_id']}") or d.annotations(f"plan-{plan['candidate_id']}"),
               30, "Grafana plan annotation")
        evidence["annotations"] = d.annotations(f"{ident}-verified")
        for trace_id in [trace["trace_id"], *traces]:
            found = d.wait(lambda: d.scores(trace_id, "drasi_recovery"), 60, f"Langfuse score for {trace_id}")
            assert all(s["subject"] == {"kind": "trace", "id": trace_id} for s in found), found
        evidence["scores"] = d.scores(traces[0], "drasi_recovery")
        checks.append("Drasi's annotation reaction posted the rejection, apply and verification and the plan "
                      "to Grafana; its score reaction posted drasi_recovery to each recovered request's trace")

        stream.until("tile-recovery", lambda r: r and r[0]["value"] == 0, 40)
        assert d.approval(ident)["status"] == "verified"
        checks.append("with no new answers Drasi's recovery evidence expires (timer) and the tile returns to "
                      "'awaiting evidence'; the recorded verification stays")
        path = d.save("recovery.json", {"checks": checks, **evidence, "stream": stream.history})
        print(json.dumps({"passed": checks, "evidence": str(path)}, indent=2))
    finally:
        stream.close()
        d.reset()


if __name__ == "__main__":
    main()
