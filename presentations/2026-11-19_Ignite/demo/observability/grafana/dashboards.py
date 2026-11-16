"""Generate the two audience Grafana dashboards (workload.json, decisions.json).

Edit the panel definitions below, run `python3 observability/grafana/dashboards.py`
from demo/, then reload Grafana provisioning. Top-level settings (uid, links,
tags) and the Decision events panel are kept from the existing JSON files.
"""
import json, sys, copy, os
D = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "dashboards")
DS = {"uid": "ignite-prometheus", "type": "prometheus"}
VALID = ' and on(view) (ignite_drasi_valid_until_seconds > time())'
FRESH = ' and on() (time() - max(ignite_drasi_projection_timestamp_seconds) < 5)'
SUMMARIES = lambda e: f'label_replace({e}, "workload", "summaries", "workload", "batch")'
UNKNOWN = {"type": "special", "options": {"match": "null", "result": {"text": "Unknown", "color": "gray"}}}

def drasi(view, extra=""):
    sel = f'view="{view}"' + extra
    return f'ignite_drasi_dashboard{{{sel}}}' + VALID

def states(mapping):
    opts = {"-1": {"text": "Unknown", "color": "gray"}}
    for k, (text, color) in mapping.items():
        opts[str(k)] = {"text": text, "color": color}
    return [{"type": "value", "options": opts}, UNKNOWN]

def row(id, title, y):
    return {"id": id, "type": "row", "title": title, "collapsed": False, "gridPos": {"x": 0, "y": y, "w": 24, "h": 1}, "panels": []}

def stat(id, title, expr, x, y, w, h, desc, mappings=None, thresholds=None, unit=None, legend=None, name=False, decimals=None, value_size=34):
    defaults = {"noValue": "Unknown"}
    if mappings is not None:
        defaults["mappings"] = mappings
        defaults["color"] = {"mode": "fixed", "fixedColor": "gray"}
    else:
        defaults["mappings"] = [UNKNOWN]
        defaults["thresholds"] = {"mode": "absolute", "steps": thresholds or [{"color": "text", "value": None}]}
    if unit: defaults["unit"] = unit
    if decimals is not None: defaults["decimals"] = decimals
    target = {"refId": "A", "expr": expr, "instant": True, "range": False, "interval": "1s"}
    if legend: target["legendFormat"] = legend
    return {"id": id, "type": "stat", "title": title, "description": desc, "gridPos": {"x": x, "y": y, "w": w, "h": h},
            "datasource": DS, "targets": [target], "fieldConfig": {"defaults": defaults, "overrides": []},
            "options": {"colorMode": "value", "graphMode": "none", "orientation": "auto", "justifyMode": "center",
                        "reduceOptions": {"values": False, "calcs": ["lastNotNull"], "fields": ""},
                        "textMode": "value_and_name" if name else "value", "wideLayout": True,
                        "text": {"titleSize": 16, "valueSize": value_size}}}

def multi_stat(id, title, exprs, x, y, w, h, desc, unit, value_size=24):
    panel = stat(id, title, exprs[0][1], x, y, w, h, desc, unit=unit, legend=exprs[0][0], name=True, value_size=value_size)
    panel["targets"] = [{"refId": chr(65 + i), "expr": e, "legendFormat": label, "instant": True, "range": False, "interval": "1s"}
                        for i, (label, e) in enumerate(exprs)]
    return panel

def series(id, title, expr, legend, x, y, w, h, desc, unit, overrides):
    return {"id": id, "type": "timeseries", "title": title, "description": desc, "gridPos": {"x": x, "y": y, "w": w, "h": h},
            "datasource": DS, "targets": [{"refId": "A", "expr": expr, "legendFormat": legend}],
            "fieldConfig": {"defaults": {"unit": unit, "min": 0, "custom": {"lineWidth": 2, "fillOpacity": 10, "showPoints": "never"}},
                            "overrides": overrides},
            "options": {"legend": {"displayMode": "list", "placement": "right", "calcs": []}, "tooltip": {"mode": "multi"}}}

def color(regex, c):
    return {"matcher": {"id": "byRegexp", "options": regex}, "properties": [{"id": "color", "value": {"mode": "fixed", "fixedColor": c}}]}

ACTION = states({0: ("None", "green"), 1: ("Approve proposed plan", "orange"), 2: ("No valid plan", "red")})
RECOVERY = states({0: ("Awaiting evidence", "orange"), 1: ("Verified", "green"), 2: ("None in progress", "text")})
TOKENS = "suffix:k tokens/min"

