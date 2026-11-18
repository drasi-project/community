import { settings } from "../demo/ui/src/settings";

// Diagnostics views (demo/ui/src/views.tsx), not part of the talk.
export type View = "operations" | "decisions" | "change" | "trace";

// Kiosk mode hides Grafana's menus; the _dash options hide the time picker and links too.
const grafana = (uid: string, kiosk = false) =>
  `${settings.grafana}/d/${uid}?from=now-5m&to=now&refresh=5s${kiosk ? "&kiosk&_dash.hideTimePicker&_dash.hideLinks&_dash.hideVariables" : ""}`;
export const products = {
  grafana: grafana("ignite-workload"),
  decisions: grafana("ignite-decisions"),
  langfuse: `${settings.langfuse}/project/ignite-probe/traces?filter=isRootObservation%3Bboolean%3B%3B%3D%3Btrue`,
  server: `${settings.server}/ui/?instance=ignite`,
  drasi: "https://drasi.io/",
};
// Dashboards as projected: kiosk mode (no Grafana toolbar), embedded in the stage.
export const dashboards = { ops: grafana("ignite-workload", true) } as const;
// The incident channel as projected (contracts.md §17); the projector's browser is signed in as oncall.
export const incidentChannel = `${settings.mattermost}/ai-platform/channels/incident-ai-support`;

// Shown on the title slide when set (bold name, then role, as in the Ignite template).
export const speaker = { name: "", role: "" };
export const session = { event: "Microsoft Ignite 2026", code: "THR1029" };

// The Ignite template's 16:9 slide, in pixels. Projected content stays above captionTop: the
// bottom 78 px are kept clear for closed captions, on slides and on the projected dashboards.
export const frame = { width: 1920, height: 1080, captionTop: 1002 } as const;

export const slides = [
  { id: "walkin", title: "Walk-in" },
  { id: "title", title: "Title" },
  // The scenario reveals its three sections one desk beat at a time.
  { id: "concepts", title: "The scenario · deployments" },
  { id: "concepts-rules", title: "The scenario · rules" },
  { id: "concepts-questions", title: "The scenario · three answers" },
  { id: "setup", title: "What you'll see" },
  { id: "drasi", title: "Drasi: change-driven computation" },
  { id: "query", title: "Who is affected: the continuous query" },
  { id: "closing", title: "Close" },
  { id: "survey", title: "Session survey" },
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
    queries: "Overdue support requests · Deployment status · one query per Grafana tile",
    models: "Policy, plan and what-if: today's plan fits, and survives losing any one deployment",
    reactions: "Publish each answer as a Prometheus metric for Grafana",
    why: "No new tool to watch: Drasi's answers arrive in the dashboards you already use.",
  },
  {
    id: "drasi-failure", after: "failure", title: "Drasi knew which customers were late, the moment they were late",
    saw: ["EU-A failed: errors and the queue rose, then Grafana alerted",
      "Each support request was counted as overdue the moment it passed its 15-second deadline, with no new event"],
    sources: "Failed attempts, and EU-A's health reports turning to Failed",
    queries: "Overdue support requests: time-aware, so a request becomes late as time passes",
    models: "Plan: the current routes no longer work, so a new plan is proposed",
    reactions: "Support overdue, Deployment status and Action needed: Approve proposed plan in Grafana, and the proposed plan as a decision event",
    why: "Nothing re-scans for late requests: Drasi re-checks each one at its deadline.",
  },
  {
    id: "drasi-policy", after: "policy", title: "Change the rules and the answer changes",
    saw: ["Summaries were restricted to Europe: with EU-A down, no plan fits the rules, and Drasi said so at once",
      "Approving the plan selected before the change was rejected",
      "Allowing the US again brought the valid plan straight back"],
    sources: "The policy change",
    queries: "Is the approval still valid? It no longer matched Drasi's current plan",
    models: "Policy re-evaluated, plan recomputed: no valid plan, so the earlier plan no longer matches the facts",
    reactions: "Policy tiles and Action needed updated; the out-of-date approval rejected, with the reason",
    why: "Every plan carries the facts it was based on, so an out-of-date approval can't be applied.",
  },
  {
    id: "drasi-recovery", after: "recovery", title: "A fix within policy, and proof it worked",
    saw: ["Support can't use US-C; summaries can", "One approval moved summaries to US-C and support to EU-B",
      "Recovery showed Verified only after the queue cleared and support requests were answered on the new route"],
    sources: "Health reports, capacity and policy; then your approval and the answered requests",
    queries: "Verified recovery: queue cleared, and at least three support requests answered on the new route",
    models: "Policy: support may only run in Europe. Plan: the complete plan that fits",
    reactions: "The approved plan applied through the gateway; then Recovery: Verified in Grafana and Langfuse",
    why: "Approving a change isn't the same as fixing the problem. Drasi checks the outcome.",
  },
  {
    id: "drasi-resilience", after: "resilience", title: "Healthy now, but not safe from the next failure",
    saw: ["Everything green, but EU-A came back with only 50k of capacity",
      "Safe if one fails? No: not if EU-B fails. Then Yes once EU-A was back to 100k"],
    sources: "The capacity change on EU-A",
    queries: "Deployment capacity and health, the what-if's input",
    models: "What-if: for each deployment, if it failed, would every workload still fit somewhere allowed?",
    reactions: "The Safe if one fails? tile, and Action needed: At risk",
    why: "Drasi answers 'are we safe?' continuously, before the outage happens.",
  },
] as const satisfies readonly Recap[];
export const allSlides = [...slides, ...recaps.map(recap => ({ id: recap.id, title: recap.title }))] as const;
export type SlideId = typeof slides[number]["id"] | typeof recaps[number]["id"];

// What the projected stage shows: a slide, the dashboard, the incident channel, or one late request.
export type StageView = SlideId | "ops" | "incident" | "request";
export const viewLabel = (view: StageView) => view === "ops" ? "Grafana · Operations"
  : view === "incident" ? "Mattermost · #incident-ai-support" : view === "request" ? "One late request"
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

// The recap for how far the live demo has got, worked out from the scenario's current state in the
// AI service (failed deployments, the policy edit, applied routes, reduced capacity). Used when the
// projected window is navigated by hand (?slide=recap).
type ScenarioState = {
  deployments: readonly { id: string; mode: string; capacity_tpm: number }[];
  workloads: readonly { id: string; deployment: string }[];
  destinations: Record<string, readonly string[]>;
};
export function recapFromState(state: ScenarioState) {
  const order = ["drasi-healthy", "drasi-failure", "drasi-policy", "drasi-recovery", "drasi-resilience"] as const;
  let step = 0;
  if (state.deployments.some(d => d.mode !== "ok")) step = 1;
  if (!(state.destinations.batch ?? []).includes("US-C")) step = 2;
  if (state.workloads.some(w => w.id === "support" && w.deployment !== "EU-A")) step = 3;
  if (state.deployments.some(d => d.capacity_tpm < 100000)) step = 4;
  return recaps.find(recap => recap.id === order[step])!;
}
