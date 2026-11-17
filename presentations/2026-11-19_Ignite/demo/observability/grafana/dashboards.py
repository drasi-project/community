"""Generate the two projected Grafana dashboards (workload.json, decisions.json).

Every Drasi tile reads Drasi's Prometheus reaction (ignite/prometheus-exporter),
one metric per tile, as listed in plan/contracts.md section 9. A tile shows
Unknown when Drasi answered -1, or when its reaction input is not ready
(drasi_reaction_input_ready{output=...} is not 1, or the reaction isn't scraped).
A missing count means 0 (or the tile's "nothing yet" value) only while its input
is ready.

Edit the panel definitions below, run `python3 observability/grafana/dashboards.py`
from demo/, then reload Grafana provisioning. Top-level settings (uid, tags) and the
Decision events panel's options are kept from the existing JSON files.

The stage shows dashboards in kiosk mode at 1.5x, so they are laid out for an
18-row grid (1280x720 before scaling). Colours mean the same everywhere:
red = broken, orange = needs action or waiting, green = OK, grey = not allowed or
nothing to show.
"""
import json, sys, copy, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../ops"))
import settings  # noqa: E402  ports.env: the Langfuse links carry its port
D = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "dashboards")
DS = {"uid": "ignite-prometheus", "type": "prometheus"}
SUMMARIES = lambda e: f'label_replace({e}, "workload", "summaries", "workload", "batch")'
GREY = "#9aa4b0"
UNKNOWN = {"type": "special", "options": {"match": "null", "result": {"text": "Unknown", "color": "gray"}}}
NO_TRAFFIC = {"type": "special", "options": {"match": "nan", "result": {"text": "No traffic", "color": "gray"}}}

def ready(output):
    return f'(drasi_reaction_input_ready{{output="{output}"}} == 1)'

def drasi(metric, output, labels="", missing=None, one=False):
    """A tile metric while its reaction input is ready; `missing` is the value when ready with no row."""
    e = f'{metric}{{{labels}}}' if labels else metric
    if one or missing is not None: e = f'max({e})'
    e = f'({e} and on() {ready(output)})'
    # max() keeps one unlabelled series, so a chart doesn't split it when the row comes and goes.
    return e if missing is None else f'max({e} or on() ({ready(output)} * 0 + {missing}))'

def states(mapping):
    opts = {"-1": {"text": "Unknown", "color": "gray"}}
    for k, (text, color) in mapping.items():
        opts[str(k)] = {"text": text, "color": color}
    return [{"type": "value", "options": opts}, UNKNOWN]

def row(id, title, y):
    return {"id": id, "type": "row", "title": title, "collapsed": False, "gridPos": {"x": 0, "y": y, "w": 24, "h": 1}, "panels": []}

def target(expr, legend=None, ref="A"):
    t = {"refId": ref, "expr": expr, "instant": True, "range": False, "interval": "1s"}
    if legend is not None: t["legendFormat"] = legend
    return t

def stat(id, title, exprs, x, y, w, h, desc, mappings=None, thresholds=None, unit=None, name=False, decimals=None,
         value_size=40, color=None, extra_mappings=()):
    """exprs: an expression, or a list of (expression, legend) pairs."""
    if isinstance(exprs, str): exprs = [(exprs, None)]
    defaults = {"noValue": "Unknown", "mappings": list(extra_mappings) + (mappings or states({})) + [NO_TRAFFIC]}
    if color: defaults["color"] = {"mode": "fixed", "fixedColor": color}
    elif mappings is not None and thresholds is None: defaults["color"] = {"mode": "fixed", "fixedColor": "text"}
    else: defaults["thresholds"] = {"mode": "absolute", "steps": thresholds or [{"color": "text", "value": None}]}
    if unit: defaults["unit"] = unit
    if decimals is not None: defaults["decimals"] = decimals
    return {"id": id, "type": "stat", "title": title, "gridPos": {"x": x, "y": y, "w": w, "h": h},
            "datasource": DS, "targets": [target(e, l, chr(65 + i)) for i, (e, l) in enumerate(exprs)],
            "fieldConfig": {"defaults": defaults, "overrides": []},
            "options": {"colorMode": "value", "graphMode": "none", "orientation": "auto", "justifyMode": "center",
                        "reduceOptions": {"values": False, "calcs": ["lastNotNull"], "fields": ""},
                        "textMode": "value_and_name" if name else "value", "wideLayout": not name, "showPercentChange": False,
                        **({"orientation": "vertical"} if name else {}),
                        "text": {"titleSize": 15, "valueSize": value_size}}}

