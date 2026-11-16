import { useEffect, useRef } from "react";
import Reveal from "reveal.js";
import "reveal.js/dist/reveal.css";
import { products, slides } from "./scenes";
import { replayClips } from "./replay";
import { Explanation } from "./Explanation";
import type { StageControl } from "../demo/ui/src/stage";

export function Audience({ control, preview }: { control: StageControl; preview: boolean }) {
  const element = useRef<HTMLDivElement>(null);
  const deck = useRef<Reveal.Api | null>(null);
  const current = useRef(control);
  current.current = control;
  const index = slides.findIndex(slide => slide.id === control.stage.slide);
  const replay = replayClips.find(clip => clip.scene === control.stage.replay);
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
      reveal.slide(slides.findIndex(s => s.id === current.current.stage.slide));
    });
    const keyboard = (event: KeyboardEvent) => {
      if (current.current.stage.replay) return;
      if (event.target instanceof HTMLElement &&
          (event.target.closest("button,a,input,textarea,select") || event.target.isContentEditable)) return;
      if ([" ", "PageDown", "ArrowRight"].includes(event.key)) {
        event.preventDefault();
        if (current.current.stage.slide === "title") current.current.showSlide("concepts");
        else if (current.current.stage.slide === "concepts") window.location.assign(products.grafana);
      }
      if (["PageUp", "ArrowLeft"].includes(event.key)) {
        event.preventDefault();
        current.current.showSlide(current.current.stage.slide === "closing" ? "concepts" : "title");
      }
    };
    window.addEventListener("keydown", keyboard);
    return () => { active = false; window.removeEventListener("keydown", keyboard); reveal.destroy(); deck.current = null; };
  }, []);
  useEffect(() => { deck.current?.slide(index); }, [index]);
  return <main className="slide-deck">
    {preview && <span className="slide-preview">Read-only preview</span>}
    <div className="reveal" ref={element} hidden={!!replay}>
      <div className="slides">{slides.map(slide => <section key={slide.id} data-slide={slide.id}><Explanation slide={slide.id} /></section>)}</div>
    </div>
    {!replay && <nav className="slide-navigation" aria-label="Slide navigation">
      {index > 0 && <button aria-label="Previous slide" onClick={() => control.showSlide(index === 2 ? "concepts" : "title")}>←</button>}
      {index === 0 && <button aria-label="Next slide" onClick={() => control.showSlide("concepts")}>→</button>}
      {index === 1 && <a className="button" href={products.grafana}>Live demo · Grafana ↗</a>}
      {index === 2 && <a href={products.grafana}>Grafana ↗</a>}
    </nav>}
    {replay && <section className="replay-screen">
      <strong className="replay-label">REPLAY · NOT LIVE</strong><h1>{replay.title}</h1>
      {replay.src ? <video controls src={replay.src} /> : <p>No current recording is available.</p>}
      <p>{replay.provenance}</p>
    </section>}
  </main>;
}
