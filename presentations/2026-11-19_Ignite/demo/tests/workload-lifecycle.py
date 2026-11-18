#!/usr/bin/env python3
"""Process lifecycle gate: Drasi's answers while the application or Server is down.

Owns both processes: run it with Drasi Server and the AI service stopped (containers and UI keep
running); it leaves both stopped. Every answer is read from Drasi (Prometheus and SSE reactions,
Server's query results); the AI service is only the application and its simulated systems."""
import json
import signal
import socket
import subprocess
import time

import drasi_outputs as d
from drasi_outputs import settings, native, runtime, DEMO


class Service:
    """The AI service, started as `ops/workload.py run` does."""

    def __init__(self):
        self.process = None
        self.log = None

    def start(self):
        self.log = (d.EVIDENCE / "lifecycle-workload.log").open("a")
        self.process = subprocess.Popen([str(runtime.WORKLOADS), "serve"], cwd=DEMO,
                                        stdout=self.log, stderr=subprocess.STDOUT)

        def ready():
            assert self.process.poll() is None, "AI service exited; see lifecycle-workload.log"
            try:
                return d.health()
            except OSError:
                return False
        d.wait(ready, 30, "AI service ready")

    def stop(self):
        if self.process and self.process.poll() is None:
            self.process.send_signal(signal.SIGINT)
            self.process.wait(timeout=20)
            assert self.process.returncode == 0, self.process.returncode
        if self.log:
            self.log.close()


def statuses():
    return {dep: d.metric("drasi_deployment_health_status", deployment=dep) for dep in ["EU-A", "EU-B", "US-C"]}


def main():
    for name in ("DRASI_API", "DRASI_METRICS", "DRASI_SSE", "WORKLOAD",
                 "DEPLOYMENT_EU_A", "DEPLOYMENT_EU_B", "DEPLOYMENT_US_C"):
        with socket.socket() as listener:
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind(("127.0.0.1", settings.port(name)))
    d.EVIDENCE.mkdir(parents=True, exist_ok=True)
    server, service = native.Server(), Service()
    checks = []
    try:
        server.start()
        service.start()
        d.reset()
        d.control("fail")
        body, _ = d.submit()
        d.wait(lambda: any(a["status"] == 503 for a in d.task(body["id"])["attempts"]), 20, "HTTP 503")
        service.stop()
        before = d.sql(f"SELECT row_to_json(t) FROM ai_requests t WHERE id='{body['id']}';")
        with d.DeskStream(["overdue-tasks", "plan-results", "tile-action"]) as stream:
            stream.until("overdue-tasks", lambda r: any(row["id"] == body["id"] for row in r), 30)
            d.wait(lambda: set(statuses().values()) == {-1}, 20, "every deployment Unknown")
            stream.until("plan-results", lambda r: r and r[0]["state"] == "unknown")
            stream.until("tile-action", lambda r: r and r[0]["value"] == -1)
        assert before == d.sql(f"SELECT row_to_json(t) FROM ai_requests t WHERE id='{body['id']}';")
        checks.append("application stopped: Drasi's timers alone make the request overdue and every deployment, "
                      "the plan and Action needed Unknown (never healthy)")

        server.stop()
        server.start()
        with d.DeskStream(["overdue-tasks", "tile-action"]) as stream:
            assert any(row["id"] == body["id"] for row in stream.get("overdue-tasks"))
            stream.until("tile-action", lambda r: r and r[0]["value"] == -1)
        d.wait(lambda: set(statuses().values()) == {-1}, 20, "every deployment Unknown after restart")
        checks.append("Server restart with the application still down reconstructs overdue and Unknown")

        service.start()
        d.control("restore")
        d.wait(lambda: d.task(body["id"])["task"]["completed_ms"], 30, "restored request completes")
        d.wait(lambda: set(statuses().values()) == {0}, 20, "every deployment Healthy")
        with d.DeskStream(["overdue-tasks", "tile-action"]) as stream:
            stream.until("overdue-tasks", lambda r: all(row["id"] != body["id"] for row in r))
            stream.until("tile-action", lambda r: r and r[0]["value"] == 0)
        checks.append("application back: reports resume, the request completes and leaves Drasi's overdue list")

        # A request misses its deadline and completes while Drasi is down.
        server.stop()
        d.control("fail")
        offline, _ = d.submit()
        d.wait(lambda: any(a["status"] == 503 for a in d.task(offline["id"])["attempts"]), 20, "HTTP 503")
        deadline = d.task(offline["id"])["task"]["deadline_ms"]
        time.sleep(max(0, (deadline - int(time.time() * 1000)) / 1000) + 0.5)
        d.control("restore")
        d.wait(lambda: d.task(offline["id"])["task"]["completed_ms"], 30, "offline request completes")
        server.start()
        history = d.Attachment("deadline-history")
        try:
            assert offline["id"] in history.rows
        finally:
            history.close()
        with d.DeskStream(["overdue-tasks"]) as stream:
            assert all(row["id"] != offline["id"] for row in stream.get("overdue-tasks"))
        checks.append("deadline crossed and request answered while Server was down: Drasi's deadline history "
                      "reconstructs the late answer on bootstrap; it is not overdue")

        # Stop between the deployment's answer and the gateway recording it.
        uncertain, _ = d.submit()
        d.wait(lambda: any(a["status"] == 200 for a in d.task(uncertain["id"])["attempts"]), 20, "HTTP 200")
        service.stop()
        record = json.loads(d.sql(f"SELECT row_to_json(t) FROM ai_requests t WHERE id='{uncertain['id']}';"))
        receipts = lambda: d.sql(f"SELECT count(*) FROM model_responses WHERE request_id='{uncertain['id']}';")
        if record["completed_ms"] is None:
            assert receipts() == "1"
            service.start()
            d.wait(lambda: d.task(uncertain["id"])["task"]["completed_ms"], 30, "reconciled request")
            assert receipts() == "1" and len(d.task(uncertain["id"])["attempts"]) == 1
            checks.append("gateway restart reconciles an answer it received but had not recorded, without asking again")
        else:
            service.start()
            checks.append("gateway restart (the receipt/record boundary was not caught this run)")

        d.control("fail")
        retired, _ = d.submit()
        old = d.health()["run_epoch"]
        d.reset()
        current = d.health()
        assert current["run_epoch"] != old and current["revision"] == 1 and current["pending"] == 0
        d.api("/tasks", retired, expected=409)
        task = d.task(retired["id"])["task"]
        assert task["cancelled_ms"] is not None and task["completed_ms"] is None
        with d.DeskStream(["overdue-tasks"]) as stream:
            assert stream.get("overdue-tasks") == []
        checks.append("reset retires pending requests, fences the old run and Drasi starts the new run clean")
        path = d.save("workload-lifecycle.json", {"checks": checks})
        print(json.dumps({"passed": checks, "evidence": str(path)}, indent=2))
    finally:
        service.stop()
        server.stop()


if __name__ == "__main__":
    main()
