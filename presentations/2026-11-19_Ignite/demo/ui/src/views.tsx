import { useEffect, useRef, useState } from "react";
import { settings } from "./settings";
import { z } from "zod";
import { products, type View } from "../../../presentation/scenes";
import { readPlan, tileValue, text, request, errorText, taskSchema, type Row, type Selection } from "./contracts";
import { useQuery, type Live } from "./live";
import { queryText } from "./queryText";

export function ProductLink({ href, children }: { href: string; children: React.ReactNode }) {
  return <a className="button product-link" href={href} target="_blank" rel="noopener noreferrer">{children} <span aria-hidden="true">↗</span></a>;
}
export function Json({ value }: { value: unknown }) {
  return <pre className="json">{JSON.stringify(value, null, 2)}</pre>;
}
function EvidenceDialog({ value, close }: { value: unknown; close: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => { ref.current?.showModal(); }, []);
  return <dialog ref={ref} role="dialog" aria-label="Query evidence" className="evidence-dialog" onCancel={close}>
    <button autoFocus onClick={close}>Close evidence</button><h2>Selected evidence snapshot · not live</h2><Json value={value} />
  </dialog>;
}
export function Basis({ value }: { value: string }) {
  return <code className="basis" title={value}>{value.slice(0, 12)}</code>;
}
export function Status({ children, bad = false }: { children: React.ReactNode; bad?: boolean }) {
  return <span className={`status ${bad ? "warning" : ""}`}>{children}</span>;
}
export function Panel({ title, children, className = "" }: { title: string; children: React.ReactNode; className?: string }) {
  return <section className={`panel ${className}`}><h2>{title}</h2>{children}</section>;
}
export function LiveBanner({ live, now }: { live: Live; now: number }) {
  const ready = !live.error && !live.drasiError && live.received !== null && now - live.received < 6000;
  return <div className={`live-banner ${ready ? "" : "warning"}`} role="status">
    <strong>{ready ? "LIVE · Drasi attached" : "UNKNOWN · Drasi or the AI service unavailable"}</strong>
    <span>{live.health ? <>Epoch <Basis value={live.health.run_epoch} /> · scenario r{live.health.revision}</> : "AI service unavailable"}
      {live.received && <> · AI service polled {Math.max(0, (now - live.received) / 1000).toFixed(1)}s ago</>}</span>
    {(live.error || live.drasiError) && <span className="error-detail">{[live.error, live.drasiError].filter(Boolean).join("; ")}</span>}
  </div>;
}

const statusLabel = ["Healthy", "Over quota", "Failed"];

function Operations({ live, setView }: { live: Live; setView: (view: View) => void }) {
  const health = live.health;
  const deployments = useQuery("tile-deployments");
  return <div className="operations-grid">
    <Panel title="Grafana is the operational home" className="product-home">
      <p className="lead">Start with measured requests.<br />Return here to confirm outcomes.</p>
      <ProductLink href={products.grafana}>Open real Grafana dashboard</ProductLink>
      <div className="telemetry-path"><b>Application OTel</b><span>→ Collector → Prometheus →</span><b>Grafana</b></div>
      <p>Request success and latency, endpoint errors, queues and throughput come from the application's telemetry. Every Drasi tile comes from Drasi's Prometheus reaction.</p>
      <p className="muted">Prometheus scraping and Grafana evaluation remain periodic. “Without polling” refers to Drasi's change-driven and time-aware computation.</p>
    </Panel>
    <Panel title="Scenario in the AI service's database">
      {health ? <>
        <table><thead><tr><th>Workload</th><th>Route</th><th>Demand</th><th>Region</th></tr></thead><tbody>
          {health.workloads.map(workload => <tr key={workload.id}><td>{workload.id}</td><td>{workload.deployment}</td><td>{workload.demand_tpm.toLocaleString()} tokens/min</td><td>{workload.region_requirement}</td></tr>)}
        </tbody></table>
        <div className="metrics"><div><strong>{health.pending}</strong><span>Pending tasks</span></div><div><strong>{health.task_target_seconds}s</strong><span>Demo task target</span></div><div><strong>{health.report_expiry_seconds}s</strong><span>Report expiry</span></div></div>
        <p>Capacities and regions are simulated. HTTP requests, telemetry and task outcomes are real.</p>
      </> : <p className="warning">Scenario unknown. No replacement fixture is displayed.</p>}
      <div className="actions"><button onClick={() => setView("decisions")}>Explain decisions</button><button onClick={() => setView("trace")}>Inspect a task</button></div>
    </Panel>
    <Panel title="Deployment status (Drasi · tile-deployments)">
      <table><thead><tr><th>Deployment</th><th>Status</th><th>Capacity</th></tr></thead><tbody>
        {deployments.rows.map(row => <tr key={text(row.deployment)}><td>{text(row.deployment)}</td>
          <td>{typeof row.status === "number" && row.status >= 0 ? statusLabel[row.status] ?? text(row.status) : "Unknown"}</td><td>{text(row.capacity_tpm)}</td></tr>)}
      </tbody></table>
      {!deployments.attached && <p className="warning">{deployments.error}</p>}
    </Panel>
  </div>;
}

