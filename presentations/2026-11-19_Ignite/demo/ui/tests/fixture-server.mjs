import { createServer } from "node:http";
import { port } from "../settings.mjs";

// Explicit TEST FIXTURES. This process is used only by playwright.config.ts.
// It stands in for two things: the AI service's API (/health, demo controls, the approval tool,
// /tasks) and Drasi Server's query attach API (/api/v1/instances/ignite/queries/{name}/attach),
// with the result rows of contracts.md §5 and §9. Its "Drasi" decides approvals the way the
// approval-valid and approval-stale queries do; nothing in the desk decides them.
const epoch = "TEST-FIXTURE-EPOCH";
const streams = new Map();
let revision, deployments, workloads, destinations, approvals, mutations, unavailable, expired, traffic, extraRows;
function reset() {
  revision = 1; approvals = []; mutations = []; unavailable = false; expired = false; extraRows = {};
  traffic = { running: false, submitted: 0, ends_ms: 0, error: null, interval_seconds: 5 };
  deployments = ["EU-A", "EU-B", "US-C"].map(id => ({ id, region: id === "US-C" ? "US" : "EU", model: "support-model", capacity_tpm: 100000, mode: "ok", reporting: true }));
  workloads = [
    { id: "support", data_class: "restricted", region_requirement: "EU", model: "support-model", demand_tpm: 60000, deployment: "EU-A" },
    { id: "batch", data_class: "public", region_requirement: "any", model: "support-model", demand_tpm: 80000, deployment: "EU-B" },
  ];
  destinations = { support: ["EU-A", "EU-B"], batch: ["EU-A", "EU-B", "US-C"] };
}
reset();

// TEST FIXTURE answers: one fixed plan while summaries may use US-C, none otherwise.
function plan() {
  const basis = `TEST-FIXTURE-BASIS-${revision}`;
  if (expired) return { id: "plan", state: "unknown", candidate_id: "", basis, revision, run_epoch: epoch, assignments: "{}", moves: 0, peak_utilisation: 0, reason: "TEST FIXTURE: a health report expired", solved_ms: Date.now() };
  if (!destinations.batch.includes("US-C")) return { id: "plan", state: "infeasible", candidate_id: "", basis, revision, run_epoch: epoch, assignments: "{}", moves: 0, peak_utilisation: 0, reason: "TEST FIXTURE: support and summaries can't both fit", solved_ms: Date.now() };
  const assignments = { batch: "US-C", support: "EU-B" };
  const moves = workloads.filter(w => assignments[w.id] !== w.deployment).length;
  return { id: "plan", state: "feasible", candidate_id: `TEST-FIXTURE-CANDIDATE-${revision}`, basis, revision, run_epoch: epoch,
    assignments: JSON.stringify(assignments), moves, peak_utilisation: 0.8, reason: "TEST FIXTURE: every workload fits under policy", solved_ms: Date.now() };
}
const statusCode = { ok: 0, throttled: 1, failed: 2 };
const approvalCode = { pending: 1, applied: 2, verified: 3, rejected: 4 };
function queries() {
  const p = plan();
  const down = deployments.some(d => d.mode !== "ok" || !d.reporting);
  const latest = approvals.find(a => a.latest);
  const action = p.state === "unknown" ? -1 : p.state === "infeasible" ? 2 : p.moves > 0 ? 1 : 0;
  const result = {
    "overdue-tasks": {},
    "plan-results": { plan: p },
    "tile-action": [{ value: action }],
    "tile-resilience": [{ value: expired ? -1 : down ? 2 : 0, lost: down ? "" : "EU-A" }],
    "tile-recovery": latest && ["applied", "verified"].includes(latest.status) ? [{ value: latest.status === "verified" ? 1 : 0 }] : [],
    "tile-running": workloads.map(w => ({ workload: w.id, deployment: w.deployment, value: destinations[w.id].includes(w.deployment) ? w.demand_tpm : -w.demand_tpm })),
    "tile-last-approval": latest ? [{ approval: latest.id, value: approvalCode[latest.status], status: latest.status, reason: latest.reason }] : [],
    "tile-deployments": deployments.map(d => ({ deployment: d.id, capacity_tpm: d.capacity_tpm, status: expired || !d.reporting ? -1 : statusCode[d.mode] })),
    "policy-results": Object.fromEntries(workloads.flatMap(w => deployments.map(d => {
      const allowed = destinations[w.id].includes(d.id);
      return [`${w.id}/${d.id}`, { id: `${w.id}/${d.id}`, subject: w.id, target: d.id, state: allowed ? "allowed" : "denied", allowed, reason: `TEST FIXTURE ${allowed ? "allows" : "denies"} it`, run_epoch: epoch }];
    }))),
    "resilience-results": { resilience: { id: "resilience", state: expired ? "unknown" : "at_risk", first_failure: "EU-A", baseline: p.state, unhealthy_bins: down ? 1 : 0, run_epoch: epoch } },
    "grafana-alerts": {},
  };
  return { ...result, ...extraRows };
}
const rowsOf = name => Object.values(queries()[name] ?? {});

// Every half second, send each attached stream the difference from what it last sent.
setInterval(() => {
  for (const [stream, state] of streams) {
    const rows = rowsOf(state.name);
    const text = JSON.stringify(rows);
    if (text === state.sent) continue;
    const results = [...JSON.parse(state.sent).map(data => ({ type: "DELETE", data })), ...rows.map(data => ({ type: "ADD", data }))];
    state.sent = text;
    stream.write(`event: change\ndata: ${JSON.stringify({ results })}\n\n`);
  }
}, 500);

