import { createServer } from "node:http";
import fs from "node:fs";
import { port } from "../settings.mjs";

// Explicit TEST FIXTURES. This process is used only by playwright.config.ts.
// It stands in for four things: the AI service's API (/health, demo controls, the approval tool,
// /tasks, the Mattermost card actions of contracts.md §14), Drasi's SSE reaction desk-stream (/drasi-stream/events and /snapshot/{alias}, the
// desk's and projector's answers), and Drasi Server's query attach API (the private diagnostics),
// with the result rows of contracts.md §5 and §9. Its "Drasi" decides approvals the way the
// approval-valid and approval-stale queries do; nothing in the desk decides them. And the incident
// channel: the cards Drasi's incident-channel reaction posts (contracts.md §15), shown by a plain
// stand-in channel page (/test/mattermost/channel) that the tests route Mattermost's URL to.
const epoch = "TEST-FIXTURE-EPOCH";
const streams = new Map();
let revision, deployments, workloads, destinations, approvals, mutations, unavailable, expired, traffic, extraRows, posts, posted, lastProposal;
function reset() {
  revision = 1; approvals = []; mutations = []; unavailable = false; expired = false; extraRows = {};
  posts = []; posted = new Set(); lastProposal = null;
  traffic = { running: false, submitted: 0, ends_ms: 0, error: null, interval_seconds: 5 };
  deployments = ["EU-A", "EU-B", "US-C"].map(id => ({ id, region: id === "US-C" ? "US" : "EU", model: "support-model", capacity_tpm: 100000, mode: "ok", reporting: true }));
  workloads = [
    { id: "support", data_class: "restricted", region_requirement: "EU", model: "support-model", demand_tpm: 60000, deployment: "EU-A" },
    { id: "batch", data_class: "public", region_requirement: "any", model: "support-model", demand_tpm: 80000, deployment: "EU-B" },
  ];
  destinations = { support: ["EU-A", "EU-B"], batch: ["EU-A", "EU-B", "US-C"] };
}
reset();

// TEST FIXTURE answers: while EU-A is healthy, what runs now; once it fails, one fixed plan while
// summaries may use US-C, none otherwise.
function plan() {
  const basis = `TEST-FIXTURE-BASIS-${revision}`;
  if (expired) return { id: "plan", state: "unknown", candidate_id: "", basis, revision, run_epoch: epoch, assignments: "{}", moves: 0, peak_utilisation: 0, reason: "TEST FIXTURE: a health report expired", solved_ms: Date.now() };
  if (!destinations.batch.includes("US-C")) return { id: "plan", state: "infeasible", candidate_id: "", basis, revision, run_epoch: epoch, assignments: "{}", moves: 0, peak_utilisation: 0, reason: "TEST FIXTURE: support and summaries can't both fit", solved_ms: Date.now() };
  const euAUp = deployments.find(d => d.id === "EU-A").mode === "ok";
  const assignments = euAUp ? Object.fromEntries(workloads.map(w => [w.id, w.deployment])) : { batch: "US-C", support: "EU-B" };
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
  const fragile = !down && deployments.find(d => d.id === "EU-A").capacity_tpm < 100000;
  const action = p.state === "unknown" ? -1 : p.state === "infeasible" ? 2 : p.moves > 0 ? 1 : fragile ? 3 : 0;
  const actionText = ["None", "Move summaries to US-C and support to EU-B", "No change fits policy and capacity", "Add capacity: losing EU-B leaves support nowhere to run"][action] ?? "";
  const result = {
    "overdue-tasks": {},
    "plan-results": { plan: p },
    "tile-action": [{ value: action, text: actionText }],
    "tile-resilience": [{ value: expired ? -1 : down ? 2 : fragile ? 0 : 1, lost: fragile ? "EU-B" : "" }],
    "tile-recovery": latest && ["applied", "verified"].includes(latest.status) ? [{ value: latest.status === "verified" ? 1 : 0 }] : [],
    "tile-running": workloads.map(w => ({ workload: w.id, deployment: w.deployment, value: destinations[w.id].includes(w.deployment) ? w.demand_tpm : -w.demand_tpm })),
    "tile-last-approval": latest ? [{ approval: latest.id, value: approvalCode[latest.status], status: latest.status, reason: latest.reason }] : [],
    "tile-deployments": deployments.map(d => ({ deployment: d.id, capacity_tpm: d.capacity_tpm, status: expired || !d.reporting ? -1 : statusCode[d.mode] })),
    "policy-results": Object.fromEntries(workloads.flatMap(w => deployments.map(d => {
      const allowed = destinations[w.id].includes(d.id);
      return [`${w.id}/${d.id}`, { id: `${w.id}/${d.id}`, subject: w.id, target: d.id, state: allowed ? "allowed" : "denied", allowed, reason: `TEST FIXTURE ${allowed ? "allows" : "denies"} it`, run_epoch: epoch }];
    }))),
    "resilience-results": { resilience: { id: "resilience", state: expired ? "unknown" : "at_risk", first_failure: "EU-A", baseline: p.state, unhealthy_bins: down ? 1 : 0, run_epoch: epoch } },
  };
  return { ...result, ...extraRows };
}
const rowsOf = name => Object.values(queries()[name] ?? {});

