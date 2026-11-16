export type View = "operations" | "decisions" | "change" | "trace";
export const products = {
  grafana: "http://localhost:13000/d/ignite-workload?from=now-5m&to=now&refresh=5s",
  decisions: "http://localhost:13000/d/ignite-decisions?from=now-5m&to=now&refresh=5s",
  langfuse: "http://localhost:13001/project/ignite-probe/traces",
  server: "http://127.0.0.1:18421/ui/?instance=ignite",
  repository: "https://github.com/drasi-project/community/tree/agentofreality-ignite/presentations/2026-11-19_Ignite",
};

export const slides = [
  { id: "title", title: "Monitoring without polling" },
  { id: "concepts", title: "The scenario" },
  { id: "closing", title: "From change to action" },
] as const;
export type SlideId = typeof slides[number]["id"];

// These are private speaking cues, not additional projected slides.
export const scenes = [
  { id: "title", title: "Title", start: 0, minutes: .5, tool: "Title slide", url: "/present?slide=title" },
  { id: "concepts", title: "The scenario", start: .5, minutes: 2, tool: "Scenario slide", url: "/present?slide=concepts" },
  { id: "normal", title: "Healthy service", start: 2.5, minutes: 2.5, tool: "Grafana · Operations", url: products.grafana },
  { id: "failure", title: "Failure and customer impact", start: 5, minutes: 3, tool: "Grafana · Operations", url: products.grafana },
  { id: "recovery", title: "Recovery", start: 8, minutes: 5, tool: "Grafana · Policy and recovery", url: products.decisions },
  { id: "change", title: "The continuous query", start: 13, minutes: 2, tool: "Drasi Server", url: products.server },
  { id: "policy", title: "Policy changes the answer", start: 15, minutes: 2, tool: "Grafana · Policy and recovery", url: products.decisions },
  { id: "resilience", title: "Healthy but fragile", start: 17, minutes: 3.5, tool: "Grafana · Policy and recovery", url: products.decisions },
  { id: "closing", title: "Close", start: 20.5, minutes: 1.5, tool: "Closing slide", url: "/present?slide=closing" },
] as const;
export type SceneId = typeof scenes[number]["id"];
export const shortRoute: readonly SceneId[] = ["title", "concepts", "normal", "failure", "recovery", "change", "resilience", "closing"];
export const timeLabel = (minutes: number) => `${Math.floor(minutes)}:${Math.round((minutes % 1) * 60).toString().padStart(2, "0")}`;
export const duration = (minutes: number) => minutes < 1 ? `${Math.round(minutes * 60)} seconds`
  : `${Math.floor(minutes)}${minutes % 1 ? "½" : ""} minute${minutes > 1 ? "s" : ""}`;
