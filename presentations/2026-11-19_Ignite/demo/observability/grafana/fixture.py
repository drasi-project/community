#!/usr/bin/env python3
"""Serve fixture /metrics in the Prometheus reaction's format, to check the dashboard without Drasi.

    python3 observability/grafana/fixture.py [--port N] [--workload] [state]
    curl -X POST 127.0.0.1:N/state/<state>      # switch state

The port defaults to IGNITE_DRASI_METRICS_PORT, which Prometheus already scrapes, so
never run it while Drasi Server runs. --workload also serves the AI service's
metrics (calls, answer time, queue) to match each state, for a Prometheus that
doesn't scrape the real AI service; never use it beside the real one.

States follow the talk (plan/contracts.md sections 9 and 16): healthy, failure (EU-A
failed, a change proposed and awaiting approval), approved (applying), applied
(verifying), verified, rejected (the facts changed: no change fits), no-plan, at-risk
(healthy, but losing EU-B leaves support nowhere to run), unknown (Drasi answered -1),
not-ready (inputs not bootstrapped: no series) and unreachable (scrape fails).
"""
import argparse
import json
import os
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "../../ops"))
import settings  # noqa: E402

# The reaction's outputs and HELP texts, from its configuration.
CONFIG = json.load(open(os.path.join(HERE, "../../plugins/prometheus/examples/ignite.json")))
OUTPUTS = list(CONFIG["outputs"])
HELP = {m["metric"]: m["help"] for ms in CONFIG["metrics"].values() for m in ms}
DEPLOYMENTS = ("EU-A", "EU-B", "US-C")
MOVE = "Move summaries to US-C and support to EU-B"


def state(status=(0, 0, 0), capacity=(100000, 100000, 100000), overdue=0, change=None,
          action=(0, "None"), readiness=(1, "Survives losing any one deployment", "", "")):
    """Series in the reaction's shape; None or 0 counts and "nothing yet" states export no series."""
    s = []
    if overdue: s.append(("ignite_drasi_overdue_requests", {"workload": "support"}, overdue))
    for d, st, cap in zip(DEPLOYMENTS, status, capacity):
        s.append(("ignite_drasi_deployment_status", {"deployment": d}, st))
        s.append(("ignite_drasi_deployment_capacity_tpm", {"deployment": d}, cap))
    if change is not None: s.append(("ignite_drasi_change_status", {}, change))
    s.append(("ignite_drasi_recommended_action", {"text": action[1]}, action[0]))
    value, text, lost, workload = readiness
    s.append(("ignite_drasi_failover_readiness", {k: v for k, v in (("text", text), ("lost", lost), ("workload", workload)) if v}, value))
    return s


DEGRADED = (2, "Already degraded", "", "")
STATES = {
    "healthy": state(),
    "failure": state(status=(2, 0, 0), overdue=4, action=(1, MOVE), readiness=DEGRADED),
    "approved": state(status=(2, 0, 0), overdue=5, change=2, action=(1, MOVE), readiness=DEGRADED),
    "applied": state(status=(2, 0, 0), overdue=2, change=3, readiness=DEGRADED),
    "verified": state(status=(2, 0, 0), change=4, readiness=DEGRADED),
    "rejected": state(status=(2, 0, 0), overdue=3, change=5, action=(2, "No change fits policy and capacity"), readiness=DEGRADED),
    "no-plan": state(status=(2, 0, 1), overdue=3, action=(2, "No change fits policy and capacity"), readiness=DEGRADED),
    "at-risk": state(capacity=(50000, 100000, 100000), change=4,
                     action=(3, "Add capacity: losing EU-B leaves support nowhere to run"),
                     readiness=(0, "At risk: losing EU-B leaves support nowhere to run", "EU-B", "support")),
    "unknown": state(status=(0, -1, -1), change=-1, action=(-1, "Unknown"), readiness=(-1, "Unknown", "", "")),
    "not-ready": None,
    "unreachable": None,
}