// desk-stream: one event stream for every output, as drasi.network/sse-sink sends it. Each
// output keeps its rows by identity and its own sequence; a snapshot reports that position.
const deskAliases = ["overdue-tasks", "plan-results", "tile-action", "tile-recovery", "tile-resilience", "tile-running", "tile-last-approval"];
const outputs = Object.fromEntries(deskAliases.map(alias => [alias, { sequence: 0, rows: {} }]));
const eventStreams = new Set();
const identity = (alias, row) => ({ namespace: `drasi.output/TEST-FIXTURE-${alias}`, id: String(row.id ?? row.workload ?? row.deployment ?? "row") });
function current(alias) { return Object.fromEntries(rowsOf(alias).map(row => [identity(alias, row).id, row])); }
function syncOutputs() {
  for (const alias of deskAliases) {
    const output = outputs[alias], rows = current(alias), changes = [];
    for (const [id, row] of Object.entries(rows)) {
      const before = output.rows[id];
      if (!before) changes.push({ operation: "ADD", identity: identity(alias, row), element: { kind: "node", labels: ["Result"] }, after: row, effectiveFrom: Date.now() });
      else if (JSON.stringify(before) !== JSON.stringify(row)) changes.push({ operation: "REPLACE", identity: identity(alias, row), element: { kind: "node", labels: ["Result"] }, before, after: row, effectiveFrom: Date.now() });
    }
    for (const [id, row] of Object.entries(output.rows)) if (!(id in rows)) changes.push({ operation: "DELETE", identity: identity(alias, row), before: row, effectiveFrom: Date.now() });
    if (!changes.length) continue;
    output.rows = rows; output.sequence++;
    const frame = { output: { alias, component: alias, port: "out" }, sequence: output.sequence, publishedAt: Date.now(), changes };
    for (const stream of eventStreams) stream.write(`data: ${JSON.stringify(frame)}\n\n`);
  }
}
setInterval(syncOutputs, 200);

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

