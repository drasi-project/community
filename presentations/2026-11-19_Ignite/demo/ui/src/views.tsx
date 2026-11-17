import { useEffect, useRef, useState } from "react";
import { settings } from "./settings";
import { z } from "zod";
import { products, type View } from "../../../presentation/scenes";
import { isCurrent, readDecision, rows, text, request, errorText, type Row, type Selection } from "./contracts";
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
  const ready = !live.error && live.health?.continuous_context === "ready" && live.context?.streams_attached &&
    live.received !== null && now - live.received < 6000;
  return <div className={`live-banner ${ready ? "" : "warning"}`} role="status">
    <strong>{ready ? "LIVE · Context ready" : "UNKNOWN · Context unavailable or stale"}</strong>
    <span>{live.health ? <>Epoch <Basis value={live.health.run_epoch} /> · input r{live.health.revision}</> : "No authoritative model"}
      {live.received && <> · mirror received {Math.max(0, (now - live.received) / 1000).toFixed(1)}s ago</>}</span>
    {live.error && <span className="error-detail">{live.error}</span>}
  </div>;
}

function Operations({ live, setView }: { live: Live; setView: (view: View) => void }) {
  const health = live.health;
  return <div className="operations-grid">
    <Panel title="Grafana is the operational home" className="product-home">
      <p className="lead">Start with measured requests.<br />Return here to confirm outcomes.</p>
      <ProductLink href={products.grafana}>Open real Grafana dashboard</ProductLink>
      <div className="telemetry-path"><b>Application OTel</b><span>→ Collector → Prometheus →</span><b>Grafana</b></div>
      <p>Request success and latency, endpoint errors, queues, throughput, allocations and alerts live in the real product tab. No synthetic charts or iframe authentication workarounds.</p>
      <p className="muted">Prometheus scraping and Grafana evaluation remain periodic. “Without polling” refers to Drasi's change-driven and time-aware computation.</p>
    </Panel>
    <Panel title="Authoritative workload configuration">
      {health ? <>
        <table><thead><tr><th>Workload</th><th>Assigned</th><th>Commitment</th><th>Scope</th></tr></thead><tbody>
          {health.model.workloads.map(workload => <tr key={workload.id}><td>{workload.id}</td><td>{workload.destination}</td><td>{workload.reservation_tpm.toLocaleString()} units/min</td><td>{workload.region}</td></tr>)}
        </tbody></table>
        <div className="metrics"><div><strong>{health.pending}</strong><span>Pending tasks</span></div><div><strong>{health.task_target_seconds}s</strong><span>Demo task target</span></div><div><strong>{health.report_expiry_seconds}s</strong><span>Report expiry</span></div></div>
        <p>Configured token units and regional labels are simulated. HTTP requests, telemetry and task outcomes are real.</p>
      </> : <p className="warning">Configuration unknown. No replacement fixture is displayed.</p>}
      <div className="actions"><button onClick={() => setView("decisions")}>Explain decisions</button><button onClick={() => setView("trace")}>Inspect a task</button></div>
    </Panel>
    <Panel title="Observed endpoint evidence">
      <table><thead><tr><th>Endpoint</th><th>Query status</th><th>Observed</th><th>Sequence</th></tr></thead><tbody>
        {rows(live.context, "endpoint-freshness").map(row => <tr key={text(row.id)}><td>{text(row.id)}</td><td>{text(row.status)}</td><td>{typeof row.observed_ms === "number" ? new Date(row.observed_ms).toLocaleTimeString() : "unknown"}</td><td>{text(row.sequence)}</td></tr>)}
      </tbody></table>
      {!live.context?.streams_attached && <p className="warning">Disconnected mirror: rows are not proof of current health.</p>}
    </Panel>
    <Panel title="Grafana alert transitions → Drasi">
      {rows(live.context, "alert-context").length ? rows(live.context, "alert-context").slice(-3).map(row =>
        <div className="alert-row" key={text(row.id)}><Status bad={row.status !== "resolved"}>{text(row.status)}</Status><span>{text(row.title)}</span></div>
      ) : <p>No alert rows available. This is not a health assertion.</p>}
      <p className="muted">Normalized product webhooks; independent of scenario navigation.</p>
    </Panel>
  </div>;
}

