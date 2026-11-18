#!/usr/bin/env python3
"""Drasi Server lifecycle gate on the demo graph. Owns Server: run it with Server stopped
(the AI service, containers and UI keep running). Covers bootstrap from populated tables,
change capture, a restart that must reconstruct every answer including changes made while
Server was down, an approval recorded while Server was down, and replication reconnect.
Resets this copy's demo; leaves Server stopped."""
import importlib.util
import http.client
import json
import os
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ops"))
import settings  # noqa: E402
import queue
import socket
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request

DEMO = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("runtime", DEMO / "ops/native-runtime.py")
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)
BASE = settings.url("DRASI_API")+"/api/v1/instances/ai-ops"
EVIDENCE = DEMO / ".native/evidence"


def api(path):
    with urllib.request.urlopen(BASE + path, timeout=5) as response:
        return json.load(response)["data"]


class Attachment:
    def __init__(self, name):
        self.connection = http.client.HTTPConnection("127.0.0.1", settings.port("DRASI_API"), timeout=35)
        self.connection.request("GET", "/api/v1/instances/ai-ops/queries/" + name + "/attach")
        self.response = self.connection.getresponse()
        assert self.response.status == 200, self.response.read()
        self.socket = self.connection.sock
        self.closed = False
        self.events = queue.Queue()
        self.rows = {}
        self.history = []
        self.thread = threading.Thread(target=self.read, daemon=True)
        self.thread.start()
        kind, payload = self.event()
        assert kind == "snapshot", (kind, payload)
        self.apply(kind, payload)

    def read(self):
        try:
            kind, data = "message", []
            while line := self.response.readline():
                line = line.decode().rstrip("\r\n")
                if not line:
                    if data:
                        self.events.put((kind, json.loads("\n".join(data))))
                    kind, data = "message", []
                elif line.startswith("event:"):
                    kind = line[6:].strip()
                elif line.startswith("data:"):
                    data.append(line[5:].strip())
            self.events.put(("closed", None))
        except (OSError, ValueError) as error:
            self.events.put(("error", str(error)))

    def event(self, timeout=20):
        try:
            kind, payload = self.events.get(timeout=timeout)
        except queue.Empty as error:
            raise AssertionError(f"Timed out waiting for query changes; current rows: {self.rows}") from error
        assert kind not in ("error", "closed"), (kind, payload)
        self.history.append({"event": kind, "data": payload})
        return kind, payload

    def apply(self, kind, payload):
        if kind == "snapshot":
            self.rows = {row["id"]: row for row in payload}
            return
        for change in payload["results"]:
            operation = change["type"]
            if operation == "DELETE":
                self.rows.pop(change["data"]["id"], None)
            elif operation in ("ADD", "UPDATE"):
                row = change.get("after", change["data"])
                self.rows[row["id"]] = row
            else:
                raise AssertionError(f"Unexpected result operation: {change}")

    def until(self, predicate, timeout=20):
        end = time.monotonic() + timeout
        while not predicate(self.rows):
            remaining = end - time.monotonic()
            assert remaining > 0, self.rows
            self.apply(*self.event(remaining))
        return self.rows

    def close(self):
        # Closing the connection interrupts its background reader before restart.
        if self.closed:
            return
        self.closed = True
        self.socket.shutdown(socket.SHUT_RDWR)
        self.thread.join(timeout=3)
        self.response.close()
        self.connection.close()


class Server:
    def __init__(self):
        self.process = None
        self.log = None

    def start(self):
        self.log = (EVIDENCE / "server.log").open("a")
        self.process = subprocess.Popen(
            [sys.executable, str(DEMO / "ops/native-runtime.py"), "run"],
            cwd=DEMO, stdout=self.log, stderr=subprocess.STDOUT,
        )
        end = time.monotonic() + 45
        while time.monotonic() < end:
            if self.process.poll() is not None:
                raise AssertionError((EVIDENCE / "server.log").read_text()[-12000:])
            try:
                api("/queries/plan-results")
                return
            except (urllib.error.URLError, ConnectionError):
                time.sleep(0.2)
        raise AssertionError("Server startup deadline exceeded")

    def stop(self):
        if self.process and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=20)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
                raise AssertionError("Server did not shut down cleanly")
        if self.log:
            self.log.close()


