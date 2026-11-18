"""What the integration tests read: Drasi's outputs, never the AI service's opinion of them.

- `DeskStream`: the `desk-stream` SSE reaction (snapshot, then live frames), as the desk reads it.
- `metrics()` / `metric_until()`: the Prometheus reaction's `/metrics` (`ignite/prometheus-exporter`).
- `prometheus()`: the same series after Prometheus scraped them (what Grafana reads).
- `Attachment`: Server's query results (`/queries/{name}/attach`) for queries outside the desk stream.
- `approval()`, `routes()`, annotations, scores and `incident_cards()`: the effects of Drasi's HTTP
  reactions (the last are the `incident-channel` reaction's posts in Mattermost's #incident-ai-support).

The AI service is used only as what it is in the demo: the application (`/tasks`), the simulated
external systems (`/control`), the approval tool (`POST /approvals`) and the demo driver
(`/configuration`, `/reset`). See plan/contracts.md sections 4, 9 and 10.
"""
import base64
import http.client
import importlib.util
import json
import os
import queue
import re
import socket
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path
import sys

DEMO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(DEMO / "ops"))
import settings  # noqa: E402


def _environment():
    # demo/.env (generated, private): database and Grafana/Langfuse credentials.
    values = {}
    for line in (DEMO / ".env").read_text().splitlines():
        key, separator, value = line.partition("=")
        if separator and not key.startswith("#"):
            values[key.strip()] = value.strip().strip("'\"")
    for key, value in values.items():
        os.environ.setdefault(key, value)
    return values


ENV = _environment()
_spec = importlib.util.spec_from_file_location("ignite_runtime", DEMO / "ops/native-runtime.py")
runtime = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(runtime)
_spec = importlib.util.spec_from_file_location("ignite_native", DEMO / "tests/native-integration.py")
native = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(native)
Attachment = native.Attachment
EVIDENCE = DEMO / ".native/evidence"


def wait(predicate, timeout=30, what="bounded condition", interval=0.2):
    end = time.monotonic() + timeout
    while True:
        value = predicate()
        if value:
            return value
        if time.monotonic() >= end:
            raise AssertionError(f"{what} did not become true within {timeout}s")
        time.sleep(interval)


def sql(text):
    return runtime.sql(text, tuples=True).strip()


def sql_json(text):
    value = sql(text)
    return json.loads(value) if value else None


# The application, the simulated systems and the demo driver (AI service).

def api(path, body=None, expected=200):
    request = urllib.request.Request(settings.url("WORKLOAD") + path,
        data=None if body is None else json.dumps(body).encode(),
        headers={"Content-Type": "application/json"})
    try:
        response = urllib.request.urlopen(request, timeout=10)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        value = json.load(response)
        assert response.status == expected, (path, response.status, value)
        return value


def health():
    return api("/health")


def control(action, deployment="EU-A"):
    return api("/control", {"run_epoch": health()["run_epoch"], "deployment": deployment, "action": action})


def configure(**changes):
    state = health()
    return api("/configuration", {"run_epoch": state["run_epoch"], "revision": state["revision"], **changes})


def reset():
    """Demo driver reset: new run, seed scenario, traffic stopped. Waits for Drasi to re-derive."""
    state = health()
    result = api("/reset", {k: state[k] for k in ["run_epoch", "revision"]})
    with DeskStream(["plan-results", "tile-action"]) as stream:
        stream.until("plan-results", lambda rows: rows and rows[0]["run_epoch"] == result["run_epoch"]
                     and rows[0]["state"] == "feasible" and rows[0]["moves"] == 0, 45)
        stream.until("tile-action", lambda rows: rows and rows[0]["value"] == 0, 30)
    return result


def submit(workload="support"):
    epoch = health()["run_epoch"]
    body = {"id": f"{epoch}-test-{uuid.uuid4()}", "run_epoch": epoch, "workload": workload,
            "content": "SYNTHETIC RESTRICTED support text" if workload == "support" else "PUBLIC DOCUMENT text"}
    return body, api("/tasks", body)


def task(ident):
    return api("/tasks/" + ident)


