#!/usr/bin/env python3
"""Executable Phase 1 gate. Mutates only Ignite fixture records; leaves a reset fixture."""
import importlib.util
import http.client
import json
import os
from pathlib import Path
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
BASE = "http://127.0.0.1:18421/api/v1/instances/ignite"
EVIDENCE = DEMO / ".native/evidence"


def api(path):
    with urllib.request.urlopen(BASE + path, timeout=5) as response:
        return json.load(response)["data"]


class Attachment:
    def __init__(self, name):
        self.connection = http.client.HTTPConnection("127.0.0.1", 18421, timeout=35)
        self.connection.request("GET", "/api/v1/instances/ignite/queries/" + name + "/attach")
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
                api("/queries/capacity-results")
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
    for port in (18421, 18422):
        with socket.socket() as listener:
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind(("127.0.0.1", port))
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "server.log").write_text("")
    runtime.verify_bundle()
    runtime.sql((DEMO / "database/schema.sql").read_text())
    runtime.sql((DEMO / "database/workload.sql").read_text())
    runtime.sql((DEMO / "database/reset.sql").read_text())
    passed, histories, attachments = [], {}, []
    server = Server()
    try:
        server.start()
        context = Attachment("endpoint-context")
        results = Attachment("capacity-results")
        attachments += [context, results]
        context.until(lambda rows: len(rows) == 3)
        results.until(lambda rows: len(rows) == 3)
        assert results.rows["EU-A"]["spare_tpm"] == 40000, results.rows
        assert results.rows["EU-B"]["spare_tpm"] == 20000, results.rows
        assert results.rows["US-C"]["spare_tpm"] == 100000, results.rows
        epoch = results.rows["EU-A"]["run_epoch"]
        passed.append("populated PostgreSQL bootstrap -> context CQ -> native capacity -> result CQ")

        runtime.sql("UPDATE ignite_endpoints SET available=false, revision=revision+1 WHERE id='EU-A';")
        results.until(lambda rows: rows.get("EU-A", {}).get("status") == "unavailable")
        context.until(lambda rows: rows.get("EU-A", {}).get("available") is False)
        assert results.rows["EU-A"]["input_revision"] == 2
        passed.append("CDC update propagates through both CQs and plugin")

        runtime.sql("DELETE FROM ignite_endpoints WHERE id='US-C';")
        results.until(lambda rows: "US-C" not in rows)
        passed.append("CDC delete retracts assessment")
        histories["live"] = results.history
        results.close()
        runtime.sql("UPDATE ignite_endpoints SET budget_tpm=50000, revision=revision+1 WHERE id='EU-B';")
        reconnected = Attachment("capacity-results")
        attachments.append(reconnected)
        reconnected.until(lambda rows: rows.get("EU-B", {}).get("status") == "overcommitted")
        assert set(reconnected.rows) == {"EU-A", "EU-B"}
        histories["reconnect"] = reconnected.history
        passed.append("snapshot-first reconnect replaces state, including missed deletion/update")

        for attachment in attachments:
            attachment.close()
        attachments.clear()
        server.stop()
        runtime.sql("UPDATE ignite_endpoints SET budget_tpm=90000, revision=revision+1 WHERE id='EU-B';")
        server.start()
        restarted = Attachment("capacity-results")
        attachments.append(restarted)
        restarted.until(lambda rows: len(rows) == 2 and rows["EU-B"]["spare_tpm"] == 10000)
        assert restarted.rows["EU-A"]["status"] == "unavailable"
        assert restarted.rows["EU-A"]["run_epoch"] == epoch
        passed.append("Server restart preserves authoritative data and bootstraps downtime changes")

        # Terminate only this demo's actual replication backend, then update while reconnecting.
        runtime.sql("SELECT pg_terminate_backend(active_pid) FROM pg_replication_slots WHERE slot_name='ignite_native' AND active_pid IS NOT NULL;")
        runtime.sql("UPDATE ignite_endpoints SET budget_tpm=95000, revision=revision+1 WHERE id='EU-B';")
        restarted.until(lambda rows: rows["EU-B"]["spare_tpm"] == 15000)
        passed.append("PostgreSQL replication reconnect resumes CDC")

        alerts = Attachment("grafana-alerts")
        attachments.append(alerts)
        for status in ("firing", "resolved"):
            payload = {"groupKey": "native-check", "status": status, "title": "AI capacity", "message": status}
            request = urllib.request.Request(
                "http://127.0.0.1:18422/alerts/grafana",
                data=json.dumps(payload).encode(), headers={"Content-Type": "application/json"},
            )
            with urllib.request.urlopen(request, timeout=5) as response:
                assert 200 <= response.status < 300
            alerts.until(lambda rows: rows.get("native-check", {}).get("status") == status)
        histories["alerts"] = alerts.history
        passed.append("native Grafana webhook firing/resolved -> actual alert CQ")

        runtime.sql((DEMO / "database/reset.sql").read_text())
        restarted.until(lambda rows: len(rows) == 3 and all(row["run_epoch"] != epoch for row in rows.values()))
        assert restarted.rows["EU-A"]["status"] == "within_budget"
        assert restarted.rows["EU-B"]["spare_tpm"] == 20000
        assert restarted.rows["US-C"]["spare_tpm"] == 100000
        assert len({row["run_epoch"] for row in restarted.rows.values()}) == 1
        histories["restart-reset"] = restarted.history
        passed.append("explicit scenario reset changes epoch and restores fixture via CDC")

        for attachment in attachments:
            attachment.close()
        attachments.clear()
        server.stop()
        runtime.sql("DELETE FROM ignite_endpoints;")
        server.start()
        empty = Attachment("capacity-results")
        attachments.append(empty)
        assert empty.rows == {}, empty.rows
        runtime.sql((DEMO / "database/reset.sql").read_text())
        empty.until(lambda rows: len(rows) == 3)
        assert empty.rows["EU-A"]["spare_tpm"] == 40000
        histories["empty-start"] = empty.history
        passed.append("empty bootstrap followed by CDC insertion populates all three assessments")
    finally:
        for attachment in attachments:
            attachment.close()
        server.stop()
        (EVIDENCE / "events.json").write_text(json.dumps(histories, indent=2) + "\n")
        (EVIDENCE / "checks.json").write_text(json.dumps({"passed": passed}, indent=2) + "\n")
    print(json.dumps({"passed": passed}, indent=2))


if __name__ == "__main__":
    main()
