#!/usr/bin/env python3
"""Reaction lifecycle gate: Drasi's reactions when the systems they call are down.

Owns both processes: run it with Drasi Server and the AI service stopped (containers and UI keep
running); it leaves both stopped and Grafana and Langfuse running.
- The gateway reaction (failurePolicy strict, 10 retries within 30 s) applies an approval once the
  gateway configuration API is back.
- Grafana and Langfuse outages do not hold up the gateway reaction or recovery verification; their
  reactions skip what they could not deliver (plan/contracts.md section 10, known limits)."""
import importlib.util
import json
import socket
import subprocess
import time
import uuid

import drasi_outputs as d
from drasi_outputs import settings, native

spec = importlib.util.spec_from_file_location("ignite_workload_lifecycle", d.DEMO / "tests/workload-lifecycle.py")
lifecycle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lifecycle)
recovery_spec = importlib.util.spec_from_file_location("ignite_recovery", d.DEMO / "tests/recovery-integration.py")
recovery = importlib.util.module_from_spec(recovery_spec)
recovery_spec.loader.exec_module(recovery)
OBSERVERS = [settings.container("grafana"), settings.container("langfuse-web")]


def main():
    for name in ("DRASI_API", "DRASI_METRICS", "DRASI_SSE", "WORKLOAD"):
        with socket.socket() as listener:
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind(("127.0.0.1", settings.port(name)))
    d.EVIDENCE.mkdir(parents=True, exist_ok=True)
    server, service = native.Server(), lifecycle.Service()
    checks = []
    try:
        server.start()
        service.start()
        d.reset()
        with d.DeskStream(recovery.ALIASES) as stream:
            d.control("fail")
            plan = recovery.proposed(stream)
            body, _, _ = recovery.stuck_request(stream)
            service.stop()
            # The approval tool is a separate system in a real company; here its record is written
            # directly because the AI service (which plays it) is the system that is down.
            ident = str(uuid.uuid4())
            d.sql(f"""BEGIN; UPDATE ignite_approvals SET latest=false WHERE latest;
                INSERT INTO ignite_approvals (id, run_epoch, candidate_id, basis, revision, status, latest, requested_ms)
                SELECT '{ident}', run_epoch, '{plan['candidate_id']}', '{plan['basis']}', {plan['revision']},
                       'pending', true, (extract(epoch FROM clock_timestamp())*1000)::bigint
                FROM ignite_scenario WHERE singleton; COMMIT;""")
            time.sleep(5)
            assert d.approval(ident)["status"] == "pending"
            service.start()
            recovery.applied(stream, ident, body)
            assert len(json.loads(d.sql("SELECT coalesce(json_agg(id),'[]') FROM ignite_deliveries WHERE "
                                         f"result->>'result'='applied' AND received_ms >= {d.approval(ident)['requested_ms']};"))) == 1
            checks.append("gateway configuration API down when Drasi approved: the strict gateway reaction "
                          "retries and applies the plan once when it is back")

        d.reset()
        subprocess.run(["podman", "stop", *OBSERVERS], check=True, capture_output=True)
        with d.DeskStream(recovery.ALIASES) as stream:
            d.control("fail")
            plan = recovery.proposed(stream)
            body, _, _ = recovery.stuck_request(stream)
            ident = d.approve(plan)
            recovery.applied(stream, ident, body)
            recovery.verified(stream, ident)
        checks.append("with Grafana and Langfuse down, Drasi still applies the approval and verifies "
                      "recovery; the annotation and score reactions skip what they cannot deliver")
        path = d.save("recovery-lifecycle.json", {"checks": checks})
        print(json.dumps({"passed": checks, "evidence": str(path)}, indent=2))
    finally:
        subprocess.run(["podman", "start", *OBSERVERS], check=True, capture_output=True)
        try:
            d.reset()
        except Exception as error:  # leave the processes stopped either way
            print(f"final reset failed: {error}")
        service.stop()
        server.stop()


if __name__ == "__main__":
    main()
