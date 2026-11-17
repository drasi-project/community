#!/usr/bin/env python3
"""Serve fixture /metrics in the Prometheus reaction's format, to check the dashboards without Drasi.

    python3 observability/grafana/fixture.py [--port N] [state]
    curl -X POST 127.0.0.1:N/state/<state>      # switch state

The port defaults to IGNITE_DRASI_METRICS_PORT, which Prometheus already scrapes, so
never run it while Drasi Server runs. States follow the talk (plan/contracts.md
section 9): healthy, failure, applied, verified, no-plan, at-risk, unknown (Drasi
answered -1), not-ready (inputs not bootstrapped: no series) and unreachable (scrape fails).
"""
import argparse
import os
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../ops"))
import settings  # noqa: E402

OUTPUTS = ["overdue", "deployments", "capacity", "demand", "recovery", "action", "resilience",
           "running", "proposed", "policy", "last-approval"]
HELP = {
    "ignite_drasi_overdue_requests": "Requests past their deadline and still unanswered, per workload.",
    "ignite_drasi_deployment_status": "Latest health report: 0 healthy, 1 throttled, 2 failed, -1 unknown.",
    "ignite_drasi_deployment_capacity_tpm": "Configured capacity of each deployment, tokens per minute.",
    "ignite_drasi_capacity_tpm": "Capacity in tokens per minute: total, on healthy deployments, and needed.",
    "ignite_drasi_recovery_state": "Recovery after an applied plan: 0 awaiting evidence, 1 verified, 2 nothing to verify, -1 unknown.",
    "ignite_drasi_action_needed": "0 none, 1 approve proposed plan, 2 no valid plan, 3 at risk, -1 unknown.",
    "ignite_drasi_safe_if_one_fails": "1 yes, 0 no (lost names the first deployment whose loss leaves no plan), 2 already one down, -1 unknown.",
    "ignite_drasi_running_tpm": "Demand of each workload on its configured deployment; negative when policy no longer allows it (held).",
    "ignite_drasi_proposed_tpm": "Demand of each workload on the deployment Drasi's feasible plan assigns.",
    "ignite_drasi_policy_allowed": "Placement policy: 1 allowed, 0 not allowed, -1 unknown.",
    "ignite_drasi_last_approval_state": "Latest approval: 1 applying, 2 applied awaiting evidence, 3 verified, 4 rejected, -1 unknown.",
}
DEPLOYMENTS = ("EU-A", "EU-B", "US-C")
SUPPORT, BATCH = 60000, 80000
POLICY = {("support", "EU-A"): 1, ("support", "EU-B"): 1, ("support", "US-C"): 0,
          ("batch", "EU-A"): 1, ("batch", "EU-B"): 1, ("batch", "US-C"): 1}


def state(status=(0, 0, 0), capacity=(100000, 100000, 100000), overdue=0, recovery=None, action=0,
          safe=(1, None), running=(("support", "EU-A", SUPPORT), ("batch", "EU-B", BATCH)),
          proposed=(), policy=POLICY, approval=None):
    """Series in the reaction's shape; None or 0 counts and "nothing yet" states export no series."""
    s = []
    if overdue: s.append(("ignite_drasi_overdue_requests", {"workload": "support"}, overdue))
    for d, st, cap in zip(DEPLOYMENTS, status, capacity):
        s.append(("ignite_drasi_deployment_status", {"deployment": d}, st))
        s.append(("ignite_drasi_deployment_capacity_tpm", {"deployment": d}, cap))
    s.append(("ignite_drasi_capacity_tpm", {"kind": "total"}, sum(capacity)))
    s.append(("ignite_drasi_capacity_tpm", {"kind": "healthy"}, sum(c for c, st in zip(capacity, status) if st == 0)))
    s.append(("ignite_drasi_capacity_tpm", {"kind": "needed"}, SUPPORT + BATCH))
    if recovery is not None: s.append(("ignite_drasi_recovery_state", {}, recovery))
    s.append(("ignite_drasi_action_needed", {}, action))
    s.append(("ignite_drasi_safe_if_one_fails", {"lost": safe[1]} if safe[1] else {}, safe[0]))
    for w, d, v in running: s.append(("ignite_drasi_running_tpm", {"workload": w, "deployment": d}, v))
    for w, d, v in proposed: s.append(("ignite_drasi_proposed_tpm", {"workload": w, "deployment": d}, v))
    for (w, d), v in policy.items(): s.append(("ignite_drasi_policy_allowed", {"workload": w, "deployment": d}, v))
    if approval is not None: s.append(("ignite_drasi_last_approval_state", {"approval": "a-1"}, approval))
    return s


MOVED = (("support", "EU-B", SUPPORT), ("batch", "US-C", BATCH))
EDITED = {**POLICY, ("support", "EU-B"): 0}
STATES = {
    "healthy": state(),
    "failure": state(status=(2, 0, 0), overdue=4, action=1, safe=(2, None), proposed=MOVED),
    "applied": state(status=(2, 0, 0), overdue=2, recovery=0, safe=(2, None), running=MOVED, proposed=MOVED, approval=2),
    "verified": state(status=(2, 0, 0), recovery=1, safe=(2, None), running=MOVED, proposed=MOVED, approval=3),
    "no-plan": state(status=(2, 0, 1), overdue=3, action=2, safe=(2, None), policy=EDITED, approval=3,
                     running=(("support", "EU-B", -SUPPORT), ("batch", "US-C", BATCH))),
    "at-risk": state(capacity=(100000, 50000, 100000), action=3, safe=(0, "EU-A"),
                     running=(("support", "EU-A", SUPPORT), ("batch", "US-C", BATCH)), approval=3, recovery=1),
    "unknown": state(status=(0, -1, -1), recovery=-1, action=-1, safe=(-1, None), approval=-1,
                     policy={k: -1 for k in POLICY}),
    "not-ready": None,
    "unreachable": None,
}


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
    return "\n".join(lines) + "\n"


class Handler(BaseHTTPRequestHandler):
    current = "healthy"

    def do_GET(self):
        if self.path != "/metrics" or Handler.current == "unreachable":
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
        Handler.current = name
        self.send_response(204)
        self.end_headers()

    def log_message(self, *_):
        pass


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=settings.port("DRASI_METRICS"))
    parser.add_argument("state", nargs="?", default="healthy", choices=list(STATES))
    args = parser.parse_args()
    Handler.current = args.state
    print(f"fixture /metrics on 127.0.0.1:{args.port}, state {args.state}", flush=True)
    ThreadingHTTPServer(("127.0.0.1", args.port), Handler).serve_forever()