def approve(plan, ident=None, **override):
    """The approval tool records what the presenter saw; it decides nothing."""
    body = {"id": ident or str(uuid.uuid4()), "candidate_id": plan["candidate_id"],
            "basis": plan["basis"], "revision": plan["revision"], **override}
    return api("/approvals", body)["id"]


# Effects of Drasi's HTTP reactions.

def approval(ident):
    return sql_json(f"SELECT row_to_json(a) FROM ignite_approvals a WHERE id='{ident}';")


def routes():
    return {w["id"]: w["deployment"] for w in health()["workloads"]}


def grafana_auth():
    return base64.b64encode(f"admin:{ENV['GRAFANA_ADMIN_PASSWORD']}".encode()).decode()


def get(url, auth=None):
    request = urllib.request.Request(url, headers={"Authorization": f"Basic {auth}"} if auth else {})
    with urllib.request.urlopen(request, timeout=10) as response:
        return json.load(response)


def annotations(tag):
    return get(settings.url("GRAFANA") + "/api/annotations?" +
               urllib.parse.urlencode([("tags", "ignite-decision"), ("tags", tag)]), grafana_auth())


def scores(trace_id, name):
    url = settings.url("LANGFUSE") + "/api/public/v3/scores?" + urllib.parse.urlencode(
        {"traceId": trace_id, "fields": "subject,details"})
    return [s for s in get(url, ENV["LANGFUSE_AUTH_BASE64"])["data"] if s["name"] == name]


# The Prometheus reaction.

_SAMPLE = re.compile(r'^([a-zA-Z_:][a-zA-Z0-9_:]*)(?:\{(.*)\})?\s+(\S+)$')
_LABEL = re.compile(r'([a-zA-Z_][a-zA-Z0-9_]*)="((?:[^"\\]|\\.)*)"')