// TEST FIXTURE stand-in for Drasi's incident-channel reaction (contracts.md §15): the same cards,
// posted on the same triggers, without the debounce.
let postSeq = 0;
const colour = { red: "#d44a3a", orange: "#f2a33a", blue: "#3d8bd9", green: "#2fa66a", grey: "#9aa4b0" };
const field = (title, value, short = true) => ({ title, value, short });
function postCard(card, attachment, root_id = "") {
  const post = { id: `TESTPOST${++postSeq}`, root_id, create_at: Date.now(), user: "drasi",
    props: { ignite_card: card, ignite_run_epoch: epoch, attachments: [{ fallback: attachment.title, ...attachment }] } };
  posts.push(post);
  return post;
}
const once = (key, make) => { if (!posted.has(key)) { posted.add(key); make(); } };
const changeText = assignments => `Move summaries to ${assignments.batch} and support to ${assignments.support}`;
function syncCards() {
  for (const d of deployments) {
    if (d.mode === "failed") once(`failed-${d.id}`, () => {
      const runs = workloads.filter(w => w.deployment === d.id).map(w => w.id === "batch" ? "summaries" : w.id);
      postCard("failed", { color: colour.red, title: `Deployment failed: ${d.id}`,
        text: `${d.id} has reported Failed for 3 seconds. ${runs.includes("support") ? "Support runs there, so its requests are failing." : "Nothing runs there now."}`,
        fields: [field("Region", d.region), field("Capacity", `${d.capacity_tpm / 1000}k tokens/min`), field("Runs here", runs.join(", ") || "nothing")], footer: "Drasi · incident-failed" });
    });
    else posted.delete(`failed-${d.id}`);
  }
  const overdue = Object.values(extraRows["overdue-tasks"] ?? {}).filter(row => row.workload === "support").length;
  if (overdue) once("overdue", () => postCard("overdue", { color: colour.red, title: "Support requests overdue",
    text: `${overdue} customer-support requests are past their 15-second deadline. More follow until support can run somewhere.`,
    fields: [field("Overdue when posted", String(overdue)), field("Deadline", "15 s"), field("Dashboard", `[Operations › Support overdue](http://localhost:${port("GRAFANA")}/d/ignite-workload)`)], footer: "Drasi · incident-overdue" }));
  else posted.delete("overdue");
  const p = plan();
  if (p.state === "feasible" && p.moves > 0) once(`proposal-${p.candidate_id}`, () => {
    const change = changeText(JSON.parse(p.assignments));
    const context = { candidate_id: p.candidate_id, basis: p.basis, revision: p.revision, run_epoch: epoch, change };
    const url = `http://host.containers.internal:${port("WORKLOAD")}/mattermost/actions`;
    const down = deployments.filter(d => d.mode !== "ok").map(d => `${d.id} is down`).join(", ");
    const post = postCard("proposal", { color: colour.orange, title: `Proposed change: ${change}`,
      text: "Drasi's plan for the current facts: every workload on a healthy deployment its policy allows, within capacity, with the fewest moves. Approve to apply it through the gateway's configuration API.",
      fields: [field("Why", down || "Capacity changed"), field("Peak use after", "80%"),
        field("Policy", "support: EU-A, EU-B (restricted data, EU only) · summaries: EU-A, EU-B, US-C", false),
        field("Capacity", `healthy: ${deployments.filter(d => d.mode === "ok").map(d => `${d.id} ${d.capacity_tpm / 1000}k`).join(", ")} · needed: support 60k, summaries 80k`, false),
        field("Based on", `revision ${p.revision} · plan ${p.candidate_id.slice(-8)}`)],
      footer: "Drasi · incident-proposal",
      actions: [{ id: "approve", name: "Approve", type: "button", style: "primary", integration: { url, context: { action: "approve", ...context } } },
        { id: "decline", name: "Reject", type: "button", style: "danger", integration: { url, context: { action: "decline", ...context } } }] });
    lastProposal = { candidate_id: p.candidate_id, change, post };
  });
  if (lastProposal && (p.state !== "feasible" || p.candidate_id !== lastProposal.candidate_id)
      && !approvals.some(a => a.candidate_id === lastProposal.candidate_id && ["applied", "verified"].includes(a.status))) {
    const was = lastProposal;
    once(`superseded-${was.candidate_id}`, () => {
      const now = p.state === "feasible" && p.moves > 0 ? changeText(JSON.parse(p.assignments)) : null;
      postCard("superseded", { color: colour.grey, title: "Out of date: the facts changed",
        text: `The proposed change "${was.change}" no longer matches Drasi's plan, so Drasi would reject an approval of it. ${now ? "Drasi's new proposal follows." : `No change fits policy and capacity now: ${p.reason}.`}`,
        fields: [field("Was", was.change, false), field("Now", now ?? "No valid plan", false)], footer: "Drasi · incident-superseded" });
    });
  }
  for (const a of approvals) {
    if (!["applied", "verified", "rejected"].includes(a.status)) continue;
    once(`${a.id}-${a.status}`, () => {
      const by = field("Approved by", a.requested_by ?? "desk");
      const card = a.status === "applied" ? { color: colour.blue, title: `Applied: ${a.change}`,
          text: "Drasi applied the approved change through the gateway's configuration API. It calls it fixed only when the backlog has cleared and new support requests are answered on the new route.",
          fields: [by, field("Routes", "support → EU-B · summaries → US-C")] }
        : a.status === "verified" ? { color: colour.green, title: "Verified: support has recovered",
          text: "The backlog from before the change has cleared and new support requests are answered on the new route.", fields: [by, field("Change", a.change)] }
        : { color: colour.orange, title: "Rejected by Drasi: nothing was changed", text: `${a.reason}.`, fields: [by, field("Change", a.change)] };
      postCard("outcome", { ...card, footer: "Drasi · incident-outcome" }, a.post_id ?? "");
    });
  }
  const euA = deployments.find(d => d.id === "EU-A");
  const healthy = deployments.every(d => d.mode === "ok" && d.reporting);
  if (healthy && euA.capacity_tpm < 100000) once("at-risk", () => {
    posted.delete("restored");
    postCard("at-risk", { color: colour.orange, title: "Failover at risk: losing EU-B leaves support nowhere to run",
      text: "Everything is healthy, but if EU-B failed, support would have nowhere allowed to run. Support needs 60k in Europe and EU-A has 50k.",
      fields: [field("Would fail", "EU-B"), field("Affects", "support")], footer: "Drasi · incident-at-risk" });
  });
  if (posted.has("at-risk") && healthy && euA.capacity_tpm === 100000) once("restored", () => {
    posted.delete("at-risk");
    postCard("restored", { color: colour.green, title: "Failover restored", text: "Losing any one deployment now still leaves a valid plan for every workload.", fields: [], footer: "Drasi · incident-restored" });
  });
}
setInterval(syncCards, 200);