function Decisions({ live, selection }: { live: Live; selection: Selection | null }) {
  const policy = useQuery("policy-results");
  const resilience = useQuery("resilience-results");
  const { plan, error } = readPlan(live.queries["plan-results"].rows);
  const workloads = [...new Set(policy.rows.map(row => text(row.subject)))].sort();
  const targets = [...new Set(policy.rows.map(row => text(row.target)))].sort();
  const summary = resilience.rows.find(row => row.id === "resilience");
  const losses = resilience.rows.filter(row => row.id !== "resilience");
  const approval = live.queries["tile-last-approval"].rows[0];
  return <div className="decision-grid">
    <div className="decision-main">
    <Panel title="Permission is not the same as availability (policy-results)" className="eligibility">
      <div className="panel-line"><Status bad={!policy.attached}>{policy.attached ? "Drasi Rego policy plugin" : "Unknown / disconnected"}</Status><span>Only explicit allow is permission</span></div>
      {policy.rows.length ? <table><thead><tr><th>Workload</th>{targets.map(id => <th key={id}>{id}</th>)}</tr></thead><tbody>
        {workloads.map(workload => <tr key={workload}><td><b>{workload}</b></td>
          {targets.map(target => {
            const row = policy.rows.find(r => r.subject === workload && r.target === target);
            return <td key={target}><strong className={row?.state === "allowed" ? "positive" : "warning"}>{row ? row.state === "allowed" ? "Allowed" : row.state === "denied" ? "Denied" : "Unknown" : "Unknown"}</strong>
              <small>{text(row?.reason ?? "No policy result")}</small></td>;
          })}</tr>)}
      </tbody></table> : <p className="warning">{policy.error ?? "No policy results"}</p>}
    </Panel>
    <Panel title="Proposed plan (plan-results)" className="allocation">
      {plan ? <>
        <p><b>{plan.state}</b> · {plan.moves} move{plan.moves === 1 ? "" : "s"} · {plan.reason}</p>
        <table><thead><tr><th>Workload</th><th>Deployment</th></tr></thead><tbody>
          {Object.entries(plan.assignments).map(([item, bin]) => <tr key={item}><td>{item}</td><td>{bin}</td></tr>)}</tbody></table>
      </> : <p className="warning">{error}</p>}
      <p className="muted">Drasi's optimizer plugin. Routing changes apply to new and queued requests, not in-flight work.</p>
    </Panel>
    <Panel title="What if one fails (resilience-results)" className="resilience">
      <div className="large-result">{text(summary?.state)}</div>
      <p>First failure: <b>{text(summary?.first_failure || "none")}</b> · read-only, not a command.</p>
      <ul>{losses.map(row => <li key={text(row.id)}>Lose {text(row.lost ?? row.id)}: {text(row.state)}</li>)}</ul>
      {typeof summary?.basis === "string" && <span>Basis <Basis value={summary.basis} /></span>}
    </Panel>
    </div>
    <div className="decision-side">
    <Panel title="Candidate → approval → recovery" className="lifecycle">
      <Status bad={!plan || plan.state === "unknown"}>{plan ? `Plan: ${plan.state}` : "Plan: unknown"}</Status>
      {plan && <p>Scenario r{plan.revision}<br />Basis <Basis value={plan.basis} /> · candidate <Basis value={plan.candidate_id || "none"} /></p>}
      {selection && <div className="retained"><b>Selected proposal retained</b><p>Basis <Basis value={selection.basis} /> · scenario r{selection.revision}</p>
        <Status bad={selection.basis !== plan?.basis}>{selection.basis === plan?.basis ? "Matches current basis" : "Differs from Drasi's current plan"}</Status></div>}
      <p>Last approval (tile-last-approval): {approval ? <><Basis value={text(approval.approval)} /> · state {text(approval.value)}{approval.reason ? ` · ${text(approval.reason)}` : ""}</> : "none yet"}</p>
      <p>Recovery (tile-recovery): {text(live.queries["tile-recovery"].rows[0]?.value ?? "nothing to verify")}</p>
    </Panel>
    </div>
  </div>;
}

