"""Generate the projected Grafana dashboard, `ai-operations` (dashboards/workload.json).

One dashboard, two rows (plan/contracts.md section 16): **AI Deployments**, ordinary
monitoring measured from real requests, and **Decision Support**, Drasi's answers.
Every Drasi tile reads Drasi's Prometheus reaction (ignite/prometheus-exporter), one
metric per tile (contracts sections 9 and 16), except Recommended action, which shows
whichever of two Drasi answers applies (tile-action and tile-change). A tile shows
Unknown when Drasi answered -1, or when its reaction input is not ready
(drasi_reaction_input_ready{output=...} is not 1, or the reaction isn't scraped). A
missing count means 0 (or the tile's "nothing yet" value) only while its input is ready.

Every panel has a description (Grafana's (i) icon) and a top-level "key": the projector
highlights panels as `ai-operations.<key>`, placed from this file's gridPos
(presentation/panels.ts). Keep keys stable; a beat that names a missing key fails a test.

Run `python3 observability/grafana/dashboards.py` from demo/, then
`./ops/observability-stack.sh restart grafana`. Top-level settings (uid, tags) are kept
from the existing JSON file.

The stage shows the dashboard in kiosk mode at 1.35x, ending at the caption line, so it
is laid out for an 18-row grid (1422x742 before scaling). Colours are the Ignite
template's and mean the same everywhere: red = broken, yellow = needs action or waiting,
green = OK, grey = nothing to do or not applicable.
"""
import json, sys, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../ops"))
import settings  # noqa: E402  ports.env: the Langfuse and Mattermost links carry their ports
D = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "dashboards")
DS = {"uid": "ignite-prometheus", "type": "prometheus"}
# The Ignite template's palette (presentation/slides.css).
GREEN, YELLOW, RED, BLUE, PURPLE, GREY = "#8DE971", "#FFE399", "#FFB3BB", "#8DC8E8", "#C5B4E3", "#9AA4B0"
UNKNOWN = {"type": "special", "options": {"match": "null", "result": {"text": "Unknown", "color": GREY}}}
NO_TRAFFIC = {"type": "special", "options": {"match": "nan", "result": {"text": "No traffic", "color": GREY}}}
CHANNEL = f"http://localhost:{settings.port('MATTERMOST')}/ai-platform/channels/incident-ai-support"


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
    opts = {"-1": {"text": "Unknown", "color": GREY}}
    for k, (text, color) in mapping.items():
        opts[str(k)] = {"text": text, "color": color}
    return [{"type": "value", "options": opts}, UNKNOWN]


def row(id, key, title, y):
    return {"id": id, "key": key, "type": "row", "title": title, "collapsed": False,
            "gridPos": {"x": 0, "y": y, "w": 24, "h": 1}, "panels": []}


def target(expr, legend=None, ref="A"):
    t = {"refId": ref, "expr": expr, "instant": True, "range": False, "interval": "1s"}
    if legend is not None: t["legendFormat"] = legend
    return t


def stat(id, key, title, exprs, x, y, w, h, desc, mappings=None, thresholds=None, unit=None, name=False, label=False,
         decimals=None, value_size=40, extra_mappings=()):
    """exprs: an expression, or a list of (expression, legend) pairs.
    name: each series' name above its value. label: the series name *is* the answer (Drasi's text label),
    coloured by the value's mapping."""
    if isinstance(exprs, str): exprs = [(exprs, None)]
    defaults = {"noValue": "Unknown", "mappings": list(extra_mappings) + (mappings or states({})) + [NO_TRAFFIC]}
    if mappings is not None and thresholds is None: defaults["color"] = {"mode": "fixed", "fixedColor": "text"}
    else: defaults["thresholds"] = {"mode": "absolute", "steps": thresholds or [{"color": "text", "value": None}]}
    if unit: defaults["unit"] = unit
    if decimals is not None: defaults["decimals"] = decimals
    text_mode = "name" if label else "value_and_name" if name else "value"
    return {"id": id, "key": key, "type": "stat", "title": title, "description": desc,
            "gridPos": {"x": x, "y": y, "w": w, "h": h},
            "datasource": DS, "targets": [target(e, l, chr(65 + i)) for i, (e, l) in enumerate(exprs)],
            "fieldConfig": {"defaults": defaults, "overrides": []},
            "options": {"colorMode": "value", "graphMode": "none", "justifyMode": "center",
                        "orientation": "vertical" if name or label else "auto",
                        "reduceOptions": {"values": False, "calcs": ["lastNotNull"], "fields": ""},
                        "textMode": text_mode, "wideLayout": not name, "showPercentChange": False,
                        "text": {"titleSize": 15, "valueSize": value_size}}}


