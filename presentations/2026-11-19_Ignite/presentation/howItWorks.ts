// "How it works" cards: the components a scene connects, as a small graph (sources → continuous
// queries and models → reactions → where the answer shows up), shown over the dashboard (or the
// channel) next to the panels they produce, before the query, policy or optimization behind them.
export type Role = "source" | "query" | "model" | "reaction" | "lands";
// "component" is the Drasi component's real id, as the code cards name it (codeSlides.ts).
export type Node = { id: string; role: Role; col: 0 | 1 | 2 | 3; row: number; label: string; detail?: string; component?: string };
export type Flow = { nodes: Node[]; edges: [string, string][] };
// A label for a whole dashboard row (the "what I'll show you" beats).
export type Label = { label: string; chips?: { icon: string; label: string }[]; text?: string; drasi?: true };
export type How = Flow | Label;
export const isLabel = (card: How): card is Label => "label" in card;

export const howIds = ["how-status", "how-overdue", "how-readiness", "how-proposal", "label-standard", "label-drasi"] as const;
export type HowId = typeof howIds[number];
export const isHow = (id: string): id is HowId => (howIds as readonly string[]).includes(id);

const n = (id: string, role: Role, col: Node["col"], row: number, label: string, detail?: string): Node => ({ id, role, col, row, label, detail });
// A Drasi query or model node: its plain name, with its real component id underneath.
const c = (id: string, role: Role, col: Node["col"], row: number, label: string, component: string): Node => ({ id, role, col, row, label, component });

export const how: Record<HowId, How> = {
  "how-status": {
    nodes: [n("otel", "source", 0, 0, "OpenTelemetry", "health checks, every second"), n("db", "source", 0, 1, "AI service database", "workloads, routes, policy"),
      c("dh", "query", 1, 0, "Deployment health", "deployment-health"), c("ws", "query", 1, 1, "Workload status", "tile-workloads"),
      n("prom", "reaction", 2, 0, "Prometheus reaction", "a metric per answer"),
      n("grafana", "lands", 3, 0, "Grafana", "Deployment status, Workload status")],
    edges: [["otel", "dh"], ["db", "ws"], ["dh", "ws"], ["dh", "prom"], ["ws", "prom"], ["prom", "grafana"]],
  },
  "how-overdue": {
    nodes: [n("db", "source", 0, 0, "AI service database", "customer requests and answers"), n("otel", "source", 0, 1, "OpenTelemetry", "health checks, every second"),
      c("q", "query", 1, 0, "Overdue requests", "overdue-tasks"), c("dh", "query", 1, 1, "Deployment health", "deployment-health"),
      n("prom", "reaction", 2, 0, "Prometheus reaction", "a metric per answer"), n("chan", "reaction", 2, 1, "Incident channel", "when support falls behind"),
      n("grafana", "lands", 3, 0, "Grafana", "Support overdue, Deployment status"), n("mm", "lands", 3, 1, "Mattermost", "a card")],
    edges: [["db", "q"], ["otel", "dh"], ["q", "prom"], ["dh", "prom"], ["q", "chan"], ["prom", "grafana"], ["chan", "mm"]],
  },
  "how-readiness": {
    nodes: [n("db", "source", 0, 0, "AI service database", "residency policy, capacity, routes"), n("otel", "source", 0, 1, "OpenTelemetry", "health checks"),
      c("pol", "model", 1, 0, "Policy (Rego)", "policy"), c("plan", "model", 1, 1, "Plan (optimization)", "plan"),
      n("prom", "reaction", 2, 0, "Prometheus reaction", "a metric per answer"), n("chan", "reaction", 2, 1, "Incident channel", "at risk and restored"),
      n("grafana", "lands", 3, 0, "Grafana", "Failover readiness, Recommended action"), n("mm", "lands", 3, 1, "Mattermost", "cards")],
    edges: [["db", "pol"], ["db", "plan"], ["otel", "plan"], ["pol", "plan"], ["plan", "prom"], ["plan", "chan"], ["prom", "grafana"], ["chan", "mm"]],
  },
  "how-proposal": {
    nodes: [n("otel", "source", 0, 0, "OpenTelemetry", "health checks"), n("db", "source", 0, 1, "AI service database", "policy, capacity, approvals"),
      c("pol", "model", 1, 0, "Policy (Rego)", "policy"), c("plan", "model", 1, 1, "Plan (optimization)", "plan"), c("verify", "query", 1, 2, "Recovery verified", "recovery-verified"),
      n("chan", "reaction", 2, 0, "Incident channel", "proposal, with Approve"), n("api", "reaction", 2, 1, "Gateway configuration API", "applies approved routes"),
      n("card", "lands", 3, 0, "This card", "and its thread")],
    edges: [["otel", "plan"], ["db", "pol"], ["pol", "plan"], ["db", "verify"], ["plan", "chan"], ["plan", "api"], ["verify", "chan"], ["chan", "card"]],
  },
  "label-standard": { label: "The basics",
    chips: [{ icon: "otel", label: "OpenTelemetry" }, { icon: "prometheus", label: "Prometheus" }, { icon: "prometheus", label: "Grafana" }],
    text: "It tells you something is wrong" },
  "label-drasi": { label: "Context, answers, guidance", drasi: true,
    text: "Who's affected, what to do about it, and whether you'd survive the next failure" },
};
