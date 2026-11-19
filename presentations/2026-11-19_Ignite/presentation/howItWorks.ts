// "How it works" cards: the components a scene connects, shown over the dashboard (or the channel)
// just above the panel they produce, before the query, policy or optimization behind it.
export type Part = { kind: "pg" | "otel" | "query" | "clock" | "model" | "prometheus" | "mattermost" | "api"; label: string; detail?: string };
export type How = { sources: Part[]; drasi: Part[]; reactions: Part[]; lands: string };

export const howIds = ["how-status", "how-overdue", "how-readiness", "how-proposal"] as const;
export type HowId = typeof howIds[number];
export const isHow = (id: string): id is HowId => (howIds as readonly string[]).includes(id);

const db = (detail: string): Part => ({ kind: "pg", label: "AI service database", detail });
const health: Part = { kind: "otel", label: "OpenTelemetry", detail: "health checks, every second" };
const grafana: Part = { kind: "prometheus", label: "Prometheus reaction", detail: "a metric per answer → Grafana" };
const channel = (detail: string): Part => ({ kind: "mattermost", label: "Incident channel", detail });

export const how: Record<HowId, How> = {
  "how-status": {
    sources: [db("deployments, workloads, routes, policy"), health],
    drasi: [{ kind: "query", label: "Deployment health", detail: "latest check per deployment" },
      { kind: "query", label: "Workload status", detail: "where each runs: allowed? healthy?" }],
    reactions: [grafana],
    lands: "Deployment status, Workload status",
  },
  "how-overdue": {
    sources: [db("customer requests and answers")],
    drasi: [{ kind: "query", label: "Overdue requests", detail: "every request not yet answered" },
      { kind: "clock", label: "trueLater", detail: "re-checks a request at its deadline" }],
    reactions: [grafana, channel("a card when support falls behind")],
    lands: "Support overdue",
  },
  "how-readiness": {
    sources: [db("data residency policy, deployments, capacity, routes"), health],
    drasi: [{ kind: "model", label: "Policy (Rego)", detail: "where each workload may run" },
      { kind: "model", label: "Optimization", detail: "the best plan, and a what-if for losing each deployment" }],
    reactions: [grafana, channel("at risk and restored cards")],
    lands: "Failover readiness, Recommended action",
  },
  "how-proposal": {
    sources: [health, db("policy, capacity, routes, approvals")],
    drasi: [{ kind: "model", label: "Policy + optimization", detail: "fewest moves within policy" },
      { kind: "query", label: "Verify", detail: "backlog cleared on the new route" }],
    reactions: [channel("the proposal card, with Approve"), { kind: "api", label: "Gateway configuration API", detail: "applies the approved routes" }],
    lands: "This card, and its thread",
  },
};