def main():
    # Do not stop or borrow another session's running Server.
    for name in ("DRASI_API", "DRASI_METRICS", "DRASI_SSE"):
        with socket.socket() as listener:
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind(("127.0.0.1", settings.port(name)))
    import drasi_outputs as d
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "server.log").write_text("")
    runtime.verify_bundle()
    passed, histories = [], {}
    server = Server()
    outputs = ["action", "capacity", "demand", "deployments", "last-approval", "overdue", "policy",
               "proposed", "recovery", "resilience", "running"]

    def ready():
        return all(d.metric("drasi_reaction_input_ready", output=o) == 1 for o in outputs)

    try:
        server.start()
        d.reset()
        d.wait(ready, 60, "every Prometheus reaction input ready")
        with d.DeskStream(["plan-results", "tile-action", "tile-resilience"]) as stream:
            plan = stream.until("plan-results", lambda r: r and r[0]["state"] == "feasible")[0]
            stream.until("tile-action", lambda r: r and r[0]["value"] == 0)
            stream.until("tile-resilience", lambda r: r and r[0]["value"] == 1)
        passed.append("populated PostgreSQL bootstrap -> context queries -> Rego and optimizer -> result and "
                      "tile queries -> Prometheus and SSE reactions")

        d.configure(capacity={"EU-B": 90000})
        d.metric_until("drasi_deployment_capacity_tpm", lambda v: v == 90000, deployment="EU-B")
        passed.append("a scenario change propagates through change capture to the reaction")

        d.control("fail")
        d.metric_until("drasi_deployment_health_status", lambda v: v == 2, deployment="EU-A")
        with d.DeskStream(["plan-results"]) as stream:
            plan = stream.until("plan-results", lambda r: r and r[0]["state"] == "feasible" and r[0]["moves"] > 0)[0]
        server.stop()
        d.configure(capacity={"EU-B": 95000})
        body, _ = d.submit()
        approval = d.approve(plan)
        server.start()
        d.wait(ready, 60, "every input ready after restart")
        d.metric_until("drasi_deployment_capacity_tpm", lambda v: v == 95000, deployment="EU-B")
        d.metric_until("drasi_deployment_health_status", lambda v: v == 2, deployment="EU-A")
        with d.DeskStream(["plan-results", "tile-action", "tile-last-approval", "overdue-tasks"]) as stream:
            stream.until("overdue-tasks", lambda r: any(row["id"] == body["id"] for row in r), 30)
            row = stream.until("tile-last-approval", lambda r: r and r[0]["approval"] == approval
                               and r[0]["status"] != "pending", 45)[0]
            assert row["status"] == "rejected" and row["reason"] == "The facts changed since this plan was proposed", row
            stream.until("tile-action", lambda r: r and r[0]["value"] == 1)
            histories["restart"] = stream.history
        passed.append("Server restart reconstructs every answer from the tables, including changes made while "
                      "it was down; an approval recorded while it was down is decided after restart (here "
                      "rejected, because capacity changed after the plan was proposed)")

        runtime.sql(f"SELECT pg_terminate_backend(active_pid) FROM pg_replication_slots "
                    f"WHERE slot_name='{settings.SLOT}' AND active_pid IS NOT NULL;")
        d.configure(capacity={"EU-B": 100000})
        d.metric_until("drasi_deployment_capacity_tpm", lambda v: v == 100000, 60, deployment="EU-B")
        passed.append("PostgreSQL replication reconnect resumes change capture")

        d.reset()
    finally:
        server.stop()
        (EVIDENCE / "events.json").write_text(json.dumps(histories, indent=2) + "\n")
        (EVIDENCE / "checks.json").write_text(json.dumps({"passed": passed}, indent=2) + "\n")
    print(json.dumps({"passed": passed}, indent=2))


if __name__ == "__main__":
    main()
