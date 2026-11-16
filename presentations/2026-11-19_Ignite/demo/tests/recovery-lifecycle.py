#!/usr/bin/env python3
"""Owns processes. Real crash/approval and publication-outage boundaries; retains all data."""
import importlib.util
import json
import socket
import subprocess
import uuid
from pathlib import Path

DEMO=Path(__file__).resolve().parents[1]
def load(name,file):
    spec=importlib.util.spec_from_file_location(name,DEMO/"tests"/file)
    m=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m
recovery=load("recovery","recovery-integration.py")
life=load("life","workload-lifecycle.py")
work=recovery.work
native=work.native


def crash(service):
    service.process.kill()
    service.process.wait(timeout=10)
    service.log.close()


def command(id):
    return json.loads(native.runtime.sql(f"SELECT to_jsonb(c) FROM ignite_commands c WHERE id='{id}'",tuples=True))


def committed_crash(service):
    # Capture an actual accepted approval before its independent 500ms adoption pass.
    # Never seed a command/receipt or turn a missed boundary into passing evidence.
    for _ in range(4):
        recovery.reset()
        work.control("fail")
        d=work.wait(recovery.candidate)
        task,trace=work.submit()
        id=str(uuid.uuid4())
        recovery.approve(d,id)
        crash(service)
        if command(id)["status"]=="committed":
            return id,d,task,trace
        service.start()
    raise AssertionError("Could not capture committed-before-adopted boundary in four attempts")


def main():
    for port in (18421,18422,18500,18531,18532,18533):
        with socket.socket() as s:
            s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1)
            s.bind(("127.0.0.1",port))
    server,service=native.Server(),life.Service()
    checks=[]
    try:
        server.start()
        service.start()
        work.wait(lambda:work.api("/decisions").get("candidate_current"))
        id,d,task,_=committed_crash(service)
        service.start()
        work.wait(lambda:recovery.command(id)["status"] in ["gateway_applied","recovery_confirmed"])
        work.wait(lambda:work.api("/tasks/"+task["id"])["task"]["completed_ms"])
        assert recovery.approve(d,id)["duplicate"]
        service.stop()
        service.start()
        assert int(native.runtime.sql(f"SELECT count(*) FROM ignite_audit WHERE command_id='{id}' AND action='gateway_applied'",tuples=True))==1
        checks.append("real crash after approval commit reconciles once on restart; acknowledged or uncertain adoption never reapproves")

        id,d,task,_=committed_crash(service)
        native.runtime.sql("""UPDATE ignite_model SET revision=revision+1,
          payload=jsonb_set(jsonb_set(payload,'{policy,destinations,support}','["EU-A"]'),
          '{policy,revision}',to_jsonb((payload#>>'{policy,revision}')::bigint+1)) WHERE id='active';""")
        service.start()
        work.wait(lambda:recovery.command(id)["status"]=="rejected")
        assert int(native.runtime.sql(f"SELECT count(*) FROM ignite_receipts WHERE task_id='{task['id']}'",tuples=True))==0
        assert recovery.command(id)["applied_ms"] is None
        checks.append("policy changes after durable approval but before gateway adoption reject stale command on restart")

        recovery.reset()
        work.control("fail")
        d=work.wait(recovery.candidate)
        server.stop()
        work.wait(lambda:not work.api("/decisions")["streams_attached"])
        recovery.approve(d,expected=409)
        server.start()
        d=work.wait(recovery.candidate)
        checks.append("disconnected Server rejects approval; populated restart snapshots restore current candidate without fake input updates")

        # Stop only this demo's known publication destinations, not PostgreSQL or unrelated containers.
        subprocess.run(["podman","stop","ignite-observability-grafana-1","ignite-observability-langfuse-web-1"],check=True,capture_output=True)
        task,trace=work.submit()
        id=str(uuid.uuid4())
        recovery.approve(d,id)
        work.wait(lambda:recovery.command(id)["status"]=="recovery_confirmed")
        def failed_targets():
            rows=[p for p in work.api("/decisions")["publications"] if p["last_error"]]
            return rows if {p["target"] for p in rows}=={"grafana","langfuse"} else None
        failures=work.wait(failed_targets)
        service.stop()
        service.start()
        assert work.api("/decisions")["publications"]
        subprocess.run(["podman","start","ignite-observability-grafana-1","ignite-observability-langfuse-web-1"],check=True,capture_output=True)
        work.wait(lambda:not work.api("/decisions")["publications"],timeout=120)
        count=int(native.runtime.sql(f"SELECT attempts FROM ignite_publications WHERE id='{id}-recovered'",tuples=True))
        assert count>1
        env=recovery.obs.environment()
        auth=recovery.base64.b64encode(f"admin:{env['GRAFANA_ADMIN_PASSWORD']}".encode()).decode()
        annotations=recovery.obs.get("http://127.0.0.1:13000/api/annotations?"+
            recovery.urllib.parse.urlencode({"tags":f"{id}-recovered"}),auth)
        assert len(annotations)==1
        score_url="http://127.0.0.1:13001/api/public/v3/scores?"+recovery.urllib.parse.urlencode({
            "traceId":trace["trace_id"],"name":"drasi_recovery","fields":"subject"})
        scores,_=recovery.obs.wait_for("redelivered exact recovery score",lambda:
            recovery.obs.get(score_url,env["LANGFUSE_AUTH_BASE64"])["data"])
        assert len(scores)==1 and scores[0]["subject"]=={"kind":"trace","id":trace["trace_id"]}
        checks.append("real Grafana/Langfuse outages expose durable retriable publication errors, survive service restart and redeliver")
        recovery.reset()
        path=DEMO/".native/evidence/recovery-lifecycle.json"
        path.write_text(json.dumps({"checks":checks,"publication_failures":failures,"recovered_annotation_attempts":count,
            "last_task":task,"trace":trace,"redelivered_annotations":annotations,"redelivered_scores":scores},indent=2))
        print(json.dumps({"passed":checks,"evidence":str(path)},indent=2))
    finally:
        subprocess.run(["podman","start","ignite-observability-grafana-1","ignite-observability-langfuse-web-1"],check=True,capture_output=True)
        service.stop()
        server.stop()


if __name__=="__main__":
    main()
