// Diagnostics views (demo/ui/src/views.tsx), not part of the talk.
export type View = "operations" | "decisions" | "change" | "trace";

// Kiosk mode hides Grafana's menus; the _dash options hide the time picker and links too.
const grafana = (uid: string, kiosk = false) =>
  `http://localhost:13000/d/${uid}?from=now-5m&to=now&refresh=5s${kiosk ? "&kiosk&_dash.hideTimePicker&_dash.hideLinks&_dash.hideVariables" : ""}`;
export const products = {
  grafana: grafana("ignite-workload"),
  decisions: grafana("ignite-decisions"),
  langfuse: "http://localhost:13001/project/ignite-probe/traces?filter=isRootObservation%3Bboolean%3B%3B%3D%3Btrue",
  server: "http://127.0.0.1:18421/ui/?instance=ignite",
  repository: "https://github.com/drasi-project/community/tree/agentofreality-ignite/presentations/2026-11-19_Ignite",
  repositoryLabel: "github.com/drasi-project/community",
  drasi: "https://drasi.io",
};
// Dashboards as projected: kiosk mode (no Grafana toolbar), embedded in the stage.
export const dashboards = { ops: grafana("ignite-workload", true), plan: grafana("ignite-decisions", true) } as const;

// Shown on the title slide when set.
export const speaker = { name: "", role: "" };

export const slides = [
  { id: "title", title: "Title" },
  { id: "concepts", title: "The scenario" },
  { id: "drasi", title: "Drasi in one picture" },
  { id: "query", title: "Who is affected: the continuous query" },
  { id: "closing", title: "Close" },
] as const;

// "What Drasi just did" visuals, shown after each part of the live demo.
// Roles left out did not take part in that step and are shown dimmed.
export type Recap = {
  id: string; after: string; title: string; saw: string[];
  sources?: string; queries?: string; models?: string; reactions?: string; why: string;
};
export const recaps = [
  {
    id: "drasi-healthy", after: "normal", title: "Drasi's answers sit beside your metrics",
    saw: ["Real requests, measured by OpenTelemetry, Prometheus and Grafana",
      "A second row of answers computed by Drasi: overdue requests, deployment status, recovery, action needed, safety"],
    sources: "Every change in the service's database: requests, deployment reports, capacity, policy",
    queries: "Overdue support requests · Deployment status",
    models: "Policy, plan and what-if: today's plan fits, and survives losing any one deployment",
    reactions: "Stream each change to the AI service, which exports it to Grafana",
    why: "No new tool to watch: Drasi's answers arrive in the dashboards you already use.",
  },
  {
    id: "drasi-failure", after: "failure", title: "Drasi knew which customers were late, the moment they were late",
    saw: ["EU-A failed: errors and the queue rose, then Grafana alerted",
      "Each support request was counted as overdue the moment it passed its 15-second deadline, with no new event"],
    sources: "Failed attempts, and EU-A's health reports turning to Failed",
    queries: "Overdue support requests: time-aware, so a request becomes late as time passes",
    models: "Plan: the current routes no longer work, so a new plan is proposed",
    reactions: "Support overdue, Deployment status and Action needed: Approve proposed plan",
    why: "Nothing re-scans for late requests: Drasi re-checks each one at its deadline.",
  },
  {
    id: "drasi-policy", after: "policy", title: "Change the rules and the answer changes",
    saw: ["Summaries were restricted to Europe: with EU-A down, no plan fits the rules, and Drasi said so at once",
      "Approving the plan selected before the change was rejected",
      "Allowing the US again brought the valid plan straight back"],
    sources: "The policy change",
    queries: "Decision context: the current settings and each deployment's health, which the policy reads",
    models: "Policy re-evaluated, plan recomputed: no valid plan, so the earlier plan no longer matches the facts",
    reactions: "Policy tiles and Action needed updated; the AI service refused the out-of-date approval",
    why: "Every plan carries the facts it was based on, so an out-of-date approval can't be applied.",
  },
  {
    id: "drasi-recovery", after: "recovery", title: "A fix within policy, and proof it worked",
    saw: ["Support can't use US-C; summaries can", "One approval moved summaries to US-C and support to EU-B",
      "Recovery showed Verified only after the queue cleared and support requests were answered on the new route"],
    sources: "Health reports, capacity and policy; then your approval and the answered requests",
    queries: "Verified recovery: queue cleared, and at least three support requests answered on the new route",
    models: "Policy: support may only run in Europe. Plan: the complete plan that fits",
    reactions: "The plan offered for approval, then Recovery: Verified in Grafana and Langfuse",
    why: "Approving a change isn't the same as fixing the problem. Drasi checks the outcome.",
  },
  {
    id: "drasi-resilience", after: "resilience", title: "Healthy now, but not safe from the next failure",
    saw: ["Everything green, but EU-A came back with only 50k of capacity",
      "Safe if one fails? No: not if EU-B fails. Then Yes once EU-A was back to 100k"],
    sources: "The capacity change on EU-A",
    queries: "Deployment status",
    models: "What-if: for each deployment, if it failed, would every workload still fit somewhere allowed?",
    reactions: "The Safe if one fails? tile, and Action needed: At risk",
    why: "Drasi answers 'are we safe?' continuously, before the outage happens.",
  },
] as const satisfies readonly Recap[];
export const allSlides = [...slides, ...recaps.map(recap => ({ id: recap.id, title: recap.title }))] as const;
export type SlideId = typeof slides[number]["id"] | typeof recaps[number]["id"];

