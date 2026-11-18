import { settings } from "../demo/ui/src/settings";

// Diagnostics views (demo/ui/src/views.tsx), not part of the talk.
export type View = "operations" | "decisions" | "change" | "trace";

// Kiosk mode hides Grafana's menus; the _dash options hide the time picker and links too.
const grafana = (uid: string, kiosk = false) =>
  `${settings.grafana}/d/${uid}?from=now-5m&to=now&refresh=5s${kiosk ? "&kiosk&_dash.hideTimePicker&_dash.hideLinks&_dash.hideVariables" : ""}`;
export const products = {
  grafana: grafana("ai-operations"),
  decisions: grafana("ignite-decisions"),
  langfuse: `${settings.langfuse}/project/ignite-probe/traces?filter=isRootObservation%3Bboolean%3B%3B%3D%3Btrue`,
  server: `${settings.server}/ui/?instance=ignite`,
  drasi: "https://drasi.io/",
};
// Dashboards as projected: kiosk mode (no Grafana toolbar), embedded in the stage.
export const dashboards = { ops: grafana("ai-operations", true) } as const;
// The incident channel as projected (contracts.md §17); the projector's browser is signed in as oncall.
export const incidentChannel = `${settings.mattermost}/ai-platform/channels/incident-ai-support`;

// Shown on the title slide when set (bold name, then role, as in the Ignite template).
export const speaker = { name: "Allen Jones", role: "" };
export const session = { event: "Microsoft Ignite 2026", code: "THR1029" };

// The Ignite template's 16:9 slide, in pixels. Projected content stays above captionTop: the
// bottom 78 px are kept clear for closed captions, on slides and on the projected dashboards.
export const frame = { width: 1920, height: 1080, captionTop: 1002 } as const;

export const slides = [
  { id: "walkin", title: "Walk-in" },
  { id: "title", title: "Title" },
  // The scenario reveals its three sections one desk beat at a time.
  // Observability today reveals its five challenges in three desk beats.
  { id: "ecosystem", title: "Observability today · fragmented, noisy" },
  { id: "ecosystem-2", title: "Observability today · symptoms, health" },
  { id: "ecosystem-3", title: "Observability today · context elsewhere" },
  { id: "answers", title: "What would help" },
  { id: "drasi", title: "Drasi: change-driven computation" },
  { id: "scenario", title: "A typical scenario" },
  { id: "concepts", title: "The scenario · deployments" },
  { id: "concepts-rules", title: "The scenario · rules" },
  { id: "concepts-questions", title: "The scenario · three answers" },
  { id: "setup", title: "Bringing it together when EU-A fails" },
  { id: "query", title: "Who is affected: the continuous query" },
  { id: "closing", title: "Close" },
  { id: "survey", title: "Session survey" },
] as const;

// "What Drasi just did" visual, shown after Healthy but fragile (the other parts say their points instead).
// Roles left out did not take part in that step and are shown dimmed.
export type Recap = {
  id: string; after: string; title: string; saw: string[];
  sources?: string; queries?: string; models?: string; reactions?: string; why: string;
};
export const recaps = [
  {
    id: "drasi-resilience", after: "resilience", title: "Healthy now, but not safe from the next failure",
    saw: ["Everything green, but EU-A came back with only 50k of capacity",
      "Failover readiness: At risk, losing EU-B leaves support nowhere to run. Then it survives losing any one deployment once EU-A was back to 100k"],
    sources: "The capacity change on EU-A",
    queries: "Deployment capacity and health, the what-if's input",
    models: "What-if: for each deployment, if it failed, would every workload still fit somewhere allowed?",
    reactions: "Failover readiness and Recommended action (Add capacity) in Grafana, and a warning in #incident-ai-support",
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
  { id: "drasi", title: "Observability and Drasi", start: .5, minutes: 1.5 },
  { id: "concepts", title: "The scenario", start: 2, minutes: 1.5 },
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

// The recap shown when the projected window is navigated by hand (?slide=recap): the only one,
// Healthy but fragile's. The scenario state is accepted for the caller's sake.
export function recapFromState(_state: unknown) {
  return recaps[0];
}
