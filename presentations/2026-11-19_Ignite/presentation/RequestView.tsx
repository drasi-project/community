import { useEffect, useState } from "react";
import { request, taskSchema, type TaskDetail } from "../demo/ui/src/contracts";

// One late customer request, as the AI service recorded it: the same request is a trace in
// Langfuse. Shown during "Who is affected", for the oldest request in Drasi's overdue list.
type Group = { endpoint: string; outcome: string; count: number; first: number };

const outcome = (status: unknown) => status === 200 ? "answered" : status === 503 ? "failed (HTTP 503)"
  : status === 429 ? "over quota (HTTP 429)" : status == null ? "in progress" : `HTTP ${String(status)}`;
const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
const ago = (seconds: number) => seconds < 60 ? `${seconds} s` : `${Math.floor(seconds / 60)} min ${seconds % 60} s`;

export function RequestView({ taskId }: { taskId: string | null }) {
  const [data, setData] = useState<TaskDetail | null>(null);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!taskId) return;
    let active = true;
    const load = () => request(`/api/tasks/${encodeURIComponent(taskId)}`).then(value => { if (active) { setData(taskSchema.parse(value)); setNow(Date.now()); } }).catch(() => {});
    void load();
    const timer = setInterval(load, 2000);
    return () => { active = false; clearInterval(timer); };
  }, [taskId]);
  if (!taskId || !data) return <div className="request-slide"><span className="eyebrow">Who is affected · one late request</span><h1>Loading the request…</h1></div>;
  const { task, attempts } = data;
  const created = Number(task.created_ms), deadline = Number(task.deadline_ms);
  const completed = typeof task.completed_ms === "number" ? task.completed_ms : null;
  const groups: Group[] = [];
  for (const attempt of attempts) {
    const endpoint = String(attempt.endpoint), result = outcome(attempt.status), at = Number(attempt.started_ms);
    const last = groups.at(-1);
    if (last && last.endpoint === endpoint && last.outcome === result && !(last.first < deadline && at >= deadline)) last.count++;
    else groups.push({ endpoint, outcome: result, count: 1, first: at });
  }
  const waited = Math.round(((completed ?? now) - created) / 1000);
  return <div className="request-slide">
    <span className="eyebrow">Who is affected · one late request</span>
    <h1>{completed ? `Answered after ${ago(waited)}` : `A customer has been waiting ${ago(waited)}`}</h1>
    <blockquote className="question">“{String(task.content)}”</blockquote>
    <ol className="timeline">
      {[
        { at: created, className: "", text: "Request received" },
        ...groups.map(group => ({ at: group.first, className: group.outcome === "answered" ? "ok" : group.outcome === "in progress" ? "" : "bad",
          text: `Sent to ${group.endpoint}: ${group.outcome}${group.count > 1 ? ` × ${group.count}` : ""}` })),
        { at: deadline, className: "drasi", text: "15-second deadline passed: Drasi added it to the overdue list" },
      ].sort((a, b) => a.at - b.at).map((item, i) => <li key={i} className={item.className}><span>{clock(item.at)}</span><b>{item.text}</b></li>)}
    </ol>
    <p className="source">From the AI service's records. The same request is trace <code>{String(task.trace_id).slice(0, 8)}</code> in Langfuse.</p>
  </div>;
}