def series(id, title, targets, x, y, w, h, desc, unit, overrides):
    return {"id": id, "type": "timeseries", "title": title, "gridPos": {"x": x, "y": y, "w": w, "h": h},
            "datasource": DS, "targets": [{"refId": chr(65 + i), "expr": e, "legendFormat": l} for i, (e, l) in enumerate(targets)],
            "fieldConfig": {"defaults": {"unit": unit, "min": 0, "decimals": 2,
                                         "custom": {"lineWidth": 3, "fillOpacity": 8, "showPoints": "never", "spanNulls": True}},
                            "overrides": overrides},
            "options": {"legend": {"displayMode": "list", "placement": "bottom", "calcs": []}, "tooltip": {"mode": "multi"}}}

def color(name, c):
    return {"matcher": {"id": "byName", "options": name}, "properties": [{"id": "color", "value": {"mode": "fixed", "fixedColor": c}}]}

# Drasi's tile queries already apply "already one down" (resilience 2) and "at risk only while all are healthy" (action 3).
RESILIENCE = drasi("ignite_drasi_safe_if_one_fails", "resilience")
SAFE_TARGETS = [(f'{RESILIENCE} != 0', " "), (f'{RESILIENCE} == 0', "not if {{lost}} fails")]
SAFE = states({1: ("Yes", "green"), 0: ("No", "orange"), 2: ("Already one down", GREY)})
ACTION_NOW = drasi("ignite_drasi_action_needed", "action", one=True)
RECOVERY_NOW = drasi("ignite_drasi_recovery_state", "recovery", missing=2)
OVERDUE = drasi("ignite_drasi_overdue_requests", "overdue", 'workload="support"', missing=0)
ACTION = states({0: ("None", "green"), 1: ("Approve proposed plan", "orange"), 2: ("No valid plan", "red"), 3: ("At risk", "orange")})
RECOVERY = states({0: ("Awaiting evidence", "orange"), 1: ("Verified", "green"), 2: ("Nothing to verify", GREY)})
DEPLOYMENT = states({0: ("Healthy", "green"), 1: ("Over quota", "orange"), 2: ("Failed", "red")})
TOKENS = "suffix:k tokens/min"
calls = 'ignite_workload_requests_total'

