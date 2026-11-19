import { useEffect, useRef, useState } from "react";
import Reveal from "reveal.js";
import "reveal.js/dist/reveal.css";
import { allSlides, dashboards, frame, incidentChannel, scenes, type StageView } from "./scenes";
import { notes } from "./notes";
import { RequestView } from "./RequestView";
import { replayClips } from "./replay";
import { Explanation } from "./Explanation";
import { PanelHighlight, highlightRects } from "./Highlight";
import { ConfigOverlay } from "./ConfigSlides";
import type { StageControl } from "../demo/ui/src/stage";

// Dashboards are drawn at 1/zoom of the stage and scaled up, so they read from the back of the room.
// At 1.35 both dashboards (about 740 px tall in Grafana) fit above the caption line.
const zoom = Number(new URLSearchParams(location.search).get("zoom")) || 1.35;
// The incident channel is scaled up more: at 1.5 Mattermost lays out 1280 x 668 px (its desktop
// layout, above 768 px), so a whole proposal card fits and the cards read from the back of the room.
const incidentZoom = Number(new URLSearchParams(location.search).get("incidentZoom")) || 1.5;
const forwarded = [" ", "PageDown", "PageUp", "ArrowRight", "ArrowLeft", "ArrowDown", "ArrowUp", "Home", "End", "Enter", "Backspace", "b", "."];
// Without a desk, keys step through what the talk shows, in order (the late request needs the desk).
const sequence = scenes.flatMap(scene => notes[scene.id].beats.map(beat => beat.show))
  .filter((view, i, all): view is StageView => view !== "request" && view !== all[i - 1]);

