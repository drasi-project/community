import { useLayoutEffect, useRef, useState } from "react";
import overdueQuery from "../demo/drasi/overdue-tasks.cypher?raw";
import healthQuery from "../demo/drasi/deployment-health.cypher?raw";
import workloadsQuery from "../demo/drasi/tile-workloads.cypher?raw";
import rego from "../demo/drasi/policy/ignite.rego?raw";
import optimization from "../demo/drasi/optimization/optimization.json";
import { buildConfig, type ConfigId } from "./codeSlides";
import type { Rect } from "./panels";

export const configPanels = buildConfig({ overdueQuery, healthQuery, workloadsQuery, rego, optimization });

// The configuration behind the highlighted dashboard panel, on the dimmed rows above it: its inputs
// across the top, feeding the query or model's real code, and an arrow from the card down to the
// panel it populates (target: the highlighted panel on the stage).
export function ConfigOverlay({ ids, target }: { ids: readonly ConfigId[]; target: Rect | null }) {
  const card = useRef<HTMLDivElement>(null);
  const [box, setBox] = useState<{ left: number; right: number; bottom: number } | null>(null);
  const key = ids.join(",");
  useLayoutEffect(() => {
    const element = card.current;
    setBox(element ? { left: element.offsetLeft, right: element.offsetLeft + element.offsetWidth, bottom: element.offsetTop + element.offsetHeight } : null);
  }, [key]);
  const panels = ids.map(id => configPanels[id]).filter(Boolean);
  if (!panels.length) return null;
  const flow = panels[panels.length - 1].flow;
  // The arrow leaves the card above the panel's centre (or the nearest point of the card) and
  // stops just above the panel's title.
  const tipX = target ? target.x + Math.min(target.width / 2, 260) : 0, tipY = target ? target.y - 14 : 0;
  const startX = box ? Math.max(box.left + 60, Math.min(box.right - 60, tipX)) : 0;
  return <>
    <div ref={card} className="config-overlay">{panels.map(panel =>
      <div key={panel.tab} className="code-panel">
        <ul className="flow-inputs" aria-label="Inputs">{panel.flow.inputs.map(input =>
          <li key={input.name} className={input.from === "OpenTelemetry" ? "otel" : input.from === "PostgreSQL" ? "pg" : "model"}>
            <b>{input.name}</b><small>{input.from}</small></li>)}</ul>
        <p className="flow-into"><span aria-hidden="true">↓</span> {panel.flow.into}</p>
        <p className="code-tab">{panel.tab}</p>
        <pre aria-label={panel.tab}>{panel.lines.map((line, i) =>
          <span key={i} className={panel.hot.some(text => line.includes(text)) ? "hot" : undefined}><i aria-hidden="true" />{line}{"\n"}</span>)}</pre>
      </div>)}</div>
    {box && target && <svg className="config-arrow" width="1920" height="1002" viewBox="0 0 1920 1002" aria-hidden="true"
      data-start={`${startX},${box.bottom}`} data-tip={`${tipX},${tipY}`}>
      <defs><marker id="config-arrowhead" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto">
        <path d="M0,0 L10,5 L0,10 z" /></marker></defs>
      <path d={`M${startX},${box.bottom} L${tipX},${tipY}`} markerEnd="url(#config-arrowhead)" />
      {flow.after.length > 0 && <text x={Math.max(startX, tipX) + 18} y={(box.bottom + tipY) / 2 + 8}>{flow.after.join(" → ")}</text>}
    </svg>}
  </>;
}