# ---------- Operations ----------
ops = json.load(open(f"{D}/workload.json"))
ops.update({
    "title": "AI service · Operations",
    "description": "Top: ordinary monitoring, measured from real requests. Bottom: Drasi's answers, computed from changes. Deployments and capacity are simulated.",
    "time": {"from": "now-5m", "to": "now"}, "refresh": "5s", "version": ops.get("version", 1) + 1,
})
ops["annotations"]["list"] = [{
    "name": "Show Drasi decisions on charts",
    "datasource": {"type": "grafana", "uid": "-- Grafana --"},
    "enable": True, "hide": False, "iconColor": "#52d6a4",
    # Current Grafana reads tag filters from "target"; top-level "tags" is ignored.
    "target": {"type": "tags", "tags": ["ignite-decision"], "matchAny": False, "limit": 100},
}]
OVERDUE_MAP = states({})
ops["panels"] = [
    row(100, "Monitoring · measured from requests", 0),
    stat(1, "Model calls", f'sum(rate({calls}[30s]))', 0, 1, 4, 4,
         "Calls to the AI model per second, both workloads, over the last 30 seconds.", unit="reqps", decimals=1),
    stat(2, "Support errors", f'100 * (sum(rate({calls}{{workload="support",status!="200"}}[30s])) or vector(0)) / sum(rate({calls}{{workload="support"}}[30s]))', 4, 1, 5, 4,
         "Share of customer-support model calls that failed in the last 30 seconds.", unit="percent", decimals=0,
         thresholds=[{"color": "green", "value": None}, {"color": "orange", "value": 5}, {"color": "red", "value": 20}]),
    stat(3, "Answer time p95", 'histogram_quantile(0.95, sum by(le)(rate(ignite_workload_answer_seconds_bucket{workload="support"}[1m])))', 9, 1, 5, 4,
         "95th percentile time from a customer's question to its answer, over the last minute. The target is 15 seconds.", unit="s", decimals=1,
         extra_mappings=[{"type": "special", "options": {"match": "null+nan", "result": {"text": "No answers", "color": "red"}}}],
         thresholds=[{"color": "green", "value": None}, {"color": "orange", "value": 10}, {"color": "red", "value": 15}]),
    stat(4, "Support queue", 'ignite_workload_queue_depth{workload="support"}', 14, 1, 4, 4,
         "Customer-support requests waiting for an answer. One or two in flight is normal.", decimals=0,
         thresholds=[{"color": "green", "value": None}, {"color": "orange", "value": 3}, {"color": "red", "value": 6}]),
    # The same conditions as the two Grafana alert rules (provisioning/alerting/workload.yaml), so the count can be coloured.
    stat(10, "Grafana alerts firing",
         f'((sum(increase({calls}{{status!="200"}}[30s])) > bool 0) or vector(0)) + (({OVERDUE} > bool 0) or vector(0))',
         18, 1, 6, 4, "Grafana alert rules firing: model calls failing, and support requests overdue (an alert on Drasi's answer).",
         decimals=0, thresholds=[{"color": "green", "value": None}, {"color": "red", "value": 1}]),
    series(5, "Model calls", [
        (f'sum(rate({calls}{{workload="support",status="200"}}[30s])) or vector(0)', "support answered"),
        (f'sum(rate({calls}{{workload="batch",status="200"}}[30s])) or vector(0)', "summaries answered"),
        (f'sum(rate({calls}{{workload="support",status!="200"}}[30s])) or vector(0)', "support failed"),
        (f'sum(rate({calls}{{workload="batch",status!="200"}}[30s])) or vector(0)', "summaries failed"),
    ], 0, 5, 14, 6, "Model calls per second, by workload and whether they succeeded. Failed calls are retried.", "reqps",
           [color("support answered", "blue"), color("summaries answered", "yellow"), color("support failed", "red"), color("summaries failed", "orange")]),
    series(6, "Support requests waiting", [
        ('ignite_workload_queue_depth{workload="support"}', "waiting"),
        (OVERDUE, "overdue (Drasi)"),
    ], 14, 5, 10, 6, "Support requests waiting for an answer, and how many of them are past their 15-second deadline according to Drasi.", "short",
           [color("waiting", "orange"), color("overdue (Drasi)", "red")]),
    row(101, "Drasi's answers · computed from changes", 11),
    stat(7, "Support overdue", OVERDUE, 0, 12, 4, 5,
         "Customer-support requests past their 15-second target, from a Drasi continuous query. A request becomes overdue as time passes, with no new event.",
         mappings=OVERDUE_MAP, decimals=0, thresholds=[{"color": "green", "value": None}, {"color": "red", "value": 1}]),
    stat(8, "Deployment status", [(drasi("ignite_drasi_deployment_status", "deployments"), "{{deployment}}")], 4, 12, 7, 5,
         "Each deployment's latest health report. Unknown means no report for 8 seconds: Drasi notices silence as time passes.",
         mappings=DEPLOYMENT, name=True, value_size=22),
    stat(9, "Recovery", RECOVERY_NOW, 11, 12, 4, 5,
         "After an approved change: Verified once the backlog is answered and support requests succeed on the new route.", mappings=RECOVERY, value_size=30),
    stat(11, "Action needed", ACTION_NOW, 15, 12, 4, 5,
         "Approve proposed plan: Drasi has a plan that fits policy and capacity. At risk: losing one deployment would leave no valid plan.", mappings=ACTION, value_size=30),
    stat(15, "Safe if one fails?", SAFE_TARGETS,
         19, 12, 5, 5, "Drasi's what-if: if any single deployment failed, would every workload still fit somewhere allowed?",
         mappings=SAFE, name=True, value_size=30),
]

