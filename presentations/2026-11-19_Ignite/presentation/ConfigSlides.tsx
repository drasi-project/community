import { useLayoutEffect, useRef, useState } from "react";
import overdueQuery from "../demo/drasi/overdue-tasks.cypher?raw";
import healthQuery from "../demo/drasi/deployment-health.cypher?raw";
import workloadsQuery from "../demo/drasi/tile-workloads.cypher?raw";
import rego from "../demo/drasi/policy/placement.rego?raw";
import optimization from "../demo/drasi/optimization/optimization.json";
import { buildConfig, type ConfigId } from "./codeSlides";
import type { Rect } from "./panels";
import { how, isHow, type How, type Part } from "./howItWorks";
import { Icon } from "./Icon";
import { DrasiMark } from "./Visuals";

export const configPanels = buildConfig({ overdueQuery, healthQuery, workloadsQuery, rego, optimization });

// The configuration behind the highlighted dashboard panel, placed just above that panel (as near
// its centre as the margins allow), with a short arrow down to it. Its inputs, grouped by where
// they come from, feed the query or model's real code.
const margin = 92, gap = 56, top = 20;
const columns = [["sources", "Sources", "blue"], ["drasi", "Drasi", "purple"], ["reactions", "Reactions", "lime"]] as const;
function HowCard({ card }: { card: How }) {
  const part = (p: Part) => <li key={p.label} className={`part ${p.kind}`}><Icon name={p.kind} /><b>{p.label}</b>{p.detail && <small>{p.detail}</small>}</li>;
  return <div className="how-card">
    <p className="how-title">How it works</p>
    <ol className="how-flow">
      {columns.map(([key, label, tone]) => <li key={key} className={`how-col ${tone}`}><h3 className={key === "drasi" ? "drasi" : undefined}>{key === "drasi" && <DrasiMark size={26} />}{label}</h3><ul>{card[key].map(part)}</ul></li>)}
      <li className="how-col lands"><h3>Shows up as</h3><p>{card.lands}</p></li>
    </ol>
  </div>;
}

// With targets, the card sits above them with an arrow to each; with none (over the channel) it's pinned top right.
export function ConfigOverlay({ ids, targets, pinned = false }: { ids: readonly string[]; targets: readonly Rect[]; pinned?: boolean }) {
  const target = targets.length ? { x: Math.min(...targets.map(t => t.x)), y: Math.min(...targets.map(t => t.y)),
    width: Math.max(...targets.map(t => t.x + t.width)) - Math.min(...targets.map(t => t.x)), height: 0, key: "" } : null;
  const card = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const key = ids.join(",");
  useLayoutEffect(() => {
    const element = card.current;
    setSize(element ? { width: element.offsetWidth, height: element.offsetHeight } : null);
  }, [key]);
  const panels = ids.filter(id => !isHow(id)).map(id => configPanels[id as ConfigId]).filter(Boolean);
  const cards = ids.filter(isHow).map(id => how[id]);
  if (!panels.length && !cards.length) return null;
  if (pinned || !target) return <div ref={card} className="config-overlay pinned">{cards.map((c, i) => <HowCard key={i} card={c} />)}</div>;
  const centre = target.x + target.width / 2;
  const left = size ? Math.max(margin, Math.min(1920 - margin - size.width, centre - size.width / 2)) : margin;
  const cardTop = size ? Math.max(top, target.y - gap - size.height) : top;
  const bottom = size ? cardTop + size.height : 0;
  // A panel stacked under another highlighted one gets no arrow of its own (it would cross the upper one).
  const pointed = targets.filter(t => !targets.some(u => u.y < t.y && u.x < t.x + t.width && t.x < u.x + u.width));
  const tips = pointed.map(t => { const c = t.x + t.width / 2;
    return Math.max(t.x + 40, Math.min(t.x + t.width - 40, size ? Math.max(left + 60, Math.min(left + size.width - 60, c)) : c)); });
  const tipX = tips[0], tipY = target.y - 12;
  return <>
    <div ref={card} className="config-overlay" style={{ left, top: cardTop, visibility: size ? "visible" : "hidden" }}>{cards.map((c, i) => <HowCard key={i} card={c} />)}{panels.map(panel =>
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
      {pointed.map((t, i) => <path key={t.key} d={`M${tips[i]},${bottom} L${tips[i]},${t.y - 12}`} markerEnd="url(#config-arrowhead)" />)}
    </svg>}
  </>;
}