def rows_of(panel):
    """One row per series, name on the left and value on the right."""
    panel["options"].update({"orientation": "horizontal", "wideLayout": True, "justifyMode": "auto"})
    panel["options"]["text"]["titleSize"] = panel["options"]["text"]["valueSize"]
    return panel


def series(id, key, title, targets, x, y, w, h, desc, unit, overrides):
    return {"id": id, "key": key, "type": "timeseries", "title": title, "description": desc,
            "gridPos": {"x": x, "y": y, "w": w, "h": h},
            "datasource": DS, "targets": [{"refId": chr(65 + i), "expr": e, "legendFormat": l} for i, (e, l) in enumerate(targets)],
            "fieldConfig": {"defaults": {"unit": unit, "min": 0, "decimals": 2,
                                         "custom": {"lineWidth": 3, "fillOpacity": 8, "showPoints": "never", "spanNulls": True}},
                            "overrides": overrides},
            "options": {"legend": {"displayMode": "list", "placement": "bottom", "calcs": []}, "tooltip": {"mode": "multi"}}}


def color(name, c):
    return {"matcher": {"id": "byName", "options": name}, "properties": [{"id": "color", "value": {"mode": "fixed", "fixedColor": c}}]}


calls = 'ai_gateway_model_calls_total'
OVERDUE = drasi("drasi_requests_overdue", "overdue", 'workload="support"', missing=0)
DEPLOYMENT_STATUS = drasi("drasi_deployment_health_status", "deployments")
DEPLOYMENT = states({0: ("Healthy", GREEN), 1: ("Over quota", YELLOW), 2: ("Failed", RED)})
WORKLOAD = states({0: ("Healthy", GREEN), 1: ("Over quota", YELLOW), 2: ("Failing", RED), 3: ("Not allowed", RED)})

# Recommended action: one tile for Drasi's recommendation and the change it leads to. Two Drasi
# answers feed it: tile-action (what to do now, its text label naming the change) and tile-change
# (the latest approval's progress, its text label the approved change). The first that applies:
# Unknown; applying or verifying an approved change; approve a proposed change; no change fits;
# add capacity; the last change verified; none. tile-change has no row until this run has an
# approval, so before that the tile follows tile-action alone.
ACTION = drasi("drasi_recommended_action", "action")
CHANGE = drasi("drasi_change_status", "change")


def shown(expr, code, prefix=""):
    """`expr`'s series as the tile's answer `code`, its text label prefixed."""
    e = f'(({expr}) * 0 + {code})'
    return f'label_replace({e}, "text", "{prefix}$1", "text", "(.*)")' if prefix else e


RECOMMENDED = " or on() ".join([
    'label_replace(max(' + ACTION + ') == -1, "text", "Unknown", "", "")',
    shown(f'{CHANGE} == 2', 4, "Applying: "),
    shown(f'{CHANGE} == 3', 5, "Verifying: "),
    shown(f'{ACTION} == 1', 1, "Approve: "),
    shown(f'{ACTION} == 2', 2),
    shown(f'{ACTION} == 3', 3),
    shown(f'({CHANGE} == 4) and on() (max({ACTION}) == 0)', 6, "Verified: "),
    shown(f'{ACTION} == 0', 0),
])
RECOMMENDED_COLOURS = states({0: ("None", GREEN), 1: ("Change proposed", YELLOW), 2: ("No change fits", RED),
                              3: ("Add capacity", YELLOW), 4: ("Applying", YELLOW), 5: ("Verifying", YELLOW),
                              6: ("Verified", GREEN)})

# Failover readiness: tile-resilience's text label, except "Already degraded", which names the
# unhealthy deployments from Deployment status (the query knows only how many there are).
READINESS = drasi("drasi_failover_readiness", "resilience")
DOWN = f'({DEPLOYMENT_STATUS} >= 1)'
DEGRADED = (f'((label_replace(({DOWN} * 0 + 2) and on() (count({DOWN}) == 1), "text", "Already degraded: $1 down", "deployment", "(.+)")'
            f' or on() label_replace((count({DOWN}) > 1) * 0 + 2, "text", "Already degraded: more than one deployment down", "", ""))'
            f' and on() ({READINESS} == 2))')
# Text-mode tiles show nothing at all without a series, so Unknown is a series of its own.
def known(e):
    return f'({e}) or on() label_replace(vector(-1), "text", "Unknown", "", "")'


READINESS_COLOURS = states({1: ("Survives losing one", GREEN), 0: ("At risk", YELLOW), 2: ("Already degraded", GREY)})

TOP, MIDDLE, BOTTOM = 1, 5, 12

