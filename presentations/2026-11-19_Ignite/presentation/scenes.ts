import { settings } from "../demo/ui/src/settings";

// Diagnostics views (demo/ui/src/views.tsx), not part of the talk.
export type View = "operations" | "decisions" | "change" | "trace";

// Kiosk mode hides Grafana's menus; the _dash options hide the time picker and links too.
const grafana = (uid: string, kiosk = false) =>
  `${settings.grafana}/d/${uid}?from=now-5m&to=now&refresh=5s${kiosk ? "&kiosk&_dash.hideTimePicker&_dash.hideLinks&_dash.hideVariables" : ""}`;
export const products = {
  grafana: grafana("ai-operations"),
  langfuse: `${settings.langfuse}/project/ignite-probe/traces?filter=isRootObservation%3Bboolean%3B%3B%3D%3Btrue`,
  server: `${settings.server}/ui/?instance=ai-ops`,
  drasi: "https://drasi.io/",
};
// Dashboards as projected: kiosk mode (no Grafana toolbar), embedded in the stage.
export const dashboards = { ops: grafana("ai-operations", true) } as const;
// The incident channel as projected (contracts.md §17); the projector's browser is signed in as oncall.
export const incidentChannel = `${settings.mattermost}/ai-platform/channels/incident-ai-support`;

// Shown on the title slide when set (bold name, then role, as in the Ignite template).
export const speaker = { name: "Allen Jones", role: "Azure Incubations, Microsoft" }; // TODO: confirm role
export const session = { event: "Microsoft Ignite 2026", code: "THR1029" };

// The Ignite template's 16:9 slide, in pixels. Projected content stays above captionTop: the
// bottom 78 px are kept clear for closed captions, on slides and on the projected dashboards.
export const frame = { width: 1920, height: 1080, captionTop: 1002 } as const;

export const slides = [
  { id: "walkin", title: "Walk-in" },
  { id: "title", title: "Who I am" },
  { id: "project", title: "Drasi, the project" },
  { id: "problem", title: "The problem" },
  { id: "drasi-1", title: "Drasi · one source" },
  { id: "drasi-2", title: "Drasi · several sources" },
  { id: "drasi-3", title: "Drasi · many queries and reactions" },
  { id: "drasi-4", title: "Drasi · queries build on queries" },
  { id: "plugins", title: "Plugin ecosystem" },
  { id: "forms", title: "Form factors" },
  { id: "issues", title: "Observability issues" },
  { id: "wants", title: "What they want" },
  { id: "scenario-map", title: "The scenario" },
  { id: "closing", title: "What you saw" },
  { id: "links", title: "Learn more" },
  { id: "survey", title: "Session survey" },
] as const;

export const allSlides = slides;
export type SlideId = typeof slides[number]["id"];

// What the projected stage shows: a slide, the dashboard, the incident channel, or one late request.
export type StageView = SlideId | "ops" | "incident" | "request";
export const viewLabel = (view: StageView) => view === "ops" ? "Grafana · Operations"
  : view === "incident" ? "Mattermost · #incident-ai-support" : view === "request" ? "One late request"
  : slides.find(slide => slide.id === view)?.title ?? view;

// The talk's parts. Timings are minutes from the start.
export const scenes = [
  { id: "intro", title: "Intro", start: 0, minutes: 2 },
  { id: "drasi", title: "What Drasi is", start: 2, minutes: 3.5 },
  { id: "observability", title: "Observability and the scenario", start: 5.5, minutes: 2.5 },
  { id: "scene1", title: "Scene 1 · Status from telemetry and configuration", start: 8, minutes: 2.5 },
  { id: "scene2", title: "Scene 2 · The absence of change", start: 10.5, minutes: 2.5 },
  { id: "scene3", title: "Scene 3 · Compliance meets resilience", start: 13, minutes: 3 },
  { id: "scene4", title: "Scene 4 · Failure, proposal, approval", start: 16, minutes: 4.5 },
  { id: "closing", title: "Summary and close", start: 20.5, minutes: 1.5 },
] as const;
export type SceneId = typeof scenes[number]["id"];
export const timeLabel = (minutes: number) => `${Math.floor(minutes)}:${Math.round((minutes % 1) * 60).toString().padStart(2, "0")}`;
export const duration = (minutes: number) => minutes < 1 ? `${Math.round(minutes * 60)} seconds`
  : `${Math.floor(minutes)}${minutes % 1 ? "½" : ""} minute${minutes > 1 ? "s" : ""}`;
