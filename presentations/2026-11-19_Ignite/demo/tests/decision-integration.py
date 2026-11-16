#!/usr/bin/env python3
"""Real Phase 4 native query gate; owns fixture changes, not service processes."""
import importlib.util
import json
from pathlib import Path

DEMO = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("work", DEMO / "tests/workload-integration.py")
work = importlib.util.module_from_spec(spec)
spec.loader.exec_module(work)


def main():
    work.native.runtime.sql((DEMO / "database/reset.sql").read_text())
    attachments = {name: work.native.Attachment(name) for name in
                   ["decision-context", "policy-results", "allocation-results", "resilience-results"]}
    checks = []
    def result(name, predicate):
        rows = attachments[name].until(lambda rows: rows.get("active") and predicate(json.loads(rows["active"]["payload"])))
        return json.loads(rows["active"]["payload"])
    def edit(expression):
        work.native.runtime.sql("UPDATE ignite_model SET revision=revision+1,payload=" + expression + " WHERE id='active';")
    try:
        epoch = work.api("/health")["run_epoch"]
        result("allocation-results", lambda d: d["run_epoch"] == epoch and d["state"] == "feasible")
        result("policy-results", lambda d: d["run_epoch"] == epoch and d["state"] == "fresh")
        checks.append("snapshot-first real native policy/allocation bootstrap")
        work.control("fail")
        d = result("allocation-results", lambda d: d["assignments"] == {"batch":"US-C","support":"EU-B"})
        assert d["valid"] and not d["eligibility"]["support"]["US-C"]["allowed"]
        checks.append("observed EU-A failure derives complete batch->US-C and support->EU-B")
        edit("jsonb_set(jsonb_set(payload,'{policy,destinations,support}','[\"EU-A\"]'),'{policy,revision}',to_jsonb((payload#>>'{policy,revision}')::bigint+1))")
        denied = result("allocation-results", lambda d: d["state"] == "infeasible")
        assert denied["basis"] != d["basis"]
        checks.append("real changed Rego input invalidates prior candidate")
        work.native.runtime.sql((DEMO / "database/reset.sql").read_text())
        epoch = work.api("/health")["run_epoch"]
        edit("jsonb_set(jsonb_set(payload,'{workloads,1,destination}','\"US-C\"'),'{endpoints,1,budget_tpm}','50000')")
        current = result("allocation-results", lambda d: d["run_epoch"] == epoch and d["model"]["endpoints"][1]["budget_tpm"] == 50000 and d["state"] == "feasible")
        result("resilience-results", lambda d: d["basis"] == current["basis"] and d["state"] == "infeasible")
        checks.append("healthy complete current allocation fits while named EU-A loss is infeasible")
        edit("jsonb_set(payload,'{endpoints,1,budget_tpm}','100000')")
        result("resilience-results", lambda d: d["run_epoch"] == epoch and d["model"]["endpoints"][1]["budget_tpm"] == 100000 and d["state"] == "feasible" and not d["valid"])
        checks.append("capacity input restores read-only resilience without fault injection")
        work.control("pause-reports", "EU-B")
        result("allocation-results", lambda d: d["state"] == "unknown")
        checks.append("actual native timer expiration makes candidate unknown")
        path = DEMO / ".native/evidence/decision.json"
        path.write_text(json.dumps({"checks": checks, "history": {n:a.history for n,a in attachments.items()}}, indent=2))
        print(json.dumps({"passed":checks,"evidence":str(path)}, indent=2))
    finally:
        for a in attachments.values():
            a.close()
        work.native.runtime.sql((DEMO / "database/reset.sql").read_text())
        work.wait(lambda: work.api("/health")["continuous_context"] == "ready")


if __name__ == "__main__":
    main()