// What the projected stage shows: a slide, a dashboard, or one late request.
export type StageView = SlideId | "ops" | "plan" | "request";
export const viewLabel = (view: StageView) => view === "ops" ? "Grafana · Operations"
  : view === "plan" ? "Grafana · Recovery plan" : view === "request" ? "One late request"
  : view.startsWith("drasi-") ? "What Drasi just did" : slides.find(slide => slide.id === view)?.title ?? view;

// The talk's parts. Timings are minutes from the start.
export const scenes = [
  { id: "title", title: "Title", start: 0, minutes: .5 },
  { id: "concepts", title: "The scenario", start: .5, minutes: 1.5 },
  { id: "drasi", title: "Drasi in one picture", start: 2, minutes: 1.5 },
  { id: "normal", title: "Healthy service", start: 3.5, minutes: 2.5 },
  { id: "failure", title: "Failure", start: 6, minutes: 3 },
  { id: "who", title: "Who is affected", start: 9, minutes: 2.5 },
  { id: "recovery", title: "Recovery", start: 11.5, minutes: 5.5 },
  { id: "resilience", title: "Healthy but fragile", start: 17, minutes: 3.5 },
  { id: "closing", title: "Close", start: 20.5, minutes: 1.5 },
] as const;
export type SceneId = typeof scenes[number]["id"];
export const timeLabel = (minutes: number) => `${Math.floor(minutes)}:${Math.round((minutes % 1) * 60).toString().padStart(2, "0")}`;
export const duration = (minutes: number) => minutes < 1 ? `${Math.round(minutes * 60)} seconds`
  : `${Math.floor(minutes)}${minutes % 1 ? "½" : ""} minute${minutes > 1 ? "s" : ""}`;

// The recap for how far the live demo has got, worked out from its decision history since the
// last reset. Used when the projected window is navigated by hand (?slide=drasi).
type AuditRow = { action?: unknown; detail?: unknown };
export function recapFromHistory(audit: readonly AuditRow[]) {
  const order = ["drasi-healthy", "drasi-failure", "drasi-policy", "drasi-recovery", "drasi-resilience"] as const;
  let step = 0;
  for (const row of audit) {
    const detail = (row.detail ?? {}) as { action?: string; model?: {
      policy?: { destinations?: { batch?: string[] } }; endpoints?: { budget_tpm?: number }[] } };
    if (row.action === "control" && (detail.action === "fail" || detail.action === "throttle")) step = Math.max(step, 1);
    if (["approval_rejected", "application_rejected"].includes(String(row.action))) step = Math.max(step, 2);
    if (row.action === "configuration" && detail.model?.policy?.destinations?.batch && !detail.model.policy.destinations.batch.includes("US-C")) step = Math.max(step, 2);
    if (["gateway_applied", "recovery_confirmed"].includes(String(row.action))) step = Math.max(step, 3);
    if (row.action === "configuration" && detail.model?.endpoints?.some(endpoint => (endpoint.budget_tpm ?? 100000) < 100000)) step = Math.max(step, 4);
  }
  return recaps.find(recap => recap.id === order[step])!;
}
