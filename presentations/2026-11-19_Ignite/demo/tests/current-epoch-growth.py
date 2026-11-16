#!/usr/bin/env python3
"""Reproduce the soak cardinality without resetting: >=150 real tasks in ONE epoch."""
import importlib.util
import json
import time
from pathlib import Path

DEMO=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location("work",DEMO/"tests/workload-integration.py")
work=importlib.util.module_from_spec(spec)
spec.loader.exec_module(work)


def main():
    state=work.wait(lambda:s if (s:=work.api("/health"))["continuous_context"]=="ready" else None)
    epoch,revision=state["run_epoch"],state["revision"]
    before=json.loads(work.native.runtime.sql(
        "SELECT coalesce(json_agg(id),'[]') FROM ignite_tasks WHERE run_epoch=(SELECT run_epoch FROM ignite_model)",tuples=True))
    assert len(before)>=150, "Requires >=150 actual same-epoch tasks; no reset or synthetic seed workaround"
    original={table:json.loads(work.native.runtime.sql(f"SELECT coalesce(json_agg(id),'[]') FROM {table}",tuples=True))
        for table in ["ignite_tasks","ignite_attempts","ignite_commands","ignite_breaches"]}
    saved=DEMO/".native/evidence/soak-backlog-before.json"
    if saved.exists():
        retained=json.loads(saved.read_text())["history"]
        assert all(set(ids).issubset(original[table]) for table,ids in retained.items())
    initial_reports=work.api("/context")["queries"]["endpoint-freshness"]
    task,trace=work.submit()
    work.wait(lambda:work.api("/tasks/"+task["id"])["task"]["completed_ms"])
    completed=work.api("/tasks/"+task["id"])["task"]
    assert completed["completed_revision"]==revision
    summary=work.wait(lambda:r if (r:=work.api("/context")["queries"].get("recovery-task-context",{}).get("active"))
        and r["last_success_ms"]==completed["completed_ms"] and r["outstanding"]==0 else None)
    samples=[]
    for _ in range(120):
        context=work.api("/context")
        observed=int(time.time()*1000)
        assert context["streams_attached"]
        rows=context["queries"]
        assert rows["model-context"]["active"]["run_epoch"]==epoch
        assert rows["model-context"]["active"]["revision"]==revision
        assert rows["recovery-task-context"]["active"]==summary, "heartbeats changed the task-only summary"
        reports=rows["endpoint-freshness"]
        ages={id:observed-r["observed_ms"] for id,r in reports.items()}
        assert len(ages)==3 and max(ages.values())<4000, ages
        assert all(r["status"]!="unknown" for r in reports.values())
        assert all(r["run_epoch"]==epoch and r["applied_revision"]==revision for r in rows["recovery-results"].values())
        samples.append({"at_ms":observed,"report_age_ms":ages,"sequences":{id:r["sequence"] for id,r in reports.items()}})
        time.sleep(0.5)
    assert all(samples[-1]["sequences"][id]>r["sequence"]+30 for id,r in initial_reports.items())
    after=json.loads(work.native.runtime.sql(
        "SELECT coalesce(json_agg(id),'[]') FROM ignite_tasks WHERE run_epoch=(SELECT run_epoch FROM ignite_model)",tuples=True))
    assert set(before).issubset(after) and len(after)==len(before)+1
    for table,ids in original.items():
        current=json.loads(work.native.runtime.sql(f"SELECT coalesce(json_agg(id),'[]') FROM {table}",tuples=True))
        assert set(ids).issubset(current)
    evidence={"epoch":epoch,"revision":revision,"current_tasks_before":len(before),"current_tasks_after":len(after),
        "history_preserved":True,"summary":summary,"task":completed,"trace":trace,"samples":samples,
        "max_report_age_ms":max(age for s in samples for age in s["report_age_ms"].values())}
    path=DEMO/".native/evidence/current-epoch-growth.json"
    path.write_text(json.dumps(evidence,indent=2))
    print(json.dumps({k:v for k,v in evidence.items() if k not in ["samples","task","trace"]},indent=2))


if __name__=="__main__":
    main()