def mattermost(path, token="MATTERMOST_ADMIN_TOKEN", body=None):
    request = urllib.request.Request(f"http://127.0.0.1:{settings.port('MATTERMOST')}/api/v4{path}",
        data=None if body is None else json.dumps(body).encode(),
        headers={"Authorization": "Bearer " + ENV[token], "Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=10) as response:
        return json.load(response)


def incident_cards():
    """The `incident-channel` reaction's posts in #incident-ai-support, oldest first: every post by
    the `drasi` bot, as {id, root_id, create_at, card, ignite_id, run_epoch, attachment}."""
    ids = json.loads((DEMO / ".native/mattermost.json").read_text())
    posts = mattermost(f"/channels/{ids['channel_id']}/posts?per_page=200")["posts"].values()
    return [{"id": p["id"], "root_id": p.get("root_id", ""), "create_at": p["create_at"],
             "card": p["props"].get("ignite_card"), "ignite_id": p["props"].get("ignite_id"),
             "run_epoch": p["props"].get("ignite_run_epoch"), "attachment": p["props"]["attachments"][0]}
            for p in sorted(posts, key=lambda p: p["create_at"]) if p["user_id"] == ids["bot_user_id"]]


def metrics():
    """Every sample the Prometheus reaction serves: [(name, {labels}, value)]."""
    with urllib.request.urlopen(settings.url("DRASI_METRICS") + "/metrics", timeout=5) as response:
        text = response.read().decode()
    samples = []
    for line in text.splitlines():
        if line and not line.startswith("#"):
            name, labels, value = _SAMPLE.match(line).groups()
            samples.append((name, dict(_LABEL.findall(labels or "")), float(value)))
    return samples


def series(name, **labels):
    """{frozen labels: value} of one metric whose labels include `labels`."""
    return {tuple(sorted(l.items())): v for n, l, v in metrics()
            if n == name and all(l.get(k) == str(want) for k, want in labels.items())}


def metric(name, **labels):
    """The value of one series, or None when the reaction serves none (Unknown)."""
    values = list(series(name, **labels).values())
    assert len(values) <= 1, (name, labels, values)
    return values[0] if values else None


def metric_until(name, predicate, timeout=30, **labels):
    return wait(lambda: predicate(metric(name, **labels)) and (metric(name, **labels),),
                timeout, f"{name}{labels} on the Prometheus reaction")[0]


def prometheus(query):
    """Result vector of a PromQL query against this copy's Prometheus (what Grafana reads)."""
    url = settings.url("PROMETHEUS") + "/api/v1/query?" + urllib.parse.urlencode({"query": query})
    return get(url)["data"]["result"]


# The desk-stream SSE reaction.

class DeskStream:
    """One `/events` stream with each alias's snapshot applied, as the desk does (contracts §10)."""

    def __init__(self, aliases):
        self.aliases = list(aliases)
        self.connection = http.client.HTTPConnection("127.0.0.1", settings.port("DRASI_SSE"), timeout=40)
        self.connection.request("GET", "/events", headers={"Accept": "text/event-stream"})
        self.response = self.connection.getresponse()
        assert self.response.status == 200, self.response.status
        self.frames = queue.Queue()
        self.history = []
        self.heartbeats = 0
        self.closed = False
        self.thread = threading.Thread(target=self._read, daemon=True)
        self.thread.start()
        self.rows, self.positions = {}, {}
        for alias in self.aliases:
            with urllib.request.urlopen(settings.url("DRASI_SSE") + "/snapshot/" + alias, timeout=10) as response:
                self.positions[alias] = int(response.headers["x-drasi-output-position"])
                self.rows[alias] = {}
                self._apply(alias, json.load(response))

    def _read(self):
        data = []
        try:
            while line := self.response.readline():
                line = line.decode().rstrip("\r\n")
                if line.startswith("data:"):
                    data.append(line[5:].strip())
                elif not line and data:
                    self.frames.put(json.loads("\n".join(data)))
                    data = []
            self.frames.put({"type": "closed"})
        except (OSError, ValueError) as error:
            self.frames.put({"type": "error", "error": str(error)})

    def _apply(self, alias, changes):
        rows = self.rows[alias]
        for change in changes:
            key = f"{change['identity']['namespace']}/{change['identity']['id']}"
            operation = change["operation"]
            if operation in ("ADD", "REPLACE"):
                rows[key] = dict(change["after"])
            elif operation == "PATCH":
                row = rows.setdefault(key, {})
                row.update(change.get("after") or {})
                for name in change.get("removedProperties") or []:
                    row.pop(name, None)
            elif operation == "DELETE":
                rows.pop(key, None)
            else:
                raise AssertionError(change)

    def _step(self, timeout):
        try:
            frame = self.frames.get(timeout=max(timeout, 0.01))
        except queue.Empty:
            return
        if frame.get("type") == "heartbeat":
            self.heartbeats += 1
            return
        assert frame.get("type") not in ("closed", "error"), frame
        alias = frame["output"]["alias"]
        if alias in self.rows and frame["sequence"] > self.positions[alias]:
            self._apply(alias, frame["changes"])
            self.positions[alias] = frame["sequence"]
            self.history.append({"alias": alias, "sequence": frame["sequence"], "changes": frame["changes"]})

    def get(self, alias):
        while not self.frames.empty():
            self._step(0)
        return list(self.rows[alias].values())

    def until(self, alias, predicate, timeout=30):
        end = time.monotonic() + timeout
        while True:
            rows = self.get(alias)
            if predicate(rows):
                return rows
            remaining = end - time.monotonic()
            if remaining <= 0:
                raise AssertionError(f"{alias} did not reach the expected rows within {timeout}s: {rows}")
            self._step(min(remaining, 1))

    def one(self, alias):
        rows = self.get(alias)
        return rows[0] if len(rows) == 1 else None

    def holds(self, alias, predicate, seconds):
        """The predicate stays true for `seconds` (used for 'Drasi waits')."""
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            assert predicate(self.get(alias)), (alias, self.get(alias))
            self._step(min(0.2, end - time.monotonic()))

    def close(self):
        if not self.closed:
            self.closed = True
            try:
                self.connection.sock.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
            self.thread.join(timeout=3)
            self.connection.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


def save(name, evidence):
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    path = EVIDENCE / name
    path.write_text(json.dumps(evidence, indent=2, default=str) + "\n")
    return path
