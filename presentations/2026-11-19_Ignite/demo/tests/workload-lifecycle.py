#!/usr/bin/env python3
"""Owns demo subprocesses; requires stopped Server/workload. No volume or slot deletion."""
import importlib.util
import json
import signal
import socket
import subprocess
import time
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ops"))
import settings  # noqa: E402

DEMO = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("work", DEMO / "tests/workload-integration.py")
work = importlib.util.module_from_spec(spec)
spec.loader.exec_module(work)
native = work.native


def sql_value(query):
    return json.loads(native.runtime.sql(query, tuples=True))


class Service:
    def __init__(self):
        self.process = None
        self.log = None

    def start(self):
        self.log = (DEMO / ".native/evidence/lifecycle-workload.log").open("a")
        self.process = subprocess.Popen([str(native.runtime.WORKLOADS), "serve"],
            cwd=DEMO, stdout=self.log, stderr=subprocess.STDOUT)
        def ready():
            assert self.process.poll() is None, "workload exited; inspect lifecycle-workload.log"
            try:
                return work.api("/health")
            except (OSError, work.urllib.error.URLError):
                return False
        work.wait(ready, timeout=30)

    def stop(self):
        if self.process and self.process.poll() is None:
            self.process.send_signal(signal.SIGINT)
            self.process.wait(timeout=20)
            assert self.process.returncode == 0
        if self.log:
            self.log.close()


