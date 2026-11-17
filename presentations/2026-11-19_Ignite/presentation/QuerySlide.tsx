import { useQuery } from "../demo/ui/src/live";

// demo/drasi/overdue-tasks.cypher, laid out for the projector. A UI test checks it matches the file.
export const displayQuery = [
  ["MATCH (t:ignite_tasks)-[:TASK_SCENARIO]-(s:ignite_scenario)", 1],
  ["WHERE t.run_epoch = s.run_epoch", 0],
  ["  AND t.completed_ms IS NULL AND t.cancelled_ms IS NULL", 2],
  ["  AND drasi.trueLater(true, t.deadline_ms)", 3],
  ["RETURN t.id AS id, t.run_epoch AS run_epoch,", 4],
  ["       t.workload AS workload, t.trace_id AS trace_id,", 0],
  ["       t.deadline_ms AS deadline_ms, 'overdue' AS impact", 0],
] as const;

const notes = [
  "Every request, as it arrives from the database's change feed, in the current demo run",
  "Not answered or cancelled yet. When the answer is written, the request leaves the result",
  "Becomes true at the deadline. Drasi re-checks just that request at that moment: no new event, no scan",
  "The result is the list of late requests, with their traces. Drasi keeps it current and passes on every change",
];

function LiveResult() {
  const live = useQuery("overdue-tasks");
  const late = live.rows.filter(row => row.workload === "support")
    .sort((a, b) => Number(a.deadline_ms) - Number(b.deadline_ms));
  if (!live.attached) return <div className="query-live"><p><b>Live from Drasi:</b> connecting…</p></div>;
  return <div className="query-live">
    <p><b>Live from Drasi:</b> {late.length === 0 ? "no support requests are late right now"
      : `${late.length} support request${late.length === 1 ? " is" : "s are"} late right now`}</p>
    {late.length > 0 && <ol>{late.slice(0, 4).map(row => <li key={String(row.id)}>
      <span>late by {Math.max(0, Math.round((Date.now() - Number(row.deadline_ms)) / 1000))} s</span>
      <code>trace {String(row.trace_id).slice(0, 8)}</code></li>)}
      {late.length > 4 && <li className="more">and {late.length - 4} more</li>}</ol>}
  </div>;
}

// The live result connects to Drasi Server only while this slide is showing.
export function QuerySlide({ live }: { live: boolean }) {
  return <div className="query-slide">
    <span className="eyebrow">Who is affected · a continuous query</span>
    <h1>The query behind "Support overdue"</h1>
    <div className="query-body">
      <pre aria-label="overdue-tasks query">{displayQuery.map(([line, n], i) =>
        <span key={i} className={n === 3 ? "hot" : undefined}>{n ? <i>{n}</i> : <i aria-hidden="true" />}{line}{"\n"}</span>)}</pre>
      <ol className="query-notes">{notes.map(note => <li key={note}>{note}</li>)}</ol>
    </div>
    {live ? <LiveResult /> : <div className="query-live"><p><b>Live from Drasi</b></p></div>}
  </div>;
}