# ---------- Operations ----------
ops = json.load(open(f"{D}/workload.json"))
calls = 'ignite_workload_requests_total'
ops.update({
    "title": "AI service · Operations",
    "description": "Top: standard service health measured from real requests. Bottom: customer impact maintained by Drasi continuous queries. Deployments and capacity are simulated.",
    "time": {"from": "now-5m", "to": "now"}, "version": ops.get("version", 1) + 1,
})
ANNOTATIONS_ON = os.environ.get("IGNITE_ANNOTATIONS_ON") == "1"
ops["annotations"]["list"] = [{
    "name": "Show Drasi decisions on charts",
    "datasource": {"type": "grafana", "uid": "-- Grafana --"},
    "enable": ANNOTATIONS_ON, "hide": False, "iconColor": "#52d6a4",
    # Current Grafana reads tag filters from "target"; top-level "tags" is ignored.
    "target": {"type": "tags", "tags": ["ignite-decision"], "matchAny": False, "limit": 100},
}]
ops["panels"] = [
    row(100, "Service health · measured from requests", 0),
    stat(1, "Model calls", f'sum(rate({calls}[1m]))', 0, 1, 4, 3,
         "Model calls per second across both workloads, over the last minute.", unit="reqps", decimals=2),
    stat(2, "Support errors", f'100 * (sum(rate({calls}{{workload="support",status!="200"}}[1m])) or vector(0)) / sum(rate({calls}{{workload="support"}}[1m]))', 4, 1, 4, 3,
         "Share of customer-support model calls that failed in the last minute.", unit="percent", decimals=1,
         thresholds=[{"color": "green", "value": None}, {"color": "orange", "value": 5}, {"color": "red", "value": 20}]),
    stat(3, "Support p95", '1000 * histogram_quantile(0.95, sum by(le)(rate(ignite_workload_call_duration_seconds_bucket{workload="support"}[1m])))', 8, 1, 4, 3,
         "95th percentile duration of customer-support model calls in the last minute.", unit="ms", decimals=0,
         thresholds=[{"color": "green", "value": None}, {"color": "orange", "value": 1000}, {"color": "red", "value": 3000}]),
    stat(4, "Support queue", 'ignite_workload_queue_depth{workload="support"}' + FRESH, 12, 1, 4, 3,
         "Customer-support requests queued and not yet answered.", decimals=0,
         thresholds=[{"color": "green", "value": None}, {"color": "orange", "value": 1}, {"color": "red", "value": 5}]),
    series(5, "Model calls by outcome", SUMMARIES(f'sum by(workload,endpoint,status)(rate({calls}[1m]))'), "{{workload}} → {{endpoint}} · HTTP {{status}}",
           0, 4, 12, 6, "HTTP 200 succeeded. 503: deployment failed. 429: deployment over quota. 409: refused because the route changed before sending.", "reqps",
           [color(".*HTTP 200", "green"), color(".*HTTP 503", "red"), color(".*HTTP 429", "orange"), color(".*HTTP 409", "purple")]),
    series(6, "Requests answered", SUMMARIES('sum by(workload)(rate(ignite_workload_completed_total[1m]))'), "{{workload}}",
           12, 4, 12, 6, "Customer-support and document-summary requests completed per second.", "reqps",
           [color("support", "blue"), color("summaries", "yellow")]),
    row(101, "Customer impact · maintained by Drasi", 10),
    stat(7, "Support overdue", drasi("overdue", ',workload="support"'), 0, 11, 5, 3,
         "Customer-support requests unanswered past their 15-second target. A time-aware continuous query: requests become late as time passes, with no new event. The target is short for the demo.",
         decimals=0, thresholds=[{"color": "green", "value": None}, {"color": "red", "value": 1}]),
    stat(8, "Deployment status", drasi("endpoint"), 5, 11, 9, 3,
         "Latest status reported by each model deployment. Unknown means no report in the last 8 seconds: Drasi expires stale reports as time passes.",
         mappings=states({0: ("Healthy", "green"), 1: ("Throttled", "orange"), 2: ("Failed", "red")}), legend="{{endpoint}}", name=True, value_size=24),
    stat(9, "Recovery", drasi("recovery"), 14, 11, 5, 3,
         "Whether an approved recovery is working: verified only after real support requests succeed on the new route.", mappings=RECOVERY, value_size=22),
    {"id": 10, "type": "alertlist", "title": "Service alerts", "gridPos": {"x": 16, "y": 1, "w": 8, "h": 3},
     "options": {"viewMode": "list", "maxItems": 3, "sortOrder": 1, "alertName": "Ignite workload HTTP failure", "dashboardAlerts": False,
                 "stateFilter": {"firing": True, "pending": True, "normal": True, "noData": True, "error": True}}},
    stat(11, "Action needed", drasi("approval"), 19, 11, 5, 3,
         "Drasi's current recovery proposal. Open Policy and recovery to review and approve it.", mappings=ACTION, value_size=22),
]

