import { useEffect, useRef, useState } from "react";
import Reveal from "reveal.js";
import "reveal.js/dist/reveal.css";
import { allSlides, dashboards, frame, scenes, type StageView } from "./scenes";
import { notes } from "./notes";
import { RequestView } from "./RequestView";
import { replayClips } from "./replay";
import { Explanation } from "./Explanation";
import type { StageControl } from "../demo/ui/src/stage";

// Dashboards are drawn at 1/zoom of the stage and scaled up, so they read from the back of the room.
// At 1.35 both dashboards (about 740 px tall in Grafana) fit above the caption line.
const zoom = Number(new URLSearchParams(location.search).get("zoom")) || 1.35;
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

// The projected stage. The presenter desk decides what it shows; keys pressed here
// (a clicker, or arrows) are passed to the desk.
export function Audience({ control, preview }: { control: StageControl; preview: boolean }) {
  const element = useRef<HTMLDivElement>(null);
  const deck = useRef<Reveal.Api | null>(null);
  const current = useRef(control);
  current.current = control;
  const { view, replay: replayId, trace } = control.stage;
  const index = allSlides.findIndex(slide => slide.id === view);
  const scale = useFit();
  const dashboard = view === "ops" || view === "plan" ? view : null;
  const replay = replayClips.find(clip => clip.scene === replayId);
  // Dashboards stay loaded once shown, so switching back is instant.
  const [loaded, setLoaded] = useState<Set<string>>(new Set());
  useEffect(() => { if (dashboard && !loaded.has(dashboard)) setLoaded(new Set([...loaded, dashboard])); }, [dashboard, loaded]);
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
      {(["ops", "plan"] as const).filter(id => loaded.has(id)).map(id =>
        <iframe key={id} className="stage-dashboard" title={id === "ops" ? "Grafana · Operations" : "Grafana · Recovery plan"} scrolling="no"
          src={dashboards[id]} aria-hidden={!!replay || dashboard !== id}
          // Hidden with visibility, not display:none: after display:none Grafana's annotation list
          // ("Decision events") stops refreshing. It ends at the caption line.
          style={{ width: frame.width / zoom, height: frame.captionTop / zoom, transform: `scale(${zoom})`,
            visibility: !!replay || dashboard !== id ? "hidden" : "visible" }} />)}
      {dashboard && !replay && <div className="caption-zone" />}
      {view === "request" && !replay && <section className="stage-request"><RequestView taskId={trace} /></section>}
      {replay && <section className="replay-screen">
        <header><strong className="replay-label">REPLAY · NOT LIVE</strong><h1>{replay.title}</h1></header>
        <p>{replay.provenance}</p>
        {replay.src ? <video controls autoPlay src={replay.src} /> : <p className="replay-missing">No current recording is available.</p>}
      </section>}
    </div>
  </main>;
}
