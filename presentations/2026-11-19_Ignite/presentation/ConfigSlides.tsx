import { useLayoutEffect, useRef, useState } from "react";
import overdueQuery from "../demo/drasi/overdue-tasks.cypher?raw";
import healthQuery from "../demo/drasi/deployment-health.cypher?raw";
import workloadsQuery from "../demo/drasi/tile-workloads.cypher?raw";
import rego from "../demo/drasi/policy/ignite.rego?raw";
import optimization from "../demo/drasi/optimization/optimization.json";
import { buildConfig, type ConfigId } from "./codeSlides";
import type { Rect } from "./panels";

export const configPanels = buildConfig({ overdueQuery, healthQuery, workloadsQuery, rego, optimization });

// The configuration behind the highlighted dashboard panel, placed just above that panel (as near
// its centre as the margins allow), with a short arrow down to it. Its inputs, grouped by where
// they come from, feed the query or model's real code.
const margin = 92, gap = 56, top = 20;
export function ConfigOverlay({ ids, target }: { ids: readonly ConfigId[]; target: Rect | null }) {
  const card = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const key = ids.join(",");
  useLayoutEffect(() => {
    const element = card.current;
    setSize(element ? { width: element.offsetWidth, height: element.offsetHeight } : null);
  }, [key]);
  const panels = ids.map(id => configPanels[id]).filter(Boolean);
  if (!panels.length || !target) return null;
  const centre = target.x + target.width / 2;
  const left = size ? Math.max(margin, Math.min(1920 - margin - size.width, centre - size.width / 2)) : margin;
  const cardTop = size ? Math.max(top, target.y - gap - size.height) : top;
  const bottom = size ? cardTop + size.height : 0;
  const tipX = Math.max(target.x + 40, Math.min(target.x + target.width - 40, size ? Math.max(left + 60, Math.min(left + size.width - 60, centre)) : centre));
  const tipY = target.y - 12;
  return <>
    <div ref={card} className="config-overlay" style={{ left, top: cardTop, visibility: size ? "visible" : "hidden" }}>{panels.map(panel =>
      <div key={panel.tab} className="code-panel">
        <ul className="flow-inputs" aria-label="Inputs">{panel.flow.inputs.map(input =>
          <li key={input.from} className={input.from === "OpenTelemetry" ? "otel" : input.from.startsWith("Drasi") ? "model" : "pg"}>
            <small>{input.from}</small><b>{input.items}</b></li>)}</ul>
        <p className="flow-into"><span aria-hidden="true">↓</span> {panel.flow.into}</p>
        <p className="code-tab">{panel.tab}</p>
        <pre aria-label={panel.tab}>{panel.lines.map((line, i) =>
          <span key={i} className={panel.hot.some(text => line.includes(text)) ? "hot" : undefined}><i aria-hidden="true" />{line}{"\n"}</span>)}</pre>
      </div>)}</div>
    {size && <svg className="config-arrow" width="1920" height="1002" viewBox="0 0 1920 1002" aria-hidden="true"
      data-start={`${tipX},${bottom}`} data-tip={`${tipX},${tipY}`}>
      <defs><marker id="config-arrowhead" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto">
        <path d="M0,0 L10,5 L0,10 z" /></marker></defs>
      <path d={`M${tipX},${bottom} L${tipX},${tipY}`} markerEnd="url(#config-arrowhead)" />
    </svg>}
  </>;
}