# ---------- Policy and recovery ----------
dec = json.load(open(f"{D}/decisions.json"))
dec.update({
    "title": "AI service · Policy and recovery",
    "description": "Drasi's continuous policy and models: where each workload may run, the proposed routes, and whether recovery is verified. Capacity units are simulated tokens per minute.",
    "version": dec.get("version", 1) + 1,
})
policy_map = states({1: ("Allowed", "green"), 0: ("Denied", "red")})
panels = [
    row(100, "Recovery decision · Drasi", 0),
    stat(1, "Action needed", drasi("approval"), 0, 1, 8, 3,
         "Approve proposed plan: the current routes no longer work and Drasi has a complete plan that fits policy and capacity.", mappings=ACTION, value_size=30),
    stat(2, "Recovery", drasi("recovery"), 8, 1, 8, 3,
         "Verified only after real support requests succeed on the new route.", mappings=RECOVERY, value_size=30),
    stat(3, "Safe if EU-A fails?", drasi("resilience"), 16, 1, 8, 3,
         "Would every workload still fit somewhere it is allowed if EU-A were lost? A what-if assessment, not an outage.",
         mappings=states({1: ("Yes", "green"), 0: ("No", "red")}), value_size=30),
    stat(4, "Current routes", SUMMARIES(f'(ignite_drasi_dashboard{{view="configured"}} > 0){VALID}') + " / 1000", 0, 4, 12, 3,
         "Where each workload runs now, with its reserved capacity.", unit=TOKENS, legend="{{workload}} → {{endpoint}}", name=True, value_size=22),
    stat(5, "Proposed routes", SUMMARIES(f'(ignite_drasi_dashboard{{view="proposed"}} > 0){VALID}') + " / 1000", 12, 4, 12, 3,
         "Drasi's best complete allocation for the current facts. Identical to current routes when no change is needed.",
         unit=TOKENS, legend="{{workload}} → {{endpoint}}", name=True, value_size=22),
    row(101, "Where each workload may run · Drasi continuous policy", 7),
]
for pid, workload, label, x in [(6, "support", "Customer support may run on", 0), (7, "batch", "Document summaries may run on", 12)]:
    panels.append(stat(pid, label, drasi("policy", f',workload="{workload}"'), x, 8, 12, 3,
                       "Allowed or denied by the placement policy (Rego), maintained by Drasi as policy and deployments change.",
                       mappings=policy_map, legend="{{endpoint}}", name=True, value_size=24))
old_events = next(p for p in dec["panels"] if p["type"] == "annolist")
events = copy.deepcopy(old_events)
events.update({"id": 12, "title": "Decision events", "gridPos": {"x": 0, "y": 11, "w": 12, "h": 5},
               "description": "Approvals, rejections, adopted routes and verified recovery, newest first."})
panels.append(events)
HEALTHY = f'(ignite_drasi_dashboard{{view="endpoint"}}{VALID}) == 0'
panels.append(multi_stat(14, "Capacity · tokens/min", [
    ("Total", f'sum(ignite_workload_capacity_tpm{FRESH}) / 1000'),
    ("Healthy", f'sum(ignite_workload_capacity_tpm and on(endpoint) ({HEALTHY})) / 1000'),
    ("Needed", f'sum(ignite_drasi_dashboard{{view="configured"}}{VALID}) / 1000'),
], 12, 11, 6, 5, "Total capacity across all deployments, the part on healthy deployments, and what the workloads need. Enough in total doesn't mean there's a valid place for each workload.", "suffix:k"))
panels.append(stat(13, "Deployment budgets", '(ignite_workload_capacity_tpm' + FRESH + ') / 1000', 18, 11, 6, 5,
                   "Simulated capacity of each deployment, in tokens per minute.", unit="suffix:k", legend="{{endpoint}}", name=True, value_size=24))
dec["panels"] = panels

for d in (ops, dec):
    for link in d.get("links", []):
        link["url"] = link["url"].replace("from=now-1m", "from=now-5m")
for name, d in [("workload.json", ops), ("decisions.json", dec)]:
    with open(f"{D}/{name}", "w") as f:
        json.dump(d, f, indent=2, ensure_ascii=False); f.write("\n")
print("ok")