function Decisions({ live, now, selection }: { live: Live; now: number; selection: Selection | null }) {
  const allocation = readDecision(live.context, "allocation-results");
  const policy = readDecision(live.context, "policy-results");
  const resilience = readDecision(live.context, "resilience-results");
  const decision = allocation.data;
  const current = isCurrent(decision, live.health, now) && !live.error && live.context?.streams_attached;
  const policyCurrent = isCurrent(policy.data, live.health, now) && !live.error && live.context?.streams_attached;
  const resilienceCurrent = isCurrent(resilience.data, live.health, now) && !live.error && live.context?.streams_attached;
  return <div className="decision-grid">
    <div className="decision-main">
    <Panel title="Permission is not the same as availability" className="eligibility">
      <div className="panel-line"><Status bad={!policyCurrent}>{policyCurrent ? `Current Rego policy r${policy.data?.policy_revision}` : "Unknown / historical policy evidence"}</Status><span>Only explicit allow is permission</span></div>
      {policy.data ? <table><thead><tr><th>Workload</th>{policy.data.model.endpoints.map(endpoint => <th key={endpoint.id}>{endpoint.id}</th>)}</tr></thead><tbody>
        {policy.data.model.workloads.map(workload => <tr key={workload.id}><td><b>{workload.id}</b><small>{workload.reservation_tpm.toLocaleString()} units/min</small></td>
          {policy.data?.model.endpoints.map(endpoint => {
            const eligibility = policy.data?.eligibility[workload.id]?.[endpoint.id];
            return <td key={endpoint.id}><strong className={eligibility?.allowed && policyCurrent ? "positive" : "warning"}>{!policyCurrent ? "Unknown" : eligibility ? eligibility.allowed ? "Allowed" : "Denied" : "Unknown"}</strong>
              <small>{eligibility?.reason ?? "No eligibility result"}</small><small>Observed: {policy.data?.evidence[endpoint.id]?.status ?? "unknown"}</small></td>;
          })}</tr>)}
      </tbody></table> : <p className="warning">{policy.error}</p>}
    </Panel>
    <Panel title="Current versus proposed allocation" className="allocation">
      {decision ? <div className="endpoint-cards">{decision.model.endpoints.map(endpoint =>
        <article className="endpoint-card" key={endpoint.id}>
          <h3>{endpoint.id} <small>{endpoint.region}</small></h3><p>{endpoint.budget_tpm.toLocaleString()} modeled units/min</p>
          <div className="assignment"><span>Current</span>{decision.model.workloads.filter(w => w.destination === endpoint.id).map(w => <b key={w.id}>{w.id} · {w.reservation_tpm.toLocaleString()}</b>)}</div>
          <div className="assignment proposed"><span>{current ? "Proposed" : "Historical proposal"}</span>{Object.entries(decision.assignments).filter(([, destination]) => destination === endpoint.id).map(([id]) => <b key={id}>{id} · {decision.model.workloads.find(w => w.id === id)?.reservation_tpm.toLocaleString()}</b>)}</div>
        </article>)}</div> : <p className="warning">{allocation.error}</p>}
      <p className="muted">Complete native-model assignments. Routing changes apply to new/queued requests, not in-flight work.</p>
    </Panel>
    <Panel title="Independent what-if: lose EU-A" className="resilience">
      <div className="large-result">{resilienceCurrent ? resilience.data?.state : "Unknown"}</div>
      <p>{resilience.data?.reason ?? resilience.error}</p>
      <p>Failure: <b>{resilience.data?.failure ?? "unknown"}</b> · read-only, not a command.</p>
      {resilience.data && <span>Basis <Basis value={resilience.data.basis} /></span>}
    </Panel>
    </div>
    <div className="decision-side">
    <Panel title="Candidate → command → recovery" className="lifecycle">
      <Status bad={!current}>{current ? `Candidate: ${decision?.state}` : "Candidate: unknown / stale"}</Status>
      <p className="decision-reason">{decision?.reason ?? allocation.error}</p>
      {decision && <p>Model r{decision.revision} · policy r{decision.policy_revision}<br />Basis <Basis value={decision.basis} /> · candidate <Basis value={decision.candidate_id} /><br />
        Evidence lease: {Object.values(decision.evidence).length ? `${Math.max(0, (Math.min(...Object.values(decision.evidence).map(e => e.expires_ms)) - now) / 1000).toFixed(1)}s remaining` : "unknown"}</p>}
      {selection && <div className="retained"><b>Selected proposal retained</b><p>Basis <Basis value={selection.basis} /> · model r{selection.revision}</p>
        <Status bad={selection.basis !== decision?.basis}>{selection.basis === decision?.basis ? "Matches current basis" : "STALE SELECTION · server must recheck"}</Status></div>}
      <CommandSummary live={live} />
    </Panel>
    </div>
  </div>;
}

