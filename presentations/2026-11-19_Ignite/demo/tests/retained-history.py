#!/usr/bin/env python3
"""Growth regression on real retained history; no reset, deletes or synthetic seed records.

Run after repeated incident/recovery gates have retained >=75 tasks and >=15
commands, with a current applied allocation and all endpoint reporting enabled.
Submits one real support task, then observes indexed report/recovery propagation.
"""
import importlib.util
import json
import time
from pathlib import Path

DEMO=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location("work",DEMO/"tests/workload-integration.py")
work=importlib.util.module_from_spec(spec)
spec.loader.exec_module(work)


def history():
    return {table:set(json.loads(work.native.runtime.sql(
        f"SELECT coalesce(json_agg(id ORDER BY id),'[]') FROM {table}",tuples=True)))
        for table in ["ignite_tasks","ignite_attempts","ignite_commands","ignite_breaches"]}


def main():
    before=history()
    assert len(before["ignite_tasks"])>=75 and len(before["ignite_commands"])>=15, (
        "Run repeated real incident/recovery gates first; this gate must not pass on an empty history")
    saved=DEMO/".native/evidence/retained-history-before.json"
    if saved.exists():
        original=json.loads(saved.read_text())["history"]
        assert all(set(ids).issubset(before[table]) for table,ids in original.items())
    state=work.wait(lambda: (s if (s:=work.api("/health"))["continuous_context"]=="ready" else None))
    epoch,revision=state["run_epoch"],state["revision"]
    assert all(e["reporting"] for e in state["model"]["endpoints"])
    commands=work.api("/decisions")["commands"]
    command=next(c for c in commands if c["applied_revision"]==revision)
    task,trace=work.submit()
    work.wait(lambda:work.api("/tasks/"+task["id"])["task"]["completed_ms"])
    completion=work.api("/tasks/"+task["id"])["task"]
    assert completion["completed_revision"]==revision
    work.wait(lambda:work.api("/context")["queries"].get("recovery-results",{}).get(command["id"],{}).get("state")=="confirmed")
    samples=[]
    for _ in range(50):
        context=work.api("/context")
        observed=int(time.time()*1000)
        assert context["streams_attached"]
        rows=context["queries"]
        assert rows["model-context"]["active"]["run_epoch"]==epoch
        ages={name:{id:observed-r["observed_ms"] for id,r in rows[name].items()}
              for name in ["endpoint-freshness","decision-context"]}
        assert all(len(ages[name])==3 and max(ages[name].values())<4000 for name in ages), ages
        assert all(r["status"]!="unknown" for r in rows["endpoint-freshness"].values())
        recovery=rows["recovery-results"]
        assert recovery and all(r["run_epoch"]==epoch for r in recovery.values()), recovery
        r=recovery[command["id"]]
        assert r["outstanding"]==0 and r["last_success_ms"]==completion["completed_ms"],r
        assert r["recent_successes"] in [0,1],r
        if observed-completion["completed_ms"]>16000:
            assert r["recent_successes"]==0 and r["state"]=="unconfirmed",r
        samples.append({"at_ms":observed,"ages_ms":ages,"recovery":r})
        time.sleep(0.5)
    after=history()
    assert all(ids.issubset(after[table]) for table,ids in before.items())
    evidence={"counts_before":{t:len(ids) for t,ids in before.items()},
        "counts_after":{t:len(ids) for t,ids in after.items()},"history_preserved":True,
        "max_report_age_ms":max(age for s in samples for rows in s["ages_ms"].values() for age in rows.values()),
        "command":command["id"],"task":completion,"trace":trace,"samples":samples}
    path=DEMO/".native/evidence/retained-history.json"
    path.write_text(json.dumps(evidence,indent=2))
    print(json.dumps({"passed":"retained history indexed freshness, exact recovery witness and native expiry",
        "counts":evidence["counts_after"],"max_report_age_ms":evidence["max_report_age_ms"],
        "history_preserved":True,"evidence":str(path)},indent=2))


if __name__=="__main__":
    main()
