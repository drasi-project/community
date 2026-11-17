#!/usr/bin/env python3
"""Live gate for Drasi's decisions: Rego policy, the optimizer's plan and its what-if.

Drives only the simulated deployments and the demo's scenario edits, and reads every answer
from Drasi: the desk-stream SSE reaction, the Prometheus reaction and Server's query results.
Resets this copy's demo before and after; needs the running services (SETUP.md)."""
import json

import drasi_outputs as d


def assignments(plan):
    return json.loads(plan["assignments"])


def main():
    checks, evidence = [], {}
    d.reset()
    stream = d.DeskStream(["plan-results", "tile-action", "tile-resilience", "tile-running"])
    policy, losses = d.Attachment("policy-results"), d.Attachment("loss-results")
    try:
        epoch = d.health()["run_epoch"]
        plan = stream.until("plan-results", lambda r: r and r[0]["run_epoch"] == epoch
                            and r[0]["state"] == "feasible" and r[0]["moves"] == 0)[0]
        assert assignments(plan) == {"support": "EU-A", "batch": "EU-B"}, plan
        policy.until(lambda rows: len(rows) == 6 and all(r["run_epoch"] == epoch for r in rows.values()))
        states = {(r["subject"], r["target"]): r["state"] for r in policy.rows.values()}
        assert states == {("support", "EU-A"): "allowed", ("support", "EU-B"): "allowed",
                          ("support", "US-C"): "denied", ("batch", "EU-A"): "allowed",
                          ("batch", "EU-B"): "allowed", ("batch", "US-C"): "allowed"}, states
        for (workload, deployment), state in states.items():
            d.metric_until("ignite_drasi_policy_allowed", lambda v, s=state: v == (1 if s == "allowed" else 0),
                           workload=workload, deployment=deployment)
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == 1)
        stream.until("tile-action", lambda r: r and r[0]["value"] == 0)
        d.metric_until("ignite_drasi_safe_if_one_fails", lambda v: v == 1)
        d.metric_until("ignite_drasi_running_tpm", lambda v: v == 60000, workload="support", deployment="EU-A")
        d.metric_until("ignite_drasi_running_tpm", lambda v: v == 80000, workload="batch", deployment="EU-B")
        checks.append("baseline: Rego decisions, the optimizer's plan (no moves) and resilience (safe) "
                      "from the scenario tables; the Prometheus reaction serves the same answers")

        d.control("fail")
        d.metric_until("ignite_drasi_deployment_status", lambda v: v == 2, deployment="EU-A")
        plan = stream.until("plan-results", lambda r: r and r[0]["state"] == "feasible" and r[0]["moves"] > 0)[0]
        assert assignments(plan) == {"support": "EU-B", "batch": "US-C"} and plan["moves"] == 2, plan
        stream.until("tile-action", lambda r: r and r[0]["value"] == 1)
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == 2)
        d.metric_until("ignite_drasi_proposed_tpm", lambda v: v == 60000, workload="support", deployment="EU-B")
        d.metric_until("ignite_drasi_proposed_tpm", lambda v: v == 80000, workload="batch", deployment="US-C")
        losses.until(lambda rows: rows.get("EU-B", {}).get("state") == "infeasible"
                     and rows["EU-B"]["basis"] == plan["basis"])
        evidence["failure_plan"] = plan
        checks.append("EU-A reports failed: Drasi plans support -> EU-B and summaries -> US-C (2 moves); "
                      "action 'approve proposed plan', 'already one down'; losing EU-B as well leaves no plan")

        d.configure(destinations={"support": ["EU-A"]})
        policy.until(lambda rows: any(r["subject"] == "support" and r["target"] == "EU-B" and r["state"] == "denied"
                                      for r in rows.values()))
        d.metric_until("ignite_drasi_policy_allowed", lambda v: v == 0, workload="support", deployment="EU-B")
        denied = stream.until("plan-results", lambda r: r and r[0]["state"] == "infeasible")[0]
        assert denied["basis"] != plan["basis"]
        stream.until("tile-action", lambda r: r and r[0]["value"] == 2)
        d.metric_until("ignite_drasi_action_needed", lambda v: v == 2)
        evidence["infeasible_plan"] = denied
        checks.append("policy parameter edit (support may run only on EU-A) is data: Rego denies EU-B, "
                      "the plan becomes infeasible with a new basis, action 'no valid plan'")

        d.configure(destinations={"support": ["EU-A", "EU-B"]})
        plan = stream.until("plan-results", lambda r: r and r[0]["state"] == "feasible" and r[0]["moves"] == 2)[0]
        approval = d.approve(plan)
        d.wait(lambda: d.routes() == {"support": "EU-B", "batch": "US-C"}, 30, "Drasi applies the approved plan")
        d.control("restore")
        stream.until("plan-results", lambda r: r and r[0]["state"] == "feasible" and r[0]["moves"] == 0)
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == 1)
        d.configure(capacity={"EU-A": 50000})
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == 0 and r[0]["lost"] == "EU-B")
        stream.until("tile-action", lambda r: r and r[0]["value"] == 3)
        d.metric_until("ignite_drasi_safe_if_one_fails", lambda v: v == 0, lost="EU-B")
        losses.until(lambda rows: rows.get("EU-B", {}).get("state") == "infeasible")
        evidence["at_risk"] = {"approval": d.approval(approval), "losses": dict(losses.rows)}
        checks.append("after recovery with EU-A at 50k: all healthy, but 'not if EU-B fails' (at risk), "
                      "from the what-if model alone, no fault injected")

        d.configure(capacity={"EU-A": 100000})
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == 1)
        stream.until("tile-action", lambda r: r and r[0]["value"] == 0)
        checks.append("EU-A back to 100k restores 'safe if one fails'")

        d.control("pause-reports", "EU-B")
        d.metric_until("ignite_drasi_deployment_status", lambda v: v == -1, timeout=25, deployment="EU-B")
        stream.until("plan-results", lambda r: r and r[0]["state"] == "unknown", 25)
        stream.until("tile-action", lambda r: r and r[0]["value"] == -1)
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == -1)
        d.metric_until("ignite_drasi_action_needed", lambda v: v == -1)
        d.control("resume-reports", "EU-B")
        stream.until("tile-action", lambda r: r and r[0]["value"] == 0, 25)
        checks.append("EU-B stops reporting: Drasi's timer expires its last report, the deployment, plan and "
                      "answers become Unknown (never a guess); resuming reports restores them")

        path = d.save("decision.json", {"checks": checks, **evidence, "stream": stream.history,
                                        "policy": policy.history, "losses": losses.history})
        print(json.dumps({"passed": checks, "evidence": str(path)}, indent=2))
    finally:
        stream.close()
        policy.close()
        losses.close()
        d.reset()


if __name__ == "__main__":
    main()