export function CommandSummary({ live }: { live: Live }) {
  if (live.decisionsError) return <p className="warning">Command/publication status unavailable: {live.decisionsError}</p>;
  if (!live.decisions) return <p>Command status unknown.</p>;
  const command = live.decisions.commands[0];
  const recovery = command ? live.decisions.recovery[command.id] : null;
  const recoveryCurrent = live.decisions.streams_attached && !live.error && recovery?.run_epoch === live.health?.run_epoch &&
    recovery?.applied_revision === live.health?.revision && typeof recovery?.expires_ms === "number" && recovery.expires_ms > Date.now();
  return <div className="command-summary">
    {command ? <>
      <p>Command <Basis value={command.id} /> · <strong className={command.status === "rejected" ? "warning" : ""}>{command.status}</strong></p>
      <ol className="stages">
        <li><b>Committed</b><span>{new Date(command.committed_ms).toLocaleTimeString()}</span></li>
        <li><b>Gateway applied</b><span>{command.applied_ms ? `r${command.applied_revision} · ${new Date(command.applied_ms).toLocaleTimeString()}` : "Not acknowledged"}</span></li>
        <li><b>Recovery confirmed (history)</b><span>{command.recovered_ms ? new Date(command.recovered_ms).toLocaleTimeString() : "Not confirmed"}</span></li>
      </ol>
      <Status bad={!recoveryCurrent || recovery?.state !== "confirmed"}>Current recovery: {recoveryCurrent ? text(recovery?.state) : "unknown"}</Status>
      {command.error && <p className="warning">{command.error}</p>}
      <p className="muted">{live.decisions.observation_window_ms / 1000}s observation window; recent success within {live.decisions.success_freshness_ms / 1000}s. Historical confirmation does not promise current recovery.</p>
    </> : <p>No durable command in this epoch. Awaiting explicit approval.</p>}
    {live.decisions.publications.length > 0 ? <p className="warning">Publication pending / failed: {live.decisions.publications.map(item => `${item.target} (${item.attempts} attempts): ${item.last_error ?? "awaiting delivery"}`).join("; ")}</p> : <p className="muted">No undelivered publications reported.</p>}
    <details className="audit"><summary>Exact command, recovery & publication evidence</summary><Json value={live.decisions} /></details>
  </div>;
}

function FollowChange({ live }: { live: Live }) {
  const [query, setQuery] = useState("overdue-tasks");
  const stream = useQuery(query);
  const [page, setPage] = useState(0);
  const [pinned, setPinned] = useState<{ at: number; query: string; rows: Row[] } | null>(null);
  const [inspect, setInspect] = useState<Row | null>(null);
  const source = query === "overdue-tasks" ? rows(live.context, "task-context") : rows(live.context, "endpoint-freshness");
  const ordered = [...stream.rows].sort((a, b) => Number(b.created_ms ?? b.observed_ms ?? 0) - Number(a.created_ms ?? a.observed_ms ?? 0));
  const shown = ordered.slice(page * 3, page * 3 + 3);
  useEffect(() => { setPage(0); setInspect(null); }, [query]);
  return <div className="change-layout">
    <div className="flow" aria-label="Actual computation path"><span>PostgreSQL CDC</span><span>→ Continuous queries</span><span>→ Native policy & models</span><span>→ Results / reactions</span><ProductLink href={products.server}>Server graph UI</ProductLink></div>
    <div className="query-toolbar">
      <label>Actual query <select value={query} onChange={event => setQuery(event.target.value)}>{Object.keys(queryText).map(name => <option key={name}>{name}</option>)}</select></label>
      <Status bad={!stream.attached}>{stream.attached ? `Attached · ${stream.events} snapshot / difference frames` : "Unknown / disconnected"}</Status>
      <button onClick={() => setPinned({ at: Date.now(), query, rows: structuredClone(source) })}>Pin input evidence</button>
    </div>
    <div className="two-columns">
      <Panel title="Continuous query · actual bundled source"><pre className="query-code">{queryText[query]}</pre><p className="muted">Source: demo/drasi. Server delivery is snapshot-first; reconnect replaces all rows. Result files are shared by the three model-result queries.</p></Panel>
      <Panel title="Live maintained rows">
        {stream.error && <p className="warning" role="status">{stream.error}</p>}
        {stream.attached && !stream.rows.length && <p className="empty-state">Empty result set · no matching rows now</p>}
        {shown.map(row => <button className="row-card" data-row-id={text(row.id)} key={text(row.id)} onClick={() => setInspect(row)}>
          <b>{text(row.workload ?? row.id)}</b><span>{text(row.impact ?? row.state ?? row.status ?? row.completed_ms ?? row.revision)}</span>
          <small>{row.basis ? `basis ${text(row.basis).slice(0, 12)}` : `id ${text(row.id)}`}</small>
          {row.observed_ms !== undefined && <small>observed_ms {text(row.observed_ms)} · sequence {text(row.sequence)}</small>}
          {row.deadline_ms !== undefined && <small>deadline_ms {text(row.deadline_ms)}</small>}
        </button>)}
        <div className="pagination"><button disabled={page === 0} onClick={() => setPage(page - 1)}>Previous rows</button><span>{stream.rows.length} actual rows</span><button disabled={(page + 1) * 3 >= stream.rows.length} onClick={() => setPage(page + 1)}>Next rows</button></div>
      </Panel>
    </div>
    <div className="evidence-strip">{pinned ? <><b>PINNED SNAPSHOT · {new Date(pinned.at).toLocaleTimeString()} · {pinned.query}</b><span>{pinned.rows.length} input rows retained, not live</span><button onClick={() => setInspect({ captured_ms: pinned.at, rows: pinned.rows })}>Inspect pinned inputs</button></> : <><b>Demonstrate time without another source event</b><span>Pin the task/report input, then use an explicit desk control. Compare unchanged deadline, sequence and observation time with the new maintained result.</span></>}</div>
    {inspect && <EvidenceDialog value={inspect} close={() => setInspect(null)} />}
  </div>;
}

