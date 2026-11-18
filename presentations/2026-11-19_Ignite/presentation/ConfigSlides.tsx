import overdueQuery from "../demo/drasi/overdue-tasks.cypher?raw";
import healthQuery from "../demo/drasi/deployment-health.cypher?raw";
import workloadsQuery from "../demo/drasi/tile-workloads.cypher?raw";
import rego from "../demo/drasi/policy/ignite.rego?raw";
import optimization from "../demo/drasi/optimization/optimization.json";
import { buildConfig, type ConfigId } from "./codeSlides";

export const configPanels = buildConfig({ overdueQuery, healthQuery, workloadsQuery, rego, optimization });

// The configuration behind the highlighted dashboard panel, on the dimmed part of the dashboard
// above it (the Decision Support row is the bottom of the dashboard).
export function ConfigOverlay({ ids }: { ids: readonly ConfigId[] }) {
  const panels = ids.map(id => configPanels[id]).filter(Boolean);
  if (!panels.length) return null;
  return <div className="config-overlay">{panels.map(panel =>
    <div key={panel.tab} className="code-panel">
      <p className="code-tab">{panel.tab}</p>
      <div className="config-body">
        <ol className="flow-diagram" aria-label="What this brings together">
          <li className="flow-inputs">{panel.flow.inputs.map(input =>
            <span key={input.name} className={input.from === "OpenTelemetry" ? "otel" : input.from === "PostgreSQL" ? "pg" : "model"}>
              <b>{input.name}</b><small>{input.from}</small></span>)}</li>
          {panel.flow.via.map(step => <li key={step} className="flow-step">{step}</li>)}
          <li className="flow-tile">{panel.flow.tile}</li>
        </ol>
        <pre aria-label={panel.tab}>{panel.lines.map((line, i) =>
          <span key={i} className={panel.hot.some(text => line.includes(text)) ? "hot" : undefined}><i aria-hidden="true" />{line}{"\n"}</span>)}</pre>
      </div>
    </div>)}</div>;
}