function FollowChange() {
  const names = Object.keys(queryText);
  const [query, setQuery] = useState(names.includes("overdue-tasks") ? "overdue-tasks" : names[0] ?? "overdue-tasks");
  const stream = useQuery(query);
  const [page, setPage] = useState(0);
  const [pinned, setPinned] = useState<{ at: number; query: string; rows: Row[] } | null>(null);
  const [inspect, setInspect] = useState<Row | null>(null);
  const ordered = [...stream.rows].sort((a, b) => Number(b.created_ms ?? b.observed_ms ?? 0) - Number(a.created_ms ?? a.observed_ms ?? 0));
  const shown = ordered.slice(page * 3, page * 3 + 3);
  useEffect(() => { setPage(0); setInspect(null); }, [query]);
  return <div className="change-layout">
    <div className="flow" aria-label="Actual computation path"><span>PostgreSQL CDC</span><span>→ Continuous queries</span><span>→ Policy and optimizer plugins</span><span>→ Results / reactions</span><ProductLink href={products.server}>Server graph UI</ProductLink></div>
    <div className="query-toolbar">
      <label>Actual query <select value={query} onChange={event => setQuery(event.target.value)}>{names.map(name => <option key={name}>{name}</option>)}</select></label>
      <Status bad={!stream.attached}>{stream.attached ? `Attached · ${stream.events} snapshot / difference frames` : "Unknown / disconnected"}</Status>
      <button onClick={() => setPinned({ at: Date.now(), query, rows: structuredClone(stream.rows) })}>Pin current rows</button>
    </div>
    <div className="two-columns">
      <Panel title="Continuous query · actual bundled source"><pre className="query-code">{queryText[query] ?? ""}</pre><p className="muted">Source: demo/drasi. Server delivery is snapshot-first; reconnect replaces all rows.</p></Panel>
      <Panel title="Live maintained rows">
        {stream.error && <p className="warning" role="status">{stream.error}</p>}
        {stream.attached && !stream.rows.length && <p className="empty-state">Empty result set · no matching rows now</p>}
        {shown.map((row, i) => <button className="row-card" data-row-id={text(row.id)} key={text(row.id ?? i)} onClick={() => setInspect(row)}>
          <b>{text(row.workload ?? row.id)}</b><span>{text(row.impact ?? row.state ?? row.status ?? row.value ?? row.revision)}</span>
          <small>{row.basis ? `basis ${text(row.basis).slice(0, 12)}` : `id ${text(row.id)}`}</small>
          {row.deadline_ms !== undefined && <small>deadline_ms {text(row.deadline_ms)}</small>}
        </button>)}
        <div className="pagination"><button disabled={page === 0} onClick={() => setPage(page - 1)}>Previous rows</button><span>{stream.rows.length} actual rows</span><button disabled={(page + 1) * 3 >= stream.rows.length} onClick={() => setPage(page + 1)}>Next rows</button></div>
      </Panel>
    </div>
    <div className="evidence-strip">{pinned ? <><b>PINNED SNAPSHOT · {new Date(pinned.at).toLocaleTimeString()} · {pinned.query}</b><span>{pinned.rows.length} rows retained, not live</span><button onClick={() => setInspect({ captured_ms: pinned.at, rows: pinned.rows })}>Inspect pinned rows</button></> : <><b>Demonstrate time without another source event</b><span>Pin the rows, wait for a deadline to pass, and compare with the maintained result.</span></>}</div>
    {inspect && <EvidenceDialog value={inspect} close={() => setInspect(null)} />}
  </div>;
}