// The stage is always the template's 1920x1080 slide, scaled to fit the window: exactly 1:1 on a
// 1920x1080 projector at 100% scaling, letterboxed on any other shape.
function useFit() {
  const fit = () => Math.min(window.innerWidth / frame.width, window.innerHeight / frame.height);
  const [scale, setScale] = useState(fit);
  useEffect(() => {
    const resize = () => setScale(fit());
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  return scale;
}

// The demo run, for reloading the framed incident channel after a reset. A reset deletes the
// channel's posts through Mattermost's API, and an open Mattermost client keeps a "(message
// deleted)" placeholder for each until it reloads; so once the run changes, and the deletes have
// had time to finish, the frame loads the channel afresh.
function useChannelGeneration(active: boolean) {
  const [generation, setGeneration] = useState(0);
  useEffect(() => {
    if (!active) return;
    let epoch: string | null = null, timer: ReturnType<typeof setTimeout> | undefined, live = true;
    const check = async () => {
      try {
        const response = await fetch("/api/health", { cache: "no-store" });
        const next = response.ok ? String((await response.json()).run_epoch ?? "") : "";
        if (live && next && epoch && next !== epoch) {
          clearTimeout(timer);
          timer = setTimeout(() => setGeneration(n => n + 1), 4000);
        }
        if (next) epoch = next;
      } catch { /* the demo is down: keep the frame as it is */ }
    };
    void check();
    const interval = setInterval(check, 2000);
    return () => { live = false; clearInterval(interval); clearTimeout(timer); };
  }, [active]);
  return generation;
}

// The projected stage. The presenter desk decides what it shows; keys pressed here
// (a clicker, or arrows) are passed to the desk.
export function Audience({ control, preview }: { control: StageControl; preview: boolean }) {
  const element = useRef<HTMLDivElement>(null);
  const deck = useRef<Reveal.Api | null>(null);
  const current = useRef(control);
  current.current = control;
  const { view, replay: replayId, trace } = control.stage;
  // In a read-only preview, ?highlight=uid.panel,… shows a highlight without the desk.
  const highlight = preview ? new URLSearchParams(location.search).get("highlight")?.split(",") ?? [] : control.stage.highlight;
  const config = preview ? new URLSearchParams(location.search).get("config")?.split(",") ?? [] : control.stage.config;
  const index = allSlides.findIndex(slide => slide.id === view);
  const scale = useFit();
  const dashboard = view === "ops" ? view : null;
  const framed = dashboard ?? (view === "incident" ? view : null);
  const replay = replayClips.find(clip => clip.scene === replayId);
  // Dashboards and the channel stay loaded once shown, so switching back is instant.
  const [loaded, setLoaded] = useState<Set<string>>(new Set());
  useEffect(() => { if (framed && !loaded.has(framed)) setLoaded(new Set([...loaded, framed])); }, [framed, loaded]);
  const channelGeneration = useChannelGeneration(!preview && loaded.has("incident"));
  useEffect(() => {
    if (!element.current) return;
    const reveal = new Reveal(element.current, {
      embedded: true, hash: false, controls: false, progress: false, center: false,
      width: frame.width, height: frame.height, margin: 0, transition: "none", overview: false,
      keyboard: false, touch: false, autoSlide: 0, help: false, disableLayout: true,
    });
    let active = true;
    void reveal.initialize().then(() => {
      if (!active) return;
      deck.current = reveal;
      reveal.slide(Math.max(0, allSlides.findIndex(s => s.id === current.current.stage.view)));
    });
    const keyboard = (event: KeyboardEvent) => {
      if (!forwarded.includes(event.key)) return;
      event.preventDefault();
      current.current.sendKey(event.key, () => {
        const here = sequence.indexOf(current.current.stage.view);
        const back = ["PageUp", "ArrowLeft", "ArrowUp", "Backspace"].includes(event.key);
        const next = event.key === "Home" ? sequence[0] : sequence[Math.max(0, Math.min(sequence.length - 1, here + (back ? -1 : 1)))];
        if (next) current.current.show(next);
      });
    };
    window.addEventListener("keydown", keyboard);
    return () => { active = false; window.removeEventListener("keydown", keyboard); reveal.destroy(); deck.current = null; };
  }, []);
  useEffect(() => { if (index >= 0) deck.current?.slide(index); }, [index]);
  return <main className="slide-deck">
    {preview && <span className="slide-preview">Read-only preview</span>}
    <div className="stage-frame" style={{ width: frame.width, height: frame.height, transform: `translate(-50%, -50%) scale(${scale})` }}>
      <div className="reveal" ref={element} hidden={!!replay || index < 0}>
        <div className="slides">{allSlides.map(slide => <section key={slide.id} data-slide={slide.id}><Explanation slide={slide.id} active={slide.id === view} /></section>)}</div>
      </div>
      {(["ops"] as const).filter(id => loaded.has(id)).map(id =>
        <iframe key={id} className="stage-dashboard" title="Grafana · Operations" scrolling="no"
          src={dashboards[id]} aria-hidden={!!replay || dashboard !== id}
          // Hidden with visibility, not display:none: after display:none Grafana's annotation list
          // ("Decision events") stops refreshing. It ends at the caption line.
          style={{ width: frame.width / zoom, height: frame.captionTop / zoom, transform: `scale(${zoom})`,
            visibility: !!replay || dashboard !== id ? "hidden" : "visible" }} />)}
      {loaded.has("incident") && <iframe key={`incident-${channelGeneration}`} data-generation={channelGeneration} className="stage-dashboard stage-incident" title="Mattermost · #incident-ai-support"
        src={incidentChannel} aria-hidden={!!replay || view !== "incident"}
        // Signed in as oncall through the AI service's /mattermost/sign-in (contracts.md §17).
        style={{ width: frame.width / incidentZoom, height: frame.captionTop / incidentZoom, transform: `scale(${incidentZoom})`,
          visibility: !!replay || view !== "incident" ? "hidden" : "visible" }} />}
      {dashboard && !replay && <PanelHighlight src={dashboards[dashboard]} keys={highlight} zoom={zoom} />}
      {dashboard && !replay && <ConfigOverlay ids={config} targets={highlightRects(dashboards[dashboard], highlight, zoom)} />}
      {view === "incident" && !replay && <ConfigOverlay ids={config} targets={[]} pinned />}
      {framed && !replay && <div className="caption-zone" />}
      {view === "request" && !replay && <section className="stage-request"><RequestView taskId={trace} /></section>}
      {replay && <section className="replay-screen">
        <header><strong className="replay-label">REPLAY · NOT LIVE</strong><h1>{replay.title}</h1></header>
        <p>{replay.provenance}</p>
        {replay.src ? <video controls autoPlay src={replay.src} /> : <p className="replay-missing">No current recording is available.</p>}
      </section>}
    </div>
  </main>;
}
