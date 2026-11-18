import healthQuery from "../demo/drasi/deployment-health.cypher?raw";
import tileQuery from "../demo/drasi/tile-deployments.cypher?raw";
import rego from "../demo/drasi/policy/ignite.rego?raw";
import optimization from "../demo/drasi/optimization/optimization.json";
import { buildCodeSlides, type CodeSlideId } from "./codeSlides";

export const codeSlides = buildCodeSlides({ healthQuery, tileQuery, rego, optimization });

export function CodeSlide({ id }: { id: CodeSlideId }) {
  const slide = codeSlides[id];
  return <div className={`content-slide code-slide ${slide.wide ? "wide" : ""}`}>
    <h1>{slide.title}</h1>
    <div className="body">
      <div className="code-panels">{slide.panels.map(panel =>
        <div key={panel.tab} className="code-panel">
          <p className="code-tab">{panel.tab}</p>
          <pre aria-label={panel.tab}>{panel.lines.map((line, i) => {
            const mark = panel.marks.find(([text]) => line.includes(text))?.[1];
            return <span key={i}>{mark ? <i>{mark}</i> : <i aria-hidden="true" />}{line}{"\n"}</span>;
          })}</pre>
        </div>)}</div>
      <ol className="query-notes">{slide.notes.map(note => <li key={note}>{note}</li>)}</ol>
    </div>
  </div>;
}