function Trace({ live }: { live: Live }) {
  const tasks = [...live.queries["overdue-tasks"].rows].sort((a, b) => Number(a.deadline_ms) - Number(b.deadline_ms));
  const [selected, setSelected] = useState("");
  const task = tasks.find(row => row.id === selected) ?? tasks[0];
  const [detail, setDetail] = useState<{ attempts: Row[]; link: string | null } | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setDetail(null); setError(null);
    if (!task) return;
    const controller = new AbortController();
    void request(`/api/tasks/${encodeURIComponent(text(task.id))}`, { signal: AbortSignal.any([controller.signal, AbortSignal.timeout(6000)]) })
      .then(data => {
        const result = taskSchema.extend({ trace_url: z.string().url().optional() }).parse(data);
        let link: string | null = null;
        if (result.trace_url) {
          const url = new URL(result.trace_url);
          if (url.origin !== settings.langfuse || !url.pathname.startsWith("/project/ignite-probe/traces/")) throw new Error("Unexpected trace destination");
          link = result.trace_url;
        }
        setDetail({ attempts: result.attempts, link });
      }).catch(error => { if (!controller.signal.aborted) setError(errorText(error)); });
    return () => controller.abort();
  }, [task?.id]);
  return <div className="two-columns">
    <Panel title="Follow one late request (overdue-tasks)">
      <label>Task <select value={task ? text(task.id) : ""} onChange={event => setSelected(event.target.value)}>{tasks.map(row => <option key={text(row.id)} value={text(row.id)}>{text(row.workload)} · {text(row.id).slice(-12)}</option>)}</select></label>
      {task ? <><p className="identity">Task {text(task.id)}</p><p>Trace <code>{text(task.trace_id)}</code></p>
        {detail?.link ? <ProductLink href={detail.link}>Open this real Langfuse trace</ProductLink> : <p className="warning">{error ?? "Loading the trace link…"}</p>}</> : <p>{live.queries["overdue-tasks"].attached ? "No late requests right now." : "Overdue list unknown."}</p>}
      <ProductLink href={products.langfuse}>Open real Langfuse project</ProductLink>
      <p className="muted">Trace ingestion can lag; task completion alone is not delivery proof.</p>
    </Panel>
    <Panel title="Attempts (AI service task record)">
      <table><thead><tr><th>Deployment</th><th>HTTP status</th><th>Finished</th></tr></thead><tbody>{(detail?.attempts ?? []).slice(-5).map((row, i) => <tr key={text(row.id ?? i)}><td>{text(row.endpoint ?? row.deployment)}</td><td>{text(row.status)}</td><td>{row.finished_ms ? new Date(Number(row.finished_ms)).toLocaleTimeString() : "Uncertain"}</td></tr>)}</tbody></table>
      <p>Showing the latest five attempts. Inspect the product trace for the full execution.</p>
    </Panel>
  </div>;
}

export function Demo({ live, now, view, setView, selection }: { live: Live; now: number; view: View; setView: (view: View) => void; selection: Selection | null }) {
  return <div className="demo" data-testid="persistent-demo">
    <nav className="view-tabs" aria-label="Audience views">{(["operations", "decisions", "change", "trace"] as const).map(item =>
      <button key={item} aria-current={view === item ? "page" : undefined} onClick={() => setView(item)}>{({ operations: "Operations", decisions: "Decisions", change: "Follow a Change", trace: "Task / trace" })[item]}</button>)}
      <ProductLink href={products.grafana}>Real Grafana</ProductLink>
    </nav>
    <LiveBanner live={live} now={now} />
    <div className="view-body">
      <div hidden={view !== "operations"}>{view === "operations" && <Operations live={live} setView={setView} />}</div>
      <div hidden={view !== "decisions"}>{view === "decisions" && <Decisions live={live} selection={selection} />}</div>
      <div hidden={view !== "change"}>{view === "change" && <FollowChange />}</div>
      <div hidden={view !== "trace"}><Trace live={live} /></div>
    </div>
  </div>;
}