function Trace({ live }: { live: Live }) {
  const tasks = rows(live.context, "task-context").sort((a, b) => Number(b.created_ms) - Number(a.created_ms));
  const [selected, setSelected] = useState("");
  const task = tasks.find(row => row.id === selected) ?? tasks[0];
  const [link, setLink] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setLink(null); setError(null);
    if (!task) return;
    const controller = new AbortController();
    void request(`/api/tasks/${encodeURIComponent(text(task.id))}`, { signal: AbortSignal.any([controller.signal, AbortSignal.timeout(6000)]) })
      .then(data => {
        const result = z.object({ trace_url: z.string().url().optional() }).passthrough().parse(data);
        if (!result.trace_url) throw new Error("Task API has not returned a trace_url. Use the submitted task link on the desk or locate this exact trace ID in Langfuse.");
        const url = new URL(result.trace_url);
        if (url.origin !== settings.langfuse || !url.pathname.startsWith("/project/ignite-probe/traces/")) throw new Error("Unexpected trace destination");
        setLink(result.trace_url);
      }).catch(error => { if (!controller.signal.aborted) setError(errorText(error)); });
    return () => controller.abort();
  }, [task?.id]);
  const attempts = rows(live.context, "attempt-context").filter(row => row.task_id === task?.id);
  return <div className="two-columns">
    <Panel title="Follow one actual AI task">
      <label>Task <select value={task ? text(task.id) : ""} onChange={event => setSelected(event.target.value)}>{tasks.map(row => <option key={text(row.id)} value={text(row.id)}>{text(row.workload)} · {text(row.id).slice(-12)}</option>)}</select></label>
      {task ? <><p className="identity">Task {text(task.id)}</p><p>Trace <code>{text(task.trace_id)}</code></p><div className="large-result">{task.cancelled_ms ? "Cancelled" : task.completed_ms ? "Completed" : "Pending"}</div><p>{attempts.length} recorded HTTP attempts · destination {text(task.endpoint)}</p>
        {link ? <ProductLink href={link}>Open this real Langfuse trace</ProductLink> : <p className="warning">{error ?? "Loading returned trace link..."}</p>}</> : <p>No task evidence available.</p>}
      <ProductLink href={products.langfuse}>Open real Langfuse project</ProductLink>
      <p className="muted">Use actual task/trace identity. Trace ingestion and independent Drasi publication can lag; task completion alone is not delivery proof.</p>
    </Panel>
    <Panel title="Query-maintained outcome evidence">
      <table><thead><tr><th>Attempt endpoint</th><th>HTTP status</th><th>Finished</th></tr></thead><tbody>{attempts.slice(-5).map(row => <tr key={text(row.id)}><td>{text(row.endpoint)}</td><td>{text(row.status)}</td><td>{row.finished_ms ? new Date(Number(row.finished_ms)).toLocaleTimeString() : "Uncertain"}</td></tr>)}</tbody></table>
      <p>Showing latest five attempts. Inspect the product trace for the full execution.</p>
      <Status bad={rows(live.context, "overdue-tasks").some(row => row.id === task?.id)}>Current overdue result: {rows(live.context, "overdue-tasks").some(row => row.id === task?.id) ? "present" : live.context?.streams_attached ? "absent" : "unknown"}</Status>
      <p>Historical deadline breach: {rows(live.context, "breach-history").some(row => row.id === task?.id) ? "retained" : live.context?.streams_attached ? "no row in current mirror" : "unknown"}</p>
      <CommandSummary live={live} />
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
      <div hidden={view !== "operations"}><Operations live={live} setView={setView} /></div>
      <div hidden={view !== "decisions"}><Decisions live={live} now={now} selection={selection} /></div>
      <div hidden={view !== "change"}><FollowChange live={live} /></div>
      <div hidden={view !== "trace"}><Trace live={live} /></div>
    </div>
  </div>;
}