# --workload: calls per second (support ok, support failed, summaries ok, summaries failed),
# support queue depth and the support answer time (seconds) per state.
TRAFFIC = {"healthy": (2.0, 0, 3.0, 0, 1, 2.5), "failure": (0, 2.0, 3.0, 0, 9, None), "approved": (0, 2.0, 3.0, 0, 11, None),
           "applied": (2.5, 0, 3.0, 0, 3, 14.0), "verified": (2.0, 0, 3.0, 0, 1, 3.5), "rejected": (0, 2.0, 3.0, 0, 9, None),
           "no-plan": (0, 2.0, 0, 3.0, 9, None), "at-risk": (2.0, 0, 3.0, 0, 1, 2.5), "unknown": (2.0, 0, 3.0, 0, 1, 2.5),
           "not-ready": (2.0, 0, 3.0, 0, 1, 2.5), "unreachable": (2.0, 0, 3.0, 0, 1, 2.5)}
BUCKETS = (1.0, 2.0, 3.0, 5.0, 10.0, 15.0, 20.0, 30.0)


class Workload:
    """Counters that grow at the current state's rates."""
    def __init__(self):
        self.lock, self.at = threading.Lock(), time.time()
        self.calls = {k: 0.0 for k in ("support,200", "support,500", "batch,200", "batch,500")}
        self.answers = {b: 0.0 for b in BUCKETS + (float("inf"),)}

    def advance(self, name):
        with self.lock:
            now, rates = time.time(), TRAFFIC[name]
            dt, self.at = now - self.at, now
            for key, rate in zip(self.calls, rates[:4]):
                self.calls[key] += rate * dt
            if rates[5] is not None:
                for b in self.answers:
                    if b >= rates[5]: self.answers[b] += rates[0] * dt

    def render(self, name):
        self.advance(name)
        lines = ["# TYPE ignite_workload_requests_total counter"]
        lines += [f'ignite_workload_requests_total{{workload="{k.split(",")[0]}",status="{k.split(",")[1]}",endpoint="fixture"}} {int(v)}'
                  for k, v in self.calls.items()]
        lines += ["# TYPE ignite_workload_queue_depth gauge", f'ignite_workload_queue_depth{{workload="support"}} {TRAFFIC[name][4]}',
                  "# TYPE ignite_workload_answer_seconds histogram"]
        lines += [f'ignite_workload_answer_seconds_bucket{{workload="support",le="{"+Inf" if b == float("inf") else b}"}} {int(v)}'
                  for b, v in self.answers.items()]
        return lines


def render(name):
    ready = 0 if name == "not-ready" else 1
    lines, seen = [], set()
    for metric, labels, value in sorted(STATES[name] or [], key=lambda s: s[0]):
        if metric not in seen:
            seen.add(metric)
            lines += [f"# HELP {metric} {HELP[metric]}", f"# TYPE {metric} gauge"]
        label = ",".join(f'{k}="{v}"' for k, v in labels.items())
        lines.append(f"{metric}{{{label}}} {value}" if label else f"{metric} {value}")
    lines += ["# HELP drasi_reaction_input_ready 1 once the output's snapshot is loaded, 0 before or after its input fails.",
              "# TYPE drasi_reaction_input_ready gauge"]
    lines += [f'drasi_reaction_input_ready{{output="{o}"}} {ready}' for o in OUTPUTS]
    if Handler.workload: lines += Handler.workload.render(name)
    return "\n".join(lines) + "\n"


class Handler(BaseHTTPRequestHandler):
    current = "healthy"
    workload = None

    def do_GET(self):
        if self.path != "/metrics" or Handler.current == "unreachable":
            if Handler.workload: Handler.workload.advance(Handler.current)
            return self.send_error(404 if self.path != "/metrics" else 503)
        body = render(Handler.current).encode()
        self.send_response(200)
        self.send_header("content-type", "text/plain; version=0.0.4")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        name = self.path.removeprefix("/state/")
        if name not in STATES:
            return self.send_error(404, f"states: {', '.join(STATES)}")
        if Handler.workload: Handler.workload.advance(Handler.current)
        Handler.current = name
        self.send_response(204)
        self.end_headers()

    def log_message(self, *_):
        pass


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=settings.port("DRASI_METRICS"))
    parser.add_argument("--workload", action="store_true", help="also serve the AI service's metrics for each state")
    parser.add_argument("state", nargs="?", default="healthy", choices=list(STATES))
    args = parser.parse_args()
    Handler.current = args.state
    Handler.workload = Workload() if args.workload else None
    print(f"fixture /metrics on 127.0.0.1:{args.port}, state {args.state}", flush=True)
    ThreadingHTTPServer(("127.0.0.1", args.port), Handler).serve_forever()
