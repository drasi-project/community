import { useLayoutEffect, useRef, useState } from "react";
import overdueQuery from "../demo/drasi/overdue-tasks.cypher?raw";
import healthQuery from "../demo/drasi/deployment-health.cypher?raw";
import workloadsQuery from "../demo/drasi/tile-workloads.cypher?raw";
import rego from "../demo/drasi/policy/placement.rego?raw";
import optimization from "../demo/drasi/optimization/optimization.json";
import { buildConfig, type ConfigId } from "./codeSlides";
import type { Rect } from "./panels";
import { how, isHow, isLabel, type How, type Flow } from "./howItWorks";
import { Icon } from "./Icon";
import { DrasiMark } from "./Visuals";

export const configPanels = buildConfig({ overdueQuery, healthQuery, workloadsQuery, rego, optimization });

// The configuration behind the highlighted dashboard panel, placed just above that panel (as near
// its centre as the margins allow), with a short arrow down to it. Its inputs, grouped by where
// they come from, feed the query or model's real code.
const margin = 92, gap = 56, top = 20;
// The flow as a small graph, in Drasi's role colours (the same as the slides).
const colX = [0, 350, 700, 1050], nodeW = 310, nodeH = 64, rowH = 84, headH = 34;
const heads = ["Sources", "Continuous Queries and Models", "Reactions", "Shows up in"];
function FlowGraph({ flow }: { flow: Flow }) {
  const rows = Math.max(...flow.nodes.map(node => node.row)) + 1;
  const height = headH + rows * rowH - (rowH - nodeH);
  const at = (id: string) => flow.nodes.find(node => node.id === id)!;
  const y = (row: number) => headH + row * rowH;
  return <svg className="how-graph" viewBox={`0 0 ${colX[3] + nodeW} ${height}`} width={colX[3] + nodeW} height={height} role="img" aria-label="How it works">
    <defs><marker id="how-head" viewBox="0 0 10 10" refX="10" refY="5" markerUnits="userSpaceOnUse" markerWidth="14" markerHeight="14" orient="auto"><path d="M0,0 L10,5 L0,10 z" /></marker></defs>
    {heads.map((h, i) => <text key={h} className="how-head" x={colX[i]} y="22">{h}</text>)}
    {flow.edges.map(([a, b]) => { const f = at(a), t = at(b);
      if (f.col === t.col) { const x = colX[f.col] + nodeW / 2;
        return <path key={a + b} d={`M${x},${y(f.row) + nodeH} L${x},${y(t.row) - 1}`} markerEnd="url(#how-head)" />; }
      const x1 = colX[f.col] + nodeW, y1 = y(f.row) + nodeH / 2, x2 = colX[t.col] - 1, y2 = y(t.row) + nodeH / 2, bend = (x2 - x1) / 2;
      return <path key={a + b} d={`M${x1},${y1} C${x1 + bend},${y1} ${x2 - bend},${y2} ${x2},${y2}`} markerEnd="url(#how-head)" />; })}
    {flow.nodes.map(node => <g key={node.id} className={`how-node role-${node.role}`}>
      <rect x={colX[node.col]} y={y(node.row)} width={nodeW} height={nodeH} rx="8" />
      <text className="how-label" x={colX[node.col] + 14} y={y(node.row) + (node.detail || node.component ? 27 : 39)}>{node.label}</text>
      {node.component ? <text className="how-detail how-component" x={colX[node.col] + 14} y={y(node.row) + 51}>{node.component}</text>
        : node.detail && <text className="how-detail" x={colX[node.col] + 14} y={y(node.row) + 51}>{node.detail}</text>}
    </g>)}
  </svg>;
}

function HowCard({ card }: { card: How }) {
  if (isLabel(card)) return <div className={`how-card how-label${card.drasi ? " drasi" : ""}`}>
    <p className="how-title">{card.label}</p>
    {card.chips && <ol className="label-chips">{card.chips.map(c => <li key={c.label}><Icon name={c.icon} />{c.label}</li>)}</ol>}
    {card.text && <p className="label-text">{card.text}</p>}
    {card.drasi && <p className="label-powered"><DrasiMark />Powered by Drasi</p>}
  </div>;
  return <div className="how-card"><p className="how-title">How it works</p><FlowGraph flow={card} /></div>;
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
  // Above the panels if it fits, otherwise just below them if it fits there (a whole top row), else at the top.
  const lowest = Math.max(...targets.map(t => t.y + t.height));
  const below = !!size && target.y - gap - size.height < top && lowest + gap + size.height <= 1002 - 16;
  const cardTop = !size ? top : below ? lowest + gap : Math.max(top, target.y - gap - size.height);
  const bottom = size ? cardTop + size.height : 0;
  const start = below ? cardTop : bottom;
  // A panel stacked under another highlighted one gets no arrow of its own (it would cross the upper one).
  const pointed = targets.filter(t => !targets.some(u => u.y < t.y && u.x < t.x + t.width && t.x < u.x + u.width));
  const tips = pointed.map(t => { const c = t.x + t.width / 2;
    return Math.max(t.x + 40, Math.min(t.x + t.width - 40, size ? Math.max(left + 60, Math.min(left + size.width - 60, c)) : c)); });
  const tipFor = (t: Rect) => below ? t.y + t.height + 12 : t.y - 12;
  const tipX = tips[0], tipY = pointed.length ? tipFor(pointed[0]) : 0;
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
      data-start={`${tipX},${start}`} data-tip={`${tipX},${tipY}`}>
      <defs><marker id="config-arrowhead" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto">
        <path d="M0,0 L10,5 L0,10 z" /></marker></defs>
      {pointed.map((t, i) => <path key={t.key} d={`M${tips[i]},${start} L${tips[i]},${tipFor(t)}`} markerEnd="url(#config-arrowhead)" />)}
    </svg>}
  </>;
}