ops = json.load(open(f"{D}/workload.json"))
ops.update({
    "title": "AI service · Operations",
    "description": "AI Deployments: ordinary monitoring, measured from real requests. Decision Support: Drasi's answers, "
                   "computed from changes. Deployments and capacity are simulated.",
    "time": {"from": "now-5m", "to": "now"}, "refresh": "5s", "version": ops.get("version", 1) + 1,
})
ops["annotations"]["list"] = [{
    "name": "Show Drasi decisions on charts",
    "datasource": {"type": "grafana", "uid": "-- Grafana --"},
    "enable": True, "hide": False, "iconColor": GREEN,
    # Current Grafana reads tag filters from "target"; top-level "tags" is ignored.
    "target": {"type": "tags", "tags": ["drasi-decision"], "matchAny": False, "limit": 100},
}]

recommended = stat(11, "recommended-action", "Recommended action", [(known(f'({RECOMMENDED}) and on() {ready("change")}'), "{{text}}")],
                   12, BOTTOM, 12, 3,
                   "Drasi's recommended change for the current facts, and where it is. Approve: a change that fits policy and "
                   "capacity with the fewest moves, waiting for approval on its card in #incident-ai-support (the link on this "
                   "panel opens the channel). Applying, then Verifying: approved and applied; verified once the backlog has "
                   "cleared and support is answered on the new route. No change fits: policy and capacity allow no plan. "
                   "Add capacity: healthy, but losing one deployment would leave a workload nowhere to run.",
                   mappings=RECOMMENDED_COLOURS, label=True, value_size=22)
open_channel = {"title": "Open in #incident-ai-support", "url": CHANNEL, "targetBlank": True}
recommended["links"] = [open_channel]
recommended["fieldConfig"]["defaults"]["links"] = [open_channel]

ops["panels"] = [
    row(100, "ai-deployments", "AI Deployments", 0),
    stat(1, "model-calls", "Model calls", f'sum(rate({calls}[30s]))', 0, TOP, 6, 4,
         "Calls to the AI model per second, both workloads, over the last 30 seconds.", unit="reqps", decimals=1),
    stat(2, "support-errors", "Support errors",
         f'100 * (sum(rate({calls}{{workload="support",status!="200"}}[30s])) or vector(0)) / sum(rate({calls}{{workload="support"}}[30s]))',
         6, TOP, 6, 4, "Share of customer-support model calls that failed in the last 30 seconds.", unit="percent", decimals=0,
         thresholds=[{"color": GREEN, "value": None}, {"color": YELLOW, "value": 5}, {"color": RED, "value": 20}]),
    stat(3, "answer-time", "Answer time p95",
         'histogram_quantile(0.95, sum by(le)(rate(ai_gateway_answer_time_seconds_bucket{workload="support"}[1m])))', 12, TOP, 6, 4,
         "95th percentile time from a customer's question to its answer, over the last minute. The target is 15 seconds.", unit="s", decimals=1,
         extra_mappings=[{"type": "special", "options": {"match": "null+nan", "result": {"text": "No answers", "color": RED}}}],
         thresholds=[{"color": GREEN, "value": None}, {"color": YELLOW, "value": 10}, {"color": RED, "value": 15}]),
    stat(4, "support-queue", "Support queue", 'ai_gateway_queue_depth{workload="support"}', 18, TOP, 6, 4,
         "Customer-support requests waiting for an answer. One or two in flight is normal.", decimals=0,
         thresholds=[{"color": GREEN, "value": None}, {"color": YELLOW, "value": 3}, {"color": RED, "value": 6}]),
    {"id": 10, "key": "alerts-firing", "type": "alertlist", "title": "Alerts firing",
     "description": "Grafana's alert rules that are firing now: Model calls failing, and Support requests overdue "
                    "(an alert on Drasi's answer). Each also posts to #incident-ai-support.",
     "gridPos": {"x": 18, "y": MIDDLE, "w": 6, "h": 6},
     "options": {"viewMode": "list", "groupMode": "default", "groupBy": [], "maxItems": 5, "sortOrder": 1,
                 "dashboardAlerts": False, "alertName": "", "alertInstanceLabelFilter": '{component="workload"}',
                 "folder": None, "showInstances": False,
                 "stateFilter": {"firing": True, "pending": False, "noData": False, "normal": False, "error": True,
                                 "recovering": False}}},
    series(5, "model-calls-chart", "Model calls by workload", [
        (f'sum(rate({calls}{{workload="support",status="200"}}[30s])) or vector(0)', "support answered"),
        (f'sum(rate({calls}{{workload="batch",status="200"}}[30s])) or vector(0)', "summaries answered"),
        (f'sum(rate({calls}{{workload="support",status!="200"}}[30s])) or vector(0)', "support failed"),
        (f'sum(rate({calls}{{workload="batch",status!="200"}}[30s])) or vector(0)', "summaries failed"),
    ], 0, MIDDLE, 10, 6, "Model calls per second, by workload and whether they succeeded. Failed calls are retried.", "reqps",
           [color("support answered", BLUE), color("summaries answered", PURPLE), color("support failed", RED), color("summaries failed", YELLOW)]),
    series(6, "support-waiting", "Support requests waiting", [
        ('ai_gateway_queue_depth{workload="support"}', "waiting"),
        (OVERDUE, "overdue (Drasi)"),
    ], 10, MIDDLE, 8, 6, "Support requests waiting for an answer, and how many are past their 15-second deadline according to Drasi.", "short",
           [color("waiting", YELLOW), color("overdue (Drasi)", RED)]),
    row(101, "decision-support", "Decision Support", BOTTOM - 1),
    stat(7, "support-overdue", "Support overdue", OVERDUE, 0, BOTTOM, 4, 6,
         "Customer-support requests past their 15-second deadline, from a Drasi continuous query. "
         "A request becomes overdue as time passes, with no new event.",
         mappings=states({}), decimals=0, thresholds=[{"color": GREEN, "value": None}, {"color": RED, "value": 1}], value_size=44),
    rows_of(stat(8, "deployment-status", "Deployment status", [(DEPLOYMENT_STATUS, "{{deployment}}")], 4, BOTTOM, 4, 6,
         "Each deployment's health as Drasi derives it from its health-probe telemetry. Unknown means no probe for 8 seconds: "
         "Drasi notices silence as time passes.",
         mappings=DEPLOYMENT, name=True, value_size=24)),
    rows_of(stat(9, "workload-status", "Workload status", [(drasi("drasi_workload_status", "workloads", f'workload="{w}"'), "{{name}} · {{deployment}}") for w in ("support", "batch")], 8, BOTTOM, 4, 6,
         "Each workload where it runs now, from a Drasi continuous query over its route, the placement policy and its "
         "deployment's health: Healthy, Over quota or Failing as its deployment is; Not allowed when policy no longer "
         "allows it there; Unknown when its deployment has been silent for 8 seconds.",
         mappings=WORKLOAD, name=True, value_size=20)),
    recommended,
    stat(15, "failover-readiness", "Failover readiness",
         [(known(f'({READINESS} != 2) or ({READINESS} == 2 unless on() {DEGRADED})') + f' unless on() {DEGRADED}', "{{text}}"),
          (DEGRADED, "{{text}}")],
         12, BOTTOM + 3, 12, 3,
         "Drasi's what-if: if any one deployment failed, would every workload still have somewhere allowed to run?",
         mappings=READINESS_COLOURS, label=True, value_size=22),
]

