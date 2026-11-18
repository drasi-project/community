import { frame } from "./scenes";
import { panelRects, type Dashboard } from "./panels";

// The dashboards as generated (demo/observability/grafana/dashboards.py), for their panels' gridPos.
const files = import.meta.glob("../demo/observability/grafana/dashboards/*.json", { eager: true, import: "default" });
const byUid = new Map(Object.values(files).map(value => [(value as Dashboard).uid, value as Dashboard]));

// Dims the rest of the projected dashboard and outlines the panels being talked about.
export function PanelHighlight({ src, keys, zoom }: { src: string; keys: readonly string[]; zoom: number }) {
  const dashboard = byUid.get(/\/d\/([^/?]+)/.exec(src)?.[1] ?? "");
  const rects = dashboard ? panelRects(dashboard, keys, frame, zoom) : [];
  if (!rects.length) return null;
  const { width, captionTop: height } = frame;
  const pad = 6;
  return <svg className="panel-highlight" width={width} height={height} viewBox={`0 0 ${width} ${height}`} aria-hidden="true">
    <defs><mask id="panel-highlight-mask"><rect width={width} height={height} fill="white" />
      {rects.map(r => <rect key={r.key} x={r.x - pad} y={r.y - pad} width={r.width + 2 * pad} height={r.height + 2 * pad} rx={10} fill="black" />)}</mask></defs>
    <rect className="dim" width={width} height={height} mask="url(#panel-highlight-mask)" />
    {rects.map(r => <rect key={r.key} className="outline" x={r.x - pad} y={r.y - pad} width={r.width + 2 * pad} height={Math.min(r.height + 2 * pad, height - r.y + pad - 2)} rx={10} />)}
  </svg>;
}
