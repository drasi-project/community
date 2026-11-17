import { createServer } from "node:http";
import { port } from "../settings.mjs";

// Explicit TEST FIXTURES. This process is used only by playwright.config.ts.
const epoch = "TEST-FIXTURE-EPOCH";
const streams = new Map();
let revision, model, selectedBasis, commands, mutations, unavailable, expires, traffic;
function reset() {
  revision = 1; selectedBasis = "TEST-FIXTURE-BASIS-1"; commands = []; mutations = []; unavailable = false; expires = Date.now() + 3600000;
  traffic = { running: false, submitted: 0, ends_ms: 0, error: null, interval_seconds: 5 };
  model = {
    endpoints: ["EU-A", "EU-B", "US-C"].map(id => ({ id, region: id === "US-C" ? "US" : "EU", model: "support-model", budget_tpm: 100000, mode: "ok", reporting: true })),
    workloads: [{ id: "support", destination: "EU-A", reservation_tpm: 60000, model: "support-model", region: "EU" }, { id: "batch", destination: "EU-B", reservation_tpm: 80000, model: "support-model", region: "any" }],
    policy: { revision: 1, destinations: { support: ["EU-A", "EU-B"], batch: ["EU-A", "EU-B", "US-C"] } },
  };
}
reset();
function evidence() {
  return Object.fromEntries(model.endpoints.map(e => [e.id, { id: e.id, run_epoch: epoch, status: Date.now() >= expires ? "unknown" : e.mode, observed_ms: Date.now() - 100, expires_ms: expires, sequence: 17 }]));
}
function decision(kind) {
  return {
    run_epoch: epoch, revision, policy_revision: model.policy.revision, basis: selectedBasis,
    state: kind === "policy" ? "fresh" : "feasible", reason: "TEST FIXTURE: complete assignment preserves every commitment under current Rego policy and observed availability",
    model, evidence: evidence(),
    eligibility: Object.fromEntries(model.workloads.map(w => [w.id, Object.fromEntries(model.endpoints.map(e => [e.id, {
      allowed: model.policy.destinations[w.id].includes(e.id), reason: model.policy.destinations[w.id].includes(e.id) ? "TEST FIXTURE permits classification, destination and model" : "TEST FIXTURE denies classification, destination or model",
    }]))])),
    assignments: { support: "EU-B", batch: "US-C" }, candidate_id: `TEST-FIXTURE-CANDIDATE-${revision}`,
    valid: kind === "allocation", failure: kind === "resilience" ? "EU-A" : null,
  };
}
let extraRows = {};
function queries() {
  const result = {
    "endpoint-freshness": evidence(), "overdue-tasks": {}, "attempt-context": {}, "breach-history": {}, "alert-context": {}, "decision-context": {},
    "task-context": { "TEST-TASK": { id: "TEST-TASK", run_epoch: epoch, workload: "support", trace_id: "TEST-TRACE", created_ms: 100, deadline_ms: 15100, completed_ms: null, cancelled_ms: null } },
  };
  for (const kind of ["policy", "allocation", "resilience"]) {
    const data = decision(kind);
    result[`${kind}-results`] = { active: { id: "active", ...data, payload: JSON.stringify(data) } };
  }
  return { ...result, ...extraRows };
}
function json(response, value, status = 200) { response.writeHead(status, { "Content-Type": "application/json" }); response.end(JSON.stringify(value)); }
async function body(request) { let text = ""; for await (const chunk of request) text += chunk; return text ? JSON.parse(text) : {}; }
const server = createServer(async (request, response) => {
  const url = new URL(request.url, "http://localhost");
  if (url.pathname === "/test/reset") { reset(); extraRows = {}; return json(response, { fixture: true }); }
  if (url.pathname === "/test/state") return json(response, { mutations, revision });
  if (url.pathname === "/test/offline") { unavailable = (await body(request)).value; return json(response, {}); }
  if (url.pathname === "/test/expire") { expires = Date.now() - 1; return json(response, {}); }
  if (url.pathname === "/test/recover") {
    commands[0].status = "recovery_confirmed"; commands[0].applied_ms = Date.now() - 5000; commands[0].applied_revision = revision;
    commands[0].recovered_ms = Date.now(); commands[0].support_endpoint = "EU-B";
    return json(response, {});
  }
  if (url.pathname === "/test/query") {
    const input = await body(request);
    extraRows[input.query] = input.rows;
    for (const [stream, name] of streams) if (name === input.query) {
      if (input.disconnect) stream.end();
      else stream.write(`event: change\ndata: ${JSON.stringify({ results: input.changes })}\n\n`);
    }
    return json(response, {});
  }
  if (unavailable) return json(response, { error: "EXPLICIT TEST FIXTURE offline" }, 503);
  if (url.pathname === "/health") return json(response, { run_epoch: epoch, revision, model, pending: 1, continuous_context: "ready", streams_attached: true, task_target_seconds: 15, report_expiry_seconds: 8, traffic });
  if (url.pathname === "/context") return json(response, { streams_attached: true, queries: queries() });
  if (url.pathname === "/decisions") return json(response, {
    run_epoch: epoch, revision, streams_attached: true, candidate_current: true, approval: "awaiting_approval",
    policy: decision("policy"), allocation: decision("allocation"), resilience: decision("resilience"),
    recovery: {}, commands, audit: [], publications: [], observation_window_ms: 5000, success_freshness_ms: 15000,
  });
  if (url.pathname.endsWith("/attach")) {
    const name = url.pathname.split("/").at(-2);
    response.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache", Connection: "keep-alive" });
    response.write(`event: snapshot\ndata: ${JSON.stringify(Object.values(queries()[name] ?? {}))}\n\n`);
    streams.set(response, name);
    const timer = setInterval(() => response.write(": keepalive\n\n"), 1000);
    response.on("close", () => { clearInterval(timer); streams.delete(response); });
    return;
  }
  if (url.pathname.startsWith("/tasks/")) return json(response, { trace_url: `http://localhost:${port("LANGFUSE")}/project/ignite-probe/traces/TEST-TRACE`, task: { id: "TEST-TASK" }, attempts: [] });
  if (request.method === "POST") {
    const input = await body(request); mutations.push({ path: url.pathname, input: structuredClone(input) });
    if (url.pathname === "/traffic") {
      traffic.running = input.running;
      return json(response, traffic);
    }
    if (url.pathname === "/configuration") {
      if (input.model.policy.revision !== model.policy.revision) return json(response, { error: "policy revision must be retained" }, 409);
      model = input.model; revision++; model.policy.revision++; selectedBasis = `TEST-FIXTURE-BASIS-${revision}`;
    }
    if (url.pathname === "/approvals") {
      if (input.basis !== selectedBasis) return json(response, { error: "the plan you selected is out of date" }, 409);
      const duplicate = commands.find(command => command.id === input.command_id);
      if (duplicate) return json(response, { duplicate: true, command: duplicate });
      commands.unshift({
        id: input.command_id, candidate_id: input.candidate_id, basis: input.basis, run_epoch: epoch,
        input_revision: revision, policy_revision: model.policy.revision, decision: decision("allocation"),
        status: "committed", committed_ms: Date.now(), applied_ms: null, applied_revision: null,
        support_endpoint: null, recovered_ms: null, error: null,
      });
      return json(response, { duplicate: false, command: commands[0] });
    }
    return json(response, { test_fixture: true, run_epoch: epoch, revision, trace_url: `http://localhost:${port("LANGFUSE")}/project/ignite-probe/traces/TEST-TRACE` });
  }
  json(response, { error: "No TEST FIXTURE route" }, 404);
});
server.listen(port("UI_FIXTURE"), "127.0.0.1");
