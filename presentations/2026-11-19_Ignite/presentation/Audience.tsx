import { useEffect, useRef, useState } from "react";
import Reveal from "reveal.js";
import "reveal.js/dist/reveal.css";
import { allSlides, dashboards, scenes, type StageView } from "./scenes";
import { notes } from "./notes";
import { RequestView } from "./RequestView";
import { replayClips } from "./replay";
import { Explanation } from "./Explanation";
import type { StageControl } from "../demo/ui/src/stage";

// Dashboards are drawn at 1/zoom of the window and scaled up, so they read from the back of the room.
const zoom = Number(new URLSearchParams(location.search).get("zoom")) || 1.5;
const forwarded = [" ", "PageDown", "PageUp", "ArrowRight", "ArrowLeft", "ArrowDown", "ArrowUp", "Home", "End", "Enter", "Backspace", "b", "."];
// Without a desk, keys step through what the talk shows, in order (the late request needs the desk).
const sequence = scenes.flatMap(scene => notes[scene.id].beats.map(beat => beat.show))
  .filter((view, i, all): view is StageView => view !== "request" && view !== all[i - 1]);

// The projected stage. The presenter desk decides what it shows; keys pressed here
// (a clicker, or arrows) are passed to the desk.
export function Audience({ control, preview }: { control: StageControl; preview: boolean }) {
  const element = useRef<HTMLDivElement>(null);
  const deck = useRef<Reveal.Api | null>(null);
  const current = useRef(control);
  current.current = control;
  const { view, replay: replayId, trace } = control.stage;
  const index = allSlides.findIndex(slide => slide.id === view);
  const dashboard = view === "ops" || view === "plan" ? view : null;
  const replay = replayClips.find(clip => clip.scene === replayId);
  // Dashboards stay loaded once shown, so switching back is instant.
  const [loaded, setLoaded] = useState<Set<string>>(new Set());
  useEffect(() => { if (dashboard && !loaded.has(dashboard)) setLoaded(new Set([...loaded, dashboard])); }, [dashboard, loaded]);
  useEffect(() => {
    if (!element.current) return;
    const reveal = new Reveal(element.current, {
      embedded: true, hash: false, controls: false, progress: false, center: false,
      width: 1280, height: 720, margin: 0, transition: "none", overview: false,
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
    <div className="reveal" ref={element} hidden={!!replay || index < 0}>
      <div className="slides">{allSlides.map(slide => <section key={slide.id} data-slide={slide.id}><Explanation slide={slide.id} active={slide.id === view} /></section>)}</div>
    </div>
    {(["ops", "plan"] as const).filter(id => loaded.has(id)).map(id =>
      <iframe key={id} className="stage-dashboard" title={id === "ops" ? "Grafana · Operations" : "Grafana · Recovery plan"} scrolling="no"
        src={dashboards[id]} aria-hidden={!!replay || dashboard !== id}
        // Hidden with visibility, not display:none: after display:none Grafana's annotation list
        // ("Decision events") stops refreshing.
        style={{ width: `${100 / zoom}vw`, height: `${100 / zoom}vh`, transform: `scale(${zoom})`,
          visibility: !!replay || dashboard !== id ? "hidden" : "visible" }} />)}
    {view === "request" && !replay && <section className="stage-request"><RequestView taskId={trace} /></section>}
    {replay && <section className="replay-screen">
      <strong className="replay-label">REPLAY · NOT LIVE</strong><h1>{replay.title}</h1>
      {replay.src ? <video controls autoPlay src={replay.src} /> : <p>No current recording is available.</p>}
      <p>{replay.provenance}</p>
    </section>}
  </main>;
}