# Grafana 13 shows a panel's menu (⋮) only while the pointer is on that panel, so the menus
# blink in and out as the pointer crosses the dashboard; there is no dashboard setting for it.
# This invisible HTML panel keeps every menu visible. Grafana renders its <style> only with
# GF_PANELS_DISABLE_SANITIZE_HTML (compose.yaml); otherwise it is stripped and menus stay hover-only.
# It is listed first, outside the rows, so the projector's row highlights (presentation/panels.ts)
# ignore it, and sits below the last row.
MENU_STYLE = ('<style>button[data-testid^="data-testid Panel menu"].show-on-hover'
              '{opacity:1!important;visibility:visible!important;background:none}'
              '[data-viz-panel-key="panel-200"]{pointer-events:none}</style>')
ops["panels"].insert(0, {"id": 200, "key": "panel-menus", "type": "text", "title": "", "transparent": True,
                         "description": "Keeps every panel's menu visible (no content).",
                         "gridPos": {"x": 0, "y": 18, "w": 24, "h": 1}, "options": {"mode": "html", "content": MENU_STYLE}})

# Request counts: whole numbers, and a readable scale while nothing is waiting.
waiting = next(p for p in ops["panels"] if p["id"] == 6)
waiting["fieldConfig"]["defaults"].update({"decimals": 0, "softMax": 10})

# Links for use outside the stage (kiosk mode hides them).
LANGFUSE = f"http://localhost:{settings.port('LANGFUSE')}/project/ignite-probe/traces?filter=isRootObservation%3Bboolean%3B%3B%3D%3Btrue"
link = lambda title, url: {"title": title, "url": url, "type": "link", "icon": "external link", "targetBlank": True,
                           "asDropdown": False, "includeVars": False, "keepTime": False, "tags": [], "tooltip": ""}
ops["links"] = [link("#incident-ai-support", CHANNEL), link("Langfuse traces", LANGFUSE)]
with open(f"{D}/workload.json", "w") as f:
    json.dump(ops, f, indent=2, ensure_ascii=False); f.write("\n")
print("ok")
