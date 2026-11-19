// Rough slides for the new flow (NEW_FLOW.md): plain text and boxes, to be designed later.
// Each is a title, an optional lead, then bullets and/or a left-to-right flow of columns.
export type Tone = "blue" | "purple" | "green" | "lime" | "gray";
export type Column = { label: string; items: string[]; tone: Tone };
export type RoughSlide = { title: string; lead?: string; bullets?: string[]; flow?: Column[]; note?: string };

const src = (items: string[]): Column => ({ label: "Sources", items, tone: "blue" });
const query = (items: string[], label = "Continuous queries"): Column => ({ label, items, tone: "purple" });
const model = (items: string[]): Column => ({ label: "Continuous models", items, tone: "green" });
const react = (items: string[]): Column => ({ label: "Reactions", items, tone: "lime" });
const out = (items: string[]): Column => ({ label: "Where it lands", items, tone: "gray" });

export const roughIds = ["project", "problem", "drasi-1", "drasi-2", "drasi-3", "plugins", "forms", "issues", "wants",
  "scenario-map", "how-1", "how-2", "how-3", "how-4", "links"] as const;
export type RoughId = typeof roughIds[number];

export const rough: Record<RoughId, RoughSlide> = {
  project: { title: "Drasi", lead: "An open-source project for change-driven systems",
    bullets: ["A CNCF Sandbox project since XXX (TODO: date)", "In active development", "A growing community of users and contributors", "drasi.io · github.com/drasi-project"] },
  problem: { title: "The problem",
    lead: "Detecting meaningful changes in data as they happen, and reacting to them, is harder than it should be",
    bullets: ["Polling and diffing on a schedule: late, and wasteful", "Change feeds and event streams: low-level, per-system plumbing", "Joining changes across systems, and noticing when something didn't happen, is custom code every time"] },
  "drasi-1": { title: "Sources, Continuous Queries, Reactions", lead: "Start simple: one source, one query, one reaction",
    flow: [src(["PostgreSQL"]), query(["Orders that are late"]), react(["Notify the team"])] },
  "drasi-2": { title: "Sources, Continuous Queries, Reactions", lead: "One query can join changes from several sources",
    flow: [src(["PostgreSQL", "OpenTelemetry", "Kubernetes"]), query(["Orders that are late, on unhealthy infrastructure"]), react(["Notify the team"])] },
  "drasi-3": { title: "Sources, Continuous Queries, Reactions", lead: "Many sources, many queries, many reactions: each answer kept current as data changes",
    flow: [src(["PostgreSQL", "OpenTelemetry", "Kubernetes"]), query(["Late orders", "Unhealthy deployments", "Policy violations"]), react(["Dashboards", "Messaging", "Workflows"])] },
  plugins: { title: "Everything is a plugin", lead: "A growing ecosystem of sources and reactions, and tools to build your own (TODO: check the lists against drasi.io)",
    flow: [src(["PostgreSQL", "MySQL", "SQL Server", "Cosmos DB", "Event Hubs", "Kubernetes", "OpenTelemetry", "…"]),
      query(["Cypher continuous queries", "Policy (Rego)", "Optimization"], "Continuous models"),
      react(["HTTP", "SSE", "SignalR", "Event Grid", "Dapr pub/sub", "Storage queues", "Prometheus", "…"])] },
  forms: { title: "Use it the way that fits", lead: "One engine, several form factors (TODO: confirm which are available at Ignite)",
    flow: [{ label: "In your process", items: ["Rust library (drasi-core)", "Node.js", "Python", ".NET"], tone: "blue" },
      { label: "As a service", items: ["Drasi Server: one standalone service", "This demo runs on it"], tone: "purple" },
      { label: "On Kubernetes", items: ["Drasi for Kubernetes", "Sources, queries and reactions as resources"], tone: "green" }] },
  issues: { title: "Observability: what organizations report",
    bullets: ["Too much data", "Too many tools", "Lots of noise: low-value alerts", "Symptoms, not answers: who's affected, what changed, what to do?", "Context lives elsewhere: owners, policy, deployments, business data", "TODO: tighten from the research"] },
  wants: { title: "What they want",
    bullets: ["More context, joined across systems", "Raise the signal above the noise: situations, not individual metrics", "Outcomes, including what didn't happen", "Action backed by evidence, with safeguards", "In the tools they already use"] },
  "scenario-map": { title: "The scenario", lead: "An AI service with two workloads, and the tools its team already uses",
    flow: [{ label: "Workloads", items: ["Customer support: restricted data, answer in 15 s", "Document summaries: public data"], tone: "purple" },
      { label: "Deployments", items: ["EU-A (Europe): support", "EU-B (Europe): summaries", "US-C (US): extra capacity, being provisioned"], tone: "blue" },
      { label: "Policy and business rules", items: ["Restricted data stays in Europe", "Summaries in Europe for now", "Changes need human approval"], tone: "green" },
      { label: "Supporting tools", items: ["OpenTelemetry", "Prometheus and Grafana", "Langfuse", "Mattermost"], tone: "gray" }] },
  "how-1": { title: "How it works: telemetry + configuration → dashboard",
    flow: [src(["OpenTelemetry: health probes", "PostgreSQL: deployments, workloads, routes"]), query(["deployment-health", "tile-deployments", "tile-workloads"]),
      react(["Prometheus reaction"]), out(["Grafana: Deployment status, Workload status"])] },
  "how-2": { title: "How it works: the absence of change",
    flow: [src(["PostgreSQL: AI requests", "OpenTelemetry: health probes"]), query(["overdue-tasks: drasi.trueLater(deadline)", "deployment-health: Unknown 8 s after the last probe"]),
      react(["Prometheus reaction", "Incident channel (HTTP)"]), out(["Grafana: Support overdue, Deployment status", "Mattermost card"])] },
  "how-3": { title: "How it works: policy + optimization → resilience",
    flow: [src(["PostgreSQL: residency policy", "PostgreSQL: deployments, capacity, routes", "OpenTelemetry: health"]), model(["policy (Rego)", "plan: optimization, loss scenario (what-if)"]),
      react(["Prometheus reaction", "Incident channel (HTTP)"]), out(["Grafana: Failover readiness, Recommended action", "Mattermost: at risk / restored"])] },
  "how-4": { title: "How it works: failure → proposal → approved change",
    flow: [src(["OpenTelemetry: health", "PostgreSQL: policy, capacity, approvals"]), model(["policy", "plan: fewest moves within policy"]),
      react(["Incident channel: proposal card", "Gateway configuration API: apply", "Grafana annotations, Langfuse scores"]), out(["Mattermost: Approve on the card", "Verified in Grafana"])] },
  links: { title: "Learn more",
    bullets: ["drasi.io: docs, getting started", "github.com/drasi-project: code, samples, this demo (TODO: link to the demo folder)", "TODO: community calls, Discord"] },
};