// The TEST FIXTURE's stand-in for approval-valid / approval-stale and the gateway reaction.
function decide(approval) {
  const p = plan();
  if (approval.status !== "pending") return;
  if (p.state === "feasible" && p.candidate_id === approval.candidate_id && p.basis === approval.basis && p.revision === approval.revision) {
    const assignments = JSON.parse(p.assignments);
    for (const w of workloads) w.deployment = assignments[w.id];
    revision++; approval.status = "applied"; approval.applied_revision = revision;
  } else {
    approval.status = "rejected"; approval.reason = "The plan changed since it was approved";
  }
}

function json(response, value, status = 200) { response.writeHead(status, { "Content-Type": "application/json" }); response.end(JSON.stringify(value)); }
async function body(request) { let text = ""; for await (const chunk of request) text += chunk; return text ? JSON.parse(text) : {}; }
const health = () => ({ run_epoch: epoch, revision, pending: 1, task_target_seconds: 15, report_expiry_seconds: 8, traffic, deployments, workloads, destinations });
const server = createServer(async (request, response) => {
  const url = new URL(request.url, "http://localhost");
  if (url.pathname === "/test/reset") { reset(); return json(response, { fixture: true }); }
  if (url.pathname === "/test/state") return json(response, { mutations, revision, approvals });
  if (url.pathname === "/test/offline") { unavailable = (await body(request)).value; return json(response, {}); }
  if (url.pathname === "/test/expire") { expired = true; return json(response, {}); }
  if (url.pathname === "/test/recover") { const latest = approvals.find(a => a.latest); if (latest) latest.status = "verified"; return json(response, {}); }
  if (url.pathname === "/test/query") {
    const input = await body(request);
    extraRows[input.query] = input.rows;
    if (input.disconnect) for (const [stream, state] of streams) if (state.name === input.query) stream.end();
    return json(response, {});
  }
  if (url.pathname.startsWith("/api/v1/instances/ignite/queries/") && url.pathname.endsWith("/results"))
    return json(response, { success: true, data: rowsOf(url.pathname.split("/").at(-2)), error: null });
  if (url.pathname.endsWith("/attach")) {
    const name = url.pathname.split("/").at(-2);
    const rows = rowsOf(name);
    response.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache", Connection: "keep-alive" });
    response.write(`event: snapshot\ndata: ${JSON.stringify(rows)}\n\n`);
    streams.set(response, { name, sent: JSON.stringify(rows) });
    const timer = setInterval(() => response.write(": heartbeat\n\n"), 1000);
    response.on("close", () => { clearInterval(timer); streams.delete(response); });
    return;
  }
  if (unavailable) return json(response, { error: "EXPLICIT TEST FIXTURE offline" }, 503);
  if (url.pathname === "/health") return json(response, health());
  if (url.pathname.startsWith("/tasks/")) return json(response, { trace_url: `http://localhost:${port("LANGFUSE")}/project/ignite-probe/traces/TEST-TRACE`, task: { id: "TEST-TASK" }, attempts: [] });
  if (request.method === "POST") {
    const input = await body(request); mutations.push({ path: url.pathname, input: structuredClone(input) });
    if (url.pathname === "/traffic") { traffic.running = input.running; return json(response, traffic); }
    if (url.pathname === "/control") {
      const d = deployments.find(d => d.id === input.deployment);
      if (!d) return json(response, { error: "unknown deployment" }, 400);
      if (input.action === "fail") d.mode = "failed";
      if (input.action === "throttle") d.mode = "throttled";
      if (input.action === "restore") { d.mode = "ok"; d.reporting = true; }
      if (input.action === "pause-reports") d.reporting = false;
      if (input.action === "resume-reports") d.reporting = true;
      return json(response, { run_epoch: epoch, revision });
    }
    if (url.pathname === "/configuration") {
      if (input.run_epoch !== epoch || input.revision !== revision) return json(response, { error: "the scenario changed; try again" }, 409);
      for (const [id, capacity] of Object.entries(input.capacity ?? {})) deployments.find(d => d.id === id).capacity_tpm = capacity;
      Object.assign(destinations, input.destinations ?? {});
      revision++;
      return json(response, { run_epoch: epoch, revision });
    }
    if (url.pathname === "/approvals") {
      if (!approvals.some(a => a.id === input.id)) {
        for (const a of approvals) a.latest = false;
        const approval = { id: input.id, candidate_id: input.candidate_id, basis: input.basis, revision: input.revision, status: "pending", reason: "", latest: true };
        approvals.unshift(approval);
        setTimeout(() => decide(approval), 300);
      }
      return json(response, { id: input.id });
    }
    if (url.pathname === "/reset") { reset(); mutations.push({ path: url.pathname, input: structuredClone(input) }); return json(response, { run_epoch: epoch, revision }); }
    return json(response, { test_fixture: true, run_epoch: epoch, revision, trace_url: `http://localhost:${port("LANGFUSE")}/project/ignite-probe/traces/TEST-TRACE` });
  }
  json(response, { error: "No TEST FIXTURE route" }, 404);
});
server.listen(port("UI_FIXTURE"), "127.0.0.1");
