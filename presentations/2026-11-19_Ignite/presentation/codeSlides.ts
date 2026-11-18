// Slides that show the real configuration behind an answer, laid out like the overdue query:
// code in white panels, numbered markers on the lines the numbered notes explain.
// The text comes from demo/drasi as Drasi runs it (comments dropped; long JSON parts elided).
type Panel = { tab: string; lines: string[]; marks: [string, number][] };
type Code = { title: string; panels: Panel[]; notes: string[]; wide?: boolean };

const withoutComments = (text: string, comment: string) => text.split("\n")
  .filter(line => line.trim() && !line.trim().startsWith(comment)).map(line => line.replace(/\t/g, "    "));

// With health from telemetry the config generator points the health queries at deployment-health's
// rows (label DeploymentHealth) instead of the ignite_reports table (demo/ops/config/src/main.rs).
const relabel = (lines: string[]) => lines.map(line => line.replace(/:ignite_reports\b/g, ":DeploymentHealth"));

// optimization.json's parts, one entry per line; nested detail ("terms", "where", "output") elided.
const elided = new Set(["terms", "where", "output", "match"]);
const value = (key: string, v: unknown): string => elided.has(key) ? (Array.isArray(v) ? "[…]" : "{…}")
  : v && typeof v === "object" && !Array.isArray(v)
    ? `{ ${Object.entries(v).map(([k, x]) => `"${k}": ${value(k, x)}`).join(", ")} }` : JSON.stringify(v);
const entry = (item: object) => `{ ${Object.entries(item).map(([k, v]) => `"${k}": ${value(k, v)}`).join(", ")} }`;
export type Sources = { healthQuery: string; tileQuery: string; rego: string; optimization: object };
const optimizationLines = (model: Record<string, object[]>) => ["variables", "constraints", "objectives", "scenarios"].flatMap(part =>
  [`"${part}": [`, ...model[part].map((item, i, all) => `  ${entry(item)}${i < all.length - 1 ? "," : " ],"}`)]);

export type CodeSlideId = "health" | "policy-config" | "optimization-config";
export const codeSlideIds: readonly CodeSlideId[] = ["health", "policy-config", "optimization-config"];

// The slides, built from the files' text (ConfigSlides.tsx passes them in; the tests read them).
export const buildCodeSlides = ({ healthQuery, tileQuery, rego, optimization }: Sources): Record<CodeSlideId, Code> => ({
  health: {
    title: "How Drasi knows EU-A failed",
    panels: [
      { tab: "Continuous query · deployment-health · from the OpenTelemetry source", lines: withoutComments(healthQuery, "//"),
        marks: [["MATCH (svc:Service)", 1], ["CASE toInteger(m.value)", 2], ["+ 8000 AS expires_ms", 3]] },
      { tab: "Continuous query · tile-deployments · the Deployment status tile", lines: relabel(withoutComments(tileQuery, "//")),
        marks: [["CASE WHEN drasi.trueLater", 4], ["WHEN r.status = 'failed'", 5]] },
    ],
    notes: [
      "Every second the AI service probes each deployment and exports the result as an OpenTelemetry metric. The Collector forwards it to Drasi",
      "The latest probe becomes a status: 2 is a failed probe",
      "Each reading is good for 8 seconds",
      "If the probes stop, the status turns Unknown as time passes, with no new event",
      "EU-A shows Failed because its own probes fail; nobody has to tell Drasi",
    ],
  },
  "policy-config": {
    title: "The policy behind the proposal",
    panels: [{ tab: "Continuous model · policy · drasi.standard/policy · drasi/policy/ignite.rego", lines: withoutComments(rego, "#").filter(line => !/^(package|import) /.test(line)),
      marks: [["allowed_destination if", 1], ["region_permitted if input.subject.region_requirement == input.target.region", 2],
        ["input.subject.model == input.target.model", 3], ["assessment :=", 4]] }],
    notes: [
      "Allowed destinations are data: the compliance change edits those rows",
      "Restricted data must stay in its region",
      "The deployment must serve the workload's model",
      "Drasi re-runs the rule for every workload and deployment whenever an input changes; each decision, with its reason, feeds the optimizer",
    ],
  },
  "optimization-config": {
    title: "The optimization behind the proposal",
    wide: true,
    panels: [{ tab: "Continuous model · plan · drasi.standard/optimization · drasi/optimization/optimization.json (excerpt)",
      lines: optimizationLines(optimization as Record<string, object[]>),
      marks: [['"name": "assign"', 1], ['"name": "assign-once"', 2], ['"name": "capacity"', 3], ['"name": "moves"', 4], ['"name": "loss"', 5]] }],
    notes: [
      "One yes/no choice per workload and allowed deployment",
      "Every workload is placed exactly once: nothing is dropped",
      "Load on each healthy deployment stays within its capacity",
      "Fewest moves first, then the lowest peak load",
      "What-if: solve again without each healthy deployment",
    ],
  },
});
