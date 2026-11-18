// The Drasi configuration behind each Decision Support panel, shown on the projected dashboard
// beside the highlighted panel (a beat's `config` in notes.ts; Audience.tsx and ConfigSlides.tsx).
// The text comes from demo/drasi as Drasi runs it: comments dropped, long JSON detail elided.
// flow: what the panel brings together, as the generated graph wires it (demo/.native/server.json):
// inputs on the left, then the Drasi components, then the Grafana tile.
export type Flow = { inputs: { name: string; from: string }[]; via: string[]; tile: string };
export type ConfigPanel = { tab: string; lines: string[]; hot: string[]; flow: Flow };
const pg = (name: string) => ({ name, from: "PostgreSQL" });
export const flows: Record<ConfigId, Flow> = {
  "overdue-tasks": { inputs: [pg("ignite_tasks"), pg("ignite_scenario")], via: ["overdue-tasks · tile-overdue"], tile: "Support overdue" },
  "deployment-health": { inputs: [{ name: "ignite_probe_status", from: "OpenTelemetry" }, pg("ignite_deployments")],
    via: ["deployment-health", "tile-deployments"], tile: "Deployment status" },
  "tile-workloads": { inputs: [pg("ignite_routes"), pg("ignite_workloads"), { name: "policy-results", from: "Policy model" },
    { name: "deployment-health", from: "OpenTelemetry" }], via: ["tile-workloads"], tile: "Workload status" },
  policy: { inputs: [pg("ignite_workloads"), pg("ignite_deployments"), pg("ignite_policy_destinations")],
    via: ["policy-input", "policy (Rego)", "policy-results → plan"], tile: "Recommended action" },
  optimization: { inputs: [{ name: "plan-items, plan-candidates", from: "workloads, routes" }, { name: "plan-bins", from: "capacity + health" },
    { name: "policy-results", from: "Policy model" }], via: ["plan (optimization)", "plan-results · tile-action"], tile: "Recommended action" },
  "what-if": { inputs: [{ name: "plan-bins", from: "healthy deployments" }, { name: "plan-items, plan-candidates", from: "workloads, routes" },
    { name: "policy-results", from: "Policy model" }], via: ["plan · loss scenario", "resilience-results · tile-resilience"], tile: "Failover readiness" },
};
export type Sources = { overdueQuery: string; healthQuery: string; workloadsQuery: string; rego: string; optimization: object };
export const configIds = ["overdue-tasks", "deployment-health", "tile-workloads", "policy", "optimization", "what-if"] as const;
export type ConfigId = typeof configIds[number];

const withoutComments = (text: string, comment: string) => text.split("\n")
  .filter(line => line.trim() && !line.trim().startsWith(comment)).map(line => line.replace(/\t/g, "    "));

// With health from telemetry the config generator points the health queries at deployment-health's
// rows (label DeploymentHealth) instead of the ignite_reports table (demo/ops/config/src/main.rs).
const relabel = (lines: string[]) => lines.map(line => line.replace(/:ignite_reports\b/g, ":DeploymentHealth"));

// optimization.json's parts, one entry per line; nested detail ("terms", "where", "output") elided.
const elided = new Set(["terms", "where", "output", "match"]);
const value = (key: string, v: unknown, elide: boolean): string => elide && elided.has(key) ? (Array.isArray(v) ? "[…]" : "{…}")
  : Array.isArray(v) ? `[${v.map(x => ` ${value("", x, elide)}`).join(",")} ]`
  : v && typeof v === "object" ? `{ ${Object.entries(v).map(([k, x]) => `"${k}": ${value(k, x, elide)}`).join(", ")} }` : JSON.stringify(v);
const entry = (item: object, elide = true) => value("", item, elide);

export function buildConfig({ overdueQuery, healthQuery, workloadsQuery, rego, optimization }: Sources): Record<ConfigId, ConfigPanel> {
  const model = optimization as Record<string, unknown> & Record<"variables" | "constraints" | "objectives" | "scenarios" | "sets", { name: string }[]>;
  const part = (name: "variables" | "constraints" | "objectives") =>
    [`"${name}": [`, ...model[name].map((item, i, all) => `  ${entry(item)}${i < all.length - 1 ? "," : " ],"}`)];
  const regoLines = withoutComments(rego, "#").filter(line => !/^(package|import) /.test(line));
  const panels: Record<ConfigId, Omit<ConfigPanel, "flow">> = {
    "overdue-tasks": { tab: "Continuous query · overdue-tasks → Support overdue", hot: ["drasi.trueLater"],
      lines: withoutComments(overdueQuery, "//") },
    "deployment-health": { tab: "Continuous query · deployment-health, from the OpenTelemetry source → Deployment status", hot: ["CASE toInteger(m.value)", "+ 8000"],
      lines: withoutComments(healthQuery, "//") },
    "tile-workloads": { tab: "Continuous query · tile-workloads → Workload status", hot: ["ROUTE_POLICY", "ROUTE_REPORT"],
      lines: relabel(withoutComments(workloadsQuery, "//")) },
    policy: { tab: "Continuous model · policy · drasi.standard/policy · ignite.rego (rules) → Recommended action",
      hot: ["allowed_destination if", "region_permitted if input.subject.region_requirement == input.target.region"],
      lines: regoLines.slice(0, regoLines.findIndex(line => line.startsWith("reasons contains"))).concat(regoLines.filter(line => line.startsWith("assessment"))) },
    optimization: { tab: "Continuous model · plan · drasi.standard/optimization · optimization.json (excerpt) → Recommended action",
      hot: ['"name": "assign-once"', '"name": "capacity"', '"name": "moves"'],
      lines: [...part("variables"), ...part("constraints"), ...part("objectives")] },
    "what-if": { tab: "Continuous model · plan · optimization.json (excerpt): the what-if → Failover readiness", hot: ['"name": "loss"'],
      lines: [`"atomicScenarios": ${JSON.stringify(model.atomicScenarios)},`, '"sets": [ …',
        `  ${entry(model.sets.find(set => set.name === "healthy")!, false)}, … ],`,
        '"scenarios": [', ...model.scenarios.map(item => `  ${entry(item)} ],`)] },
  };
  return Object.fromEntries(configIds.map(id => [id, { ...panels[id], flow: flows[id] }])) as Record<ConfigId, ConfigPanel>;
}