// TEST FIXTURE stand-in for POST /mattermost/actions: a button pressed on a card, as @oncall.
function pressCard(post, action) {
  const context = post.props.attachments[0].actions?.find(a => a.id === action)?.integration.context;
  if (!context) return null;
  const id = `mm-${post.id}`;
  if (!approvals.some(a => a.id === id)) {
    for (const a of approvals) a.latest = false;
    const approval = { id, candidate_id: context.candidate_id, basis: context.basis, revision: context.revision, status: action === "approve" ? "pending" : "declined",
      reason: action === "approve" ? "" : "Rejected by @oncall", latest: true, requested_by: "@oncall", post_id: post.id, change: context.change };
    approvals.unshift(approval);
    if (action === "approve") setTimeout(() => decide(approval), 300);
  }
  const at = new Date().toTimeString().slice(0, 8);
  const attachment = post.props.attachments[0];
  delete attachment.actions;
  attachment.fields = [...attachment.fields, field("Decision", action === "approve"
    ? `Approved by @oncall at ${at}. Drasi checks it still matches its current plan before anything changes.`
    : `Rejected by @oncall at ${at}. Nothing will change.`, false)];
  return id;
}
const channelPage = fs.readFileSync(new URL("./mattermost-stand-in.html", import.meta.url), "utf8");

function json(response, value, status = 200) { response.writeHead(status, { "Content-Type": "application/json" }); response.end(JSON.stringify(value)); }
async function body(request) { let text = ""; for await (const chunk of request) text += chunk; return text ? JSON.parse(text) : {}; }
const health = () => ({ run_epoch: epoch, revision, pending: 1, task_target_seconds: 15, report_expiry_seconds: 8, traffic, deployments, workloads, destinations });
const server = createServer(async (request, response) => {
  const url = new URL(request.url, "http://localhost");
  if (url.pathname === "/test/reset") { reset(); syncOutputs(); return json(response, { fixture: true }); }
  if (url.pathname === "/test/state") return json(response, { mutations, revision, approvals });
  if (url.pathname === "/test/offline") { unavailable = (await body(request)).value; return json(response, {}); }
  if (url.pathname === "/test/expire") { expired = true; return json(response, {}); }
  if (url.pathname === "/test/mattermost/channel") { response.writeHead(200, { "Content-Type": "text/html" }); return response.end(channelPage); }
  if (url.pathname === "/test/mattermost/posts") return json(response, posts);
  if (url.pathname === "/test/mattermost/press") {
    const input = await body(request);
    const post = posts.find(p => p.id === input.post_id);
    mutations.push({ path: "/mattermost/actions", input: { post_id: input.post_id, action: input.action } });
    return post && pressCard(post, input.action) ? json(response, { ephemeral_text: "Sent to the change-approval tool." }) : json(response, { error: "no such button" }, 400);
  }
  if (url.pathname === "/test/recover") { const latest = approvals.find(a => a.latest); if (latest) latest.status = "verified"; return json(response, {}); }
  if (url.pathname === "/test/query") {
    const input = await body(request);
    extraRows[input.query] = input.rows;
    if (input.disconnect) for (const [stream, state] of streams) if (state.name === input.query) stream.end();
    return json(response, {});
  }
  if (url.pathname === "/drasi-stream/events") {
    response.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache", Connection: "keep-alive" });
    response.write(": connected\n\n");
    eventStreams.add(response);
    const timer = setInterval(() => response.write(`data: ${JSON.stringify({ type: "heartbeat", ts: Date.now() })}\n\n`), 1000);
    response.on("close", () => { clearInterval(timer); eventStreams.delete(response); });
    return;
  }
  if (url.pathname.startsWith("/drasi-stream/snapshot/")) {
    const alias = decodeURIComponent(url.pathname.split("/").at(-1));
    if (!outputs[alias]) return json(response, { error: "unknown alias" }, 404);
    syncOutputs();
    const { rows, sequence } = outputs[alias];
    response.writeHead(200, { "Content-Type": "application/json", "x-drasi-output-epoch": "TEST-FIXTURE-EPOCH", "x-drasi-output-position": String(sequence) });
    return response.end(JSON.stringify(Object.values(rows).map(row => ({ operation: "ADD", identity: identity(alias, row), element: { kind: "node", labels: ["Result"] }, after: row, effectiveFrom: Date.now() }))));
  }
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
    if (url.pathname === "/mattermost/approve-latest") {
      const post = posts.findLast(p => p.props.ignite_card === "proposal" && p.props.ignite_run_epoch === input.run_epoch
        && p.props.attachments[0].actions?.some(a => a.id === "approve"));
      if (!post) return json(response, { error: "No proposed change to approve in #incident-ai-support" }, 404);
      return json(response, { post_id: post.id, approval: pressCard(post, "approve") });
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