# Request counts: whole numbers, and a readable scale while nothing is waiting.
waiting = next(p for p in ops["panels"] if p["id"] == 6)
waiting["fieldConfig"]["defaults"].update({"decimals": 0, "softMax": 10})

# ---------- Recovery plan ----------
dec = json.load(open(f"{D}/decisions.json"))
dec.update({
    "title": "AI service · Recovery plan",
    "description": "All Drasi: where each workload may run, the plan, approvals and verified recovery. Capacity is simulated tokens per minute.",
    "refresh": "5s", "version": dec.get("version", 1) + 1,
})
LAST = states({0: ("None yet", GREY), 1: ("Approved, applying", "orange"), 2: ("Applied, awaiting evidence", "orange"),
               3: ("Applied and verified", "green"), 4: ("Rejected", "orange")})
HELD = {"type": "range", "options": {"from": -100000, "to": -0.001, "result": {"text": "not allowed", "color": "orange"}}}
panels = [
    stat(1, "Action needed", ACTION_NOW, 0, 0, 6, 4,
         "Approve proposed plan: Drasi has a plan that fits policy and capacity. No valid plan: nothing fits. At risk: losing one deployment would leave no valid plan.",
         mappings=ACTION, value_size=34),
    stat(16, "Last approval", drasi("ignite_drasi_last_approval_state", "last-approval", missing=0), 6, 0, 6, 4,
         "The latest approval: applying, awaiting evidence, verified, or rejected because the facts changed since the plan was selected.",
         mappings=LAST, value_size=30),
    stat(2, "Recovery", RECOVERY_NOW, 12, 0, 6, 4,
         "Verified once the backlog is answered and support requests succeed on the new route.", mappings=RECOVERY, value_size=34),
    stat(3, "Safe if one fails?", SAFE_TARGETS,
         18, 0, 6, 4, "Drasi's what-if: if any single deployment failed, would every workload still fit somewhere allowed?",
         mappings=SAFE, name=True, value_size=34),
    stat(4, "Running now", [(SUMMARIES(f'{drasi("ignite_drasi_running_tpm", "running")} != 0') + " / 1000", "{{workload}} → {{deployment}}")], 0, 4, 12, 4,
         "Where each workload runs now, with the capacity it needs. Not allowed: Drasi's policy model no longer allows the deployment it runs on; Drasi's plan moves it once approved.",
         unit=TOKENS, name=True, value_size=30, extra_mappings=[HELD]),
    stat(5, "Proposed plan", [
        (SUMMARIES(f'({drasi("ignite_drasi_proposed_tpm", "proposed")} > 0) and on() ({ACTION_NOW} == 1)') + " / 1000", "{{workload}} → {{deployment}}"),
        (f'({ACTION_NOW} == 0 or {ACTION_NOW} == 3) * 0 - 2', " "),
        (f'({ACTION_NOW} == 2) * 0 - 3', " ")], 12, 4, 12, 4,
         "Drasi's plan for the current facts. Orange when it differs from what's running and needs approval.",
         unit=TOKENS, name=True, value_size=30, color="orange",
         extra_mappings=[{"type": "value", "options": {"-2": {"text": "Same as running now", "color": GREY}, "-3": {"text": "No valid plan", "color": "red"}}}]),
]
NOT_ALLOWED = states({1: ("Allowed", "green"), 0: ("Not allowed", "#b8c0ca")})
for pid, workload, label, x in [(6, "support", "Customer support may run on", 0), (7, "batch", "Document summaries may run on", 12)]:
    panels.append(stat(pid, label, [(drasi("ignite_drasi_policy_allowed", "policy", f'workload="{workload}"'), "{{deployment}}")], x, 8, 12, 4,
                       "From the placement policy (Rego), re-evaluated by Drasi whenever policy or deployments change.",
                       mappings=NOT_ALLOWED, name=True, value_size=30))
