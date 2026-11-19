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
    d.reset(us=False)
    stream = d.DeskStream(["plan-results", "tile-action", "tile-resilience", "tile-running"])
    policy, losses = d.Attachment("policy-results"), d.Attachment("loss-results")
    try:
        epoch = d.health()["run_epoch"]
        plan = stream.until("plan-results", lambda r: r and r[0]["run_epoch"] == epoch
                            and r[0]["state"] == "feasible" and r[0]["moves"] == 0)[0]
        assert assignments(plan) == {"support": "EU-A", "batch": "EU-B"}, plan
        policy.until(lambda rows: len(rows) == 4 and all(r["run_epoch"] == epoch for r in rows.values()))
        states = {(r["subject"], r["target"]): r["state"] for r in policy.rows.values()}
        assert all(target != "US-C" for _, target in states) and set(states.values()) == {"allowed"}, states
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == 0 and r[0]["lost"] == "EU-A")
        stream.until("tile-action", lambda r: r and r[0]["value"] == 3)
        d.metric_until("drasi_failover_readiness", lambda v: v == 0, lost="EU-A",
                       text="At risk: losing EU-A leaves no valid plan")
        d.metric_until("drasi_recommended_action", lambda v: v == 3, text="Add capacity: losing EU-A leaves no valid plan")
        losses.until(lambda rows: rows.get("EU-A", {}).get("state") == "infeasible")
        evidence["seed_at_risk"] = dict(losses.rows)
        checks.append("seed (EU-A and EU-B, both workloads Europe-only): all healthy and no moves, but at risk: "
                      "losing EU-A leaves no valid plan, from the what-if model alone, no fault injected")

        d.provision()
        policy.until(lambda rows: len(rows) == 6)
        states = {(r["subject"], r["target"]): r["state"] for r in policy.rows.values()}
        assert states[("support", "US-C")] == states[("batch", "US-C")] == "denied", states
        d.metric_until("drasi_deployment_health_status", lambda v: v == 0, deployment="US-C")
        losses.until(lambda rows: "US-C" in rows and rows.get("EU-A", {}).get("state") == "infeasible")
        d.metric_until("drasi_recommended_action", lambda v: v == 3, text="Add capacity: losing EU-A leaves no valid plan")
        checks.append("US-C provisioned (a new model_deployments row): Drasi adds it to policy, the plan and the "
                      "what-ifs; Rego denies both workloads there, so failover stays at risk")

        d.configure(destinations=d.US)
        policy.until(lambda rows: any(r["subject"] == "batch" and r["target"] == "US-C" and r["state"] == "allowed"
                                      for r in rows.values()))
        states = {(r["subject"], r["target"]): r["state"] for r in policy.rows.values()}
        assert states == {("support", "EU-A"): "allowed", ("support", "EU-B"): "allowed",
                          ("support", "US-C"): "denied", ("batch", "EU-A"): "allowed",
                          ("batch", "EU-B"): "allowed", ("batch", "US-C"): "allowed"}, states
        for (workload, deployment), state in states.items():
            d.metric_until("drasi_placement_allowed", lambda v, s=state: v == (1 if s == "allowed" else 0),
                           workload=workload, deployment=deployment)
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == 1)
        stream.until("tile-action", lambda r: r and r[0]["value"] == 0)
        d.metric_until("drasi_failover_readiness", lambda v: v == 1, text="Survives losing any one deployment")
        d.metric_until("drasi_workload_running_tpm", lambda v: v == 60000, workload="support", deployment="EU-A")
        d.metric_until("drasi_workload_running_tpm", lambda v: v == 80000, workload="batch", deployment="EU-B")
        checks.append("summaries also allowed in the US (a data change): Rego decisions, the optimizer's plan "
                      "(no moves) and resilience (safe); the Prometheus reaction serves the same answers")

        d.control("fail")
        d.metric_until("drasi_deployment_health_status", lambda v: v == 2, deployment="EU-A")
        plan = stream.until("plan-results", lambda r: r and r[0]["state"] == "feasible" and r[0]["moves"] > 0)[0]
        assert assignments(plan) == {"support": "EU-B", "batch": "US-C"} and plan["moves"] == 2, plan
        stream.until("tile-action", lambda r: r and r[0]["value"] == 1)
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == 2)
        d.metric_until("drasi_workload_proposed_tpm", lambda v: v == 60000, workload="support", deployment="EU-B")
        d.metric_until("drasi_workload_proposed_tpm", lambda v: v == 80000, workload="batch", deployment="US-C")
        losses.until(lambda rows: rows.get("EU-B", {}).get("state") == "infeasible"
                     and rows["EU-B"]["basis"] == plan["basis"])
        evidence["failure_plan"] = plan
        checks.append("EU-A reports failed: Drasi plans support -> EU-B and summaries -> US-C (2 moves); "
                      "action 'approve proposed plan', 'already one down'; losing EU-B as well leaves no plan")

        d.configure(destinations={"support": ["EU-A"]})
        policy.until(lambda rows: any(r["subject"] == "support" and r["target"] == "EU-B" and r["state"] == "denied"
                                      for r in rows.values()))
        d.metric_until("drasi_placement_allowed", lambda v: v == 0, workload="support", deployment="EU-B")
        denied = stream.until("plan-results", lambda r: r and r[0]["state"] == "infeasible")[0]
        assert denied["basis"] != plan["basis"]
        stream.until("tile-action", lambda r: r and r[0]["value"] == 2)
        d.metric_until("drasi_recommended_action", lambda v: v == 2, text="No change fits policy and capacity")
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
        assert d.routes() == {"support": "EU-B", "batch": "US-C"}, "nothing moves back"
        evidence["recovered"] = {"approval": d.approval(approval)}
        checks.append("approved and applied, then EU-A restored: the plan keeps support on EU-B and summaries "
                      "on US-C (no moves) and failover is safe again")

        d.control("pause-reports", "EU-B")
        d.metric_until("drasi_deployment_health_status", lambda v: v == -1, timeout=25, deployment="EU-B")
        stream.until("plan-results", lambda r: r and r[0]["state"] == "unknown", 25)
        stream.until("tile-action", lambda r: r and r[0]["value"] == -1)
        stream.until("tile-resilience", lambda r: r and r[0]["value"] == -1)
        d.metric_until("drasi_recommended_action", lambda v: v == -1)
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
