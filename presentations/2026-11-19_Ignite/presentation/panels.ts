// Where a Grafana panel sits on the projected stage, worked out from the dashboard JSON's gridPos
// the way Grafana lays it out: 24 columns, 30 px rows, 8 px gaps, inside kiosk mode's 16 px padding,
// all drawn at 1/zoom of the stage and scaled up (Audience.tsx). Panels inside a row are indented
// 24 px and sit 2 px lower (Grafana 12's row layout), measured from the projected dashboards. Used to highlight the panels a beat
// talks about (a beat's `highlight` in notes.ts).
export type Panel = { id?: number; key?: string; type?: string; title?: string; gridPos: { x: number; y: number; w: number; h: number } };
export type Dashboard = { uid: string; panels: Panel[] };
export type Rect = { key: string; x: number; y: number; width: number; height: number };

const columns = 24, rowHeight = 30, gap = 8, padding = 16, rowIndent = 24, rowDrop = 2;

// A panel's key: its "key" in the dashboard JSON, else its title in lower case with dashes.
export const panelKey = (panel: Panel) => panel.key
  ?? (panel.title ?? "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");

// The panels a key names: the panel itself or, for a row, every panel up to the next row.
function panelsFor(dashboard: Dashboard, key: string) {
  const at = dashboard.panels.findIndex(panel => panelKey(panel) === key);
  if (at < 0) return [];
  const panel = dashboard.panels[at];
  if (panel.type !== "row") return [panel];
  const end = dashboard.panels.findIndex((next, i) => i > at && next.type === "row");
  return dashboard.panels.slice(at + 1, end < 0 ? undefined : end);
}

// Highlight keys are "<dashboard uid>.<panel key>"; keys for other dashboards are ignored.
export function panelRects(dashboard: Dashboard, keys: readonly string[], stage: { width: number; captionTop: number }, zoom: number): Rect[] {
  const inRows = dashboard.panels.some(panel => panel.type === "row");
  const left0 = padding + (inRows ? rowIndent : 0), top0 = padding + (inRows ? rowDrop : 0);
  const column = (stage.width / zoom - left0 - padding - gap * (columns - 1)) / columns;
  return keys.filter(key => key.startsWith(`${dashboard.uid}.`)).flatMap(key => {
    const panels = panelsFor(dashboard, key.slice(dashboard.uid.length + 1));
    if (!panels.length) return [];
    const left = Math.min(...panels.map(p => p.gridPos.x)), top = Math.min(...panels.map(p => p.gridPos.y));
    const right = Math.max(...panels.map(p => p.gridPos.x + p.gridPos.w)), bottom = Math.max(...panels.map(p => p.gridPos.y + p.gridPos.h));
    const x = (left0 + left * (column + gap)) * zoom, y = (top0 + top * (rowHeight + gap)) * zoom;
    const x2 = (left0 + right * (column + gap) - gap) * zoom, y2 = Math.min(stage.captionTop, (top0 + bottom * (rowHeight + gap) - gap) * zoom);
    return [{ key, x, y, width: x2 - x, height: y2 - y }];
  });
}

// Keys that don't name a panel on any dashboard (a test checks notes.ts has none).
export const unknownKeys = (dashboards: readonly Dashboard[], keys: readonly string[]) =>
  keys.filter(key => !dashboards.some(d => key.startsWith(`${d.uid}.`) && panelsFor(d, key.slice(d.uid.length + 1)).length));