old_events = next(p for p in dec["panels"] if p["type"] == "annolist")
events = copy.deepcopy(old_events)
events.update({"id": 12, "title": "Decision events, newest first", "gridPos": {"x": 0, "y": 12, "w": 12, "h": 6},
               "description": "Drasi's plan as it changes, approvals, rejections, routes applied and verified recovery."})
# All of this run's events (reset clears them), not just the last five minutes.
events["options"].update({"limit": 4, "showTime": False, "onlyInTimeRange": False})
panels.append(events)
capacity = lambda kind, output: f'sum({drasi("ignite_drasi_capacity_tpm", output, f'kind="{kind}"')}) / 1000'
cap = stat(14, "Capacity · tokens/min", [
    (capacity("total", "capacity"), "Total"),
    (capacity("healthy", "capacity"), "Healthy"),
    (capacity("needed", "demand"), "Needed")], 12, 12, 6, 6,
    "Total capacity, the part on healthy deployments, and what the two workloads need. Enough in total doesn't mean each workload has an allowed place to run.",
    unit="suffix:k", name=True, value_size=30)
cap["options"]["orientation"] = "vertical"
panels.append(cap)
# One query per deployment keeps the order fixed when one of them fails.
def deployment(e):
    status = drasi("ignite_drasi_deployment_status", "deployments", f'deployment="{e}"')
    capacity = drasi("ignite_drasi_deployment_capacity_tpm", "deployments", f'deployment="{e}"')
    # Capacity is configuration, so it shows unless the deployment failed; unknown health shows on Deployment status.
    return f'({capacity} / 1000 unless on(deployment) ({status} == 2)) or on(deployment) (({status} == 2) * 0 - 1)', e
panels.append(stat(13, "Deployments", [deployment(e) for e in ("EU-A", "EU-B", "US-C")],
                   18, 12, 6, 6, "Each deployment's capacity; Failed while it is down. Orange when below its normal 100k.", unit="suffix:k", name=True, value_size=30,
                   thresholds=[{"color": "orange", "value": None}, {"color": "text", "value": 100}],
                   extra_mappings=[{"type": "value", "options": {"-1": {"text": "Failed", "color": "red"}}}]))
dec["panels"] = panels

# Links for use outside the stage (the stage hides them).
LANGFUSE = f"http://localhost:{settings.port('LANGFUSE')}/project/ignite-probe/traces?filter=isRootObservation%3Bboolean%3B%3B%3D%3Btrue"
link = lambda title, url: {"title": title, "url": url, "type": "link", "icon": "external link", "targetBlank": False,
                           "asDropdown": False, "includeVars": False, "keepTime": False, "tags": [], "tooltip": ""}
ops["links"] = [link("Recovery plan", "/d/ignite-decisions?from=now-5m&to=now&refresh=5s"), link("Langfuse traces", LANGFUSE)]
dec["links"] = [link("Operations", "/d/ignite-workload?from=now-5m&to=now&refresh=5s"), link("Langfuse traces", LANGFUSE)]
for name, d in [("workload.json", ops), ("decisions.json", dec)]:
    with open(f"{D}/{name}", "w") as f:
        json.dump(d, f, indent=2, ensure_ascii=False); f.write("\n")
print("ok")