def main():
    for port in [settings.port(name) for name in ("DRASI_API","DRASI_ALERT","WORKLOAD","DEPLOYMENT_EU_A","DEPLOYMENT_EU_B","DEPLOYMENT_US_C")]:
        with socket.socket() as listener:
            listener.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1)
            listener.bind(("127.0.0.1",port))
    native.EVIDENCE.mkdir(parents=True,exist_ok=True)
    native.runtime.sql((DEMO / "database/reset.sql").read_text())
    server, service = native.Server(), Service()
    attachments, checks, histories = [], [], {}
    try:
        server.start()
        service.start()
        work.wait(lambda: work.api("/health")["continuous_context"]=="ready")
        work.control("fail")
        work.control("pause-reports","EU-B")
        body, trace = work.submit()
        work.wait(lambda:any(a["status"]==503 for a in work.api("/tasks/"+body["id"])["attempts"]))
        service.stop()
        before = native.runtime.sql(f"SELECT row_to_json(t) FROM ignite_tasks t WHERE id='{body['id']}';")
        overdue, fresh = native.Attachment("overdue-tasks"), native.Attachment("endpoint-freshness")
        attachments += [overdue,fresh]
        overdue.until(lambda rows: body["id"] in rows, timeout=25)
        fresh.until(lambda rows:len(rows)==3 and all(r["status"]=="unknown" for r in rows.values()),timeout=15)
        assert before == native.runtime.sql(f"SELECT row_to_json(t) FROM ignite_tasks t WHERE id='{body['id']}';")
        histories["application_down_timers"] = {"overdue":overdue.history,"freshness":fresh.history}
        checks.append("application completely stopped: native timers alone derive overdue and all reports unknown")
        for a in attachments: a.close()
        attachments.clear()
        server.stop()
        server.start()
        overdue, fresh = native.Attachment("overdue-tasks"), native.Attachment("endpoint-freshness")
        attachments += [overdue,fresh]
        assert body["id"] in overdue.rows
        assert len(fresh.rows)==3 and all(r["status"]=="unknown" for r in fresh.rows.values())
        checks.append("Server restart snapshot reconstructs overdue and stale, never healthy/empty")
        service.start()
        work.wait(lambda:body["id"] in work.api("/context")["queries"].get("breach-history",{}))
        work.control("restore")
        work.control("resume-reports","EU-B")
        work.wait(lambda:work.api("/tasks/"+body["id"])["task"]["completed_ms"])
        overdue.until(lambda rows:body["id"] not in rows)
        work.wait(lambda:work.api("/health")["continuous_context"]=="ready")
        checks.append("observer reattach persists derived breach and restored request completes")

        # A task can miss its deadline and finish while Drasi is offline; immutable timestamps
        # allow the history query to reconstruct the breach rather than erase it on restart.
        for a in attachments: a.close()
        attachments.clear()
        server.stop()
        work.wait(lambda:work.api("/health")["continuous_context"]=="unknown")
        work.control("fail")
        offline, _ = work.submit()
        work.wait(lambda:any(a["status"]==503 for a in work.api("/tasks/"+offline["id"])["attempts"]))
        deadline = work.api("/tasks/"+offline["id"])["task"]["deadline_ms"]
        time.sleep(max(0,(deadline-int(time.time()*1000))/1000)+0.4)
        work.control("restore")
        work.wait(lambda:work.api("/tasks/"+offline["id"])["task"]["completed_ms"])
        server.start()
        work.wait(lambda:offline["id"] in work.api("/context")["queries"].get("breach-history",{}))
        assert offline["id"] not in work.api("/context")["queries"]["overdue-tasks"]
        checks.append("deadline crossed and task completed during Server outage: bootstrap reconstructs retained breach")

        # Stop between the real HTTP receipt and gateway's next completion pass.
        uncertain, _ = work.submit()
        work.wait(lambda:any(a["status"]==200 for a in work.api("/tasks/"+uncertain["id"])["attempts"]))
        service.stop()
        record = sql_value(f"SELECT row_to_json(t) FROM ignite_tasks t WHERE id='{uncertain['id']}'")
        assert record["completed_ms"] is None, "missed receipt/acknowledgement boundary; rerun gate"
        assert sql_value(f"SELECT to_json(count(*)) FROM ignite_receipts WHERE task_id='{uncertain['id']}'")==1
        service.start()
        work.wait(lambda:work.api("/tasks/"+uncertain["id"])["task"]["completed_ms"])
        assert sql_value(f"SELECT to_json(count(*)) FROM ignite_receipts WHERE task_id='{uncertain['id']}'")==1
        assert len(work.api("/tasks/"+uncertain["id"])["attempts"])==1
        checks.append("gateway restart reconciles a real unacknowledged receipt without repeated inference")

        native.runtime.sql(f"SELECT pg_terminate_backend(active_pid) FROM pg_replication_slots WHERE slot_name='{settings.SLOT}' AND active_pid IS NOT NULL;")
        work.control("pause-reports","EU-B")
        work.wait(lambda:work.api("/context")["queries"].get("endpoint-freshness",{}).get("EU-B",{}).get("status")=="unknown",timeout=20)
        work.control("resume-reports","EU-B")
        work.wait(lambda:work.api("/health")["continuous_context"]=="ready")
        checks.append("CDC transient reconnect resumes real reports and timer results")

        work.control("fail")
        retired, _ = work.submit()
        old_epoch = work.api("/health")["run_epoch"]
        native.runtime.sql((DEMO / "database/reset.sql").read_text())
        work.wait(lambda:work.api("/health")["continuous_context"]=="ready")
        current = work.api("/health")
        assert current["run_epoch"] != old_epoch and current["revision"]==1 and current["pending"]==0
        work.api("/tasks",retired,expected=409)
        record = work.api("/tasks/"+retired["id"])["task"]
        assert record["cancelled_ms"] is not None and record["completed_ms"] is None
        assert not work.api("/context")["queries"]["task-context"]
        assert body["id"] in work.api("/context")["queries"]["breach-history"]
        checks.append("explicit reset retires pending tasks, preserves history, fences old epoch and waits for coherent new reports")

        # Public batch may run in the US; this is a test-only configuration edit, not an approved recovery.
        native.runtime.sql("UPDATE ignite_model SET payload=jsonb_set(payload,'{workloads,1,destination}','\"US-C\"'),revision=revision+1;")
        public, _ = work.submit("batch")
        work.wait(lambda:work.api("/tasks/"+public["id"])["task"]["completed_ms"])
        assert work.api("/tasks/"+public["id"])["task"]["endpoint"]=="US-C"
        assert sql_value("SELECT to_json(count(*)) FROM ignite_receipts WHERE workload='support' AND endpoint='US-C'")==0
        checks.append("unrestricted public batch reaches real US-C simulator; restricted support never does")
        native.runtime.sql((DEMO / "database/reset.sql").read_text())
        work.wait(lambda:work.api("/health")["continuous_context"]=="ready")
        evidence = DEMO / ".native/evidence/workload-lifecycle.json"
        evidence.write_text(json.dumps({"checks":checks,"histories":histories},indent=2))
        print(json.dumps({"passed":checks,"evidence":str(evidence)},indent=2))
    finally:
        for a in attachments: a.close()
        service.stop()
        server.stop()


if __name__=="__main__":
    main()
