// The Drasi configuration behind each Decision Support panel, shown on the projected dashboard
// beside the highlighted panel (a beat's `config` in notes.ts; Audience.tsx and ConfigSlides.tsx).
// The text comes from demo/drasi as Drasi runs it: comments dropped, long JSON detail elided.
// flow: what the panel brings together, grouped by where it comes from (as the generated graph wires
// it, demo/.native/server.json), and what feeds the code shown.
export type Flow = { inputs: { from: string; items: string }[]; into: string };
export type ConfigPanel = { tab: string; lines: string[]; hot: string[]; flow: Flow; component: string };
// The Drasi component each card shows, by its real id (the same id labels its node in the how-it-works graph).
export const componentOf: Record<ConfigId, string> = { "overdue-tasks": "overdue-tasks", "deployment-health": "deployment-health",
  "tile-workloads": "tile-workloads", policy: "policy", optimization: "plan", "what-if": "plan" };
const db = (items: string) => ({ from: "AI service database", items });
const otel = { from: "OpenTelemetry", items: "health checks" };
const placements = { from: "Drasi policy model", items: "allowed placements" };
export const flows: Record<ConfigId, Flow> = {
  "overdue-tasks": { inputs: [db("requests, their deadlines and answers")], into: "the overdue query" },
  "deployment-health": { inputs: [otel, db("deployments")], into: "the health query" },
  "tile-workloads": { inputs: [db("workloads, routes"), placements, otel], into: "the workload status query" },
  policy: { inputs: [db("workloads, deployments, allowed destinations")], into: "the policy model (Rego)" },
  optimization: { inputs: [db("workloads, routes, demand, capacity"), otel, placements], into: "the optimization model" },
  "what-if": { inputs: [db("workloads, routes, demand, capacity"), otel, placements], into: "the optimization model's what-if" },
};

export type Sources = { overdueQuery: string; healthQuery: string; workloadsQuery: string; rego: string; optimization: object };
export const configIds = ["overdue-tasks", "deployment-health", "tile-workloads", "policy", "optimization", "what-if"] as const;
export type ConfigId = typeof configIds[number];

const withoutComments = (text: string, comment: string) => text.split("\n")
  .filter(line => line.trim() && !line.trim().startsWith(comment)).map(line => line.replace(/\t/g, "    "));

// With health from telemetry the config generator points the health queries at deployment-health's
// rows (label DeploymentHealth) instead of the deployment_health_reports table (demo/ops/config/src/main.rs).
const relabel = (lines: string[]) => lines.map(line => line.replace(/:deployment_health_reports\b/g, ":DeploymentHealth"));

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
  const panels: Record<ConfigId, Omit<ConfigPanel, "flow" | "component">> = {
    "overdue-tasks": { tab: "Continuous query · overdue-tasks", hot: ["drasi.trueLater"],
      lines: withoutComments(overdueQuery, "//") },
    "deployment-health": { tab: "Continuous query · deployment-health, from the OpenTelemetry source", hot: ["CASE toInteger(m.value)", "+ 8000"],
      lines: withoutComments(healthQuery, "//") },
    "tile-workloads": { tab: "Continuous query · tile-workloads", hot: ["ROUTE_POLICY", "ROUTE_HEALTH"],
      lines: relabel(withoutComments(workloadsQuery, "//")) },
    policy: { tab: "Continuous model · policy · drasi.standard/policy · placement.rego (rules)",
      hot: ["allowed_destination if", "region_permitted if input.subject.region_requirement == input.target.region"],
      lines: regoLines.slice(0, regoLines.findIndex(line => line.startsWith("reasons contains"))).concat(regoLines.filter(line => line.startsWith("assessment"))) },
    optimization: { tab: "Continuous model · plan · drasi.standard/optimization · optimization.json (excerpt)",
      hot: ['"name": "assign-once"', '"name": "capacity"', '"name": "moves"'],
      lines: [...part("variables"), ...part("constraints"), ...part("objectives")] },
    "what-if": { tab: "Continuous model · plan · optimization.json (excerpt): the what-if", hot: ['"name": "loss"'],
      lines: [`"atomicScenarios": ${JSON.stringify(model.atomicScenarios)},`, '"sets": [ …',
        `  ${entry(model.sets.find(set => set.name === "healthy")!, false)}, … ],`,
        '"scenarios": [', ...model.scenarios.map(item => `  ${entry(item)} ],`)] },
  };
  return Object.fromEntries(configIds.map(id => [id, { ...panels[id], flow: flows[id], component: componentOf[id] }])) as Record<ConfigId, ConfigPanel>;
}
