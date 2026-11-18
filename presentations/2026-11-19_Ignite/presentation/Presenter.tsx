import { useEffect, useRef, useState } from "react";
import { settings } from "../demo/ui/src/settings";
import { scenes, slides, recaps, products, duration, viewLabel, type StageView } from "./scenes";
import "./desk.css";
import { notes, actionLabels, glossary, questions, type ActionId, type Beat } from "./notes";
import { replayClips } from "./replay";
import { post, readPlan, tileValue, errorText, rowSchema, type Health, type Row } from "../demo/ui/src/contracts";
import { ProductLink, Json } from "../demo/ui/src/views";
import type { Live, QueryState } from "../demo/ui/src/live";
import type { StageControl } from "../demo/ui/src/stage";

// Match questions in demo/workloads/src/content.rs so the simulated model gives a specific answer.
const SUPPORT_QUESTIONS = [
  "I was charged twice for my subscription this month. Can you refund one of the charges?",
  "I can't sign in after resetting my password. It says my account is locked.",
  "My order hasn't arrived and the tracking hasn't updated for a week.",
];
const SUMMARY_REQUESTS = ["Summarize the public release notes for version 4.2."];
const pick = (items: string[]) => items[Math.floor(Math.random() * items.length)];
const workloadName: Record<string, string> = { support: "Support", batch: "Summaries" };
const modeLabel: Record<string, string> = { ok: "Healthy", failed: "Failed", throttled: "Throttled" };

const timerKey = `${settings.storageKey}-timer`;
const startTimer = () => sessionStorage.setItem(timerKey, String(Date.now()));
// The talk timer starts by itself on the first Next.
function Clock({ now }: { now: number }) {
  const value = Number(sessionStorage.getItem(timerKey) ?? NaN);
  const started = Number.isFinite(value) ? value : null;
  const [, refresh] = useState(0);
  const setStarted = (time: number) => { sessionStorage.setItem(timerKey, String(time)); refresh(n => n + 1); };
  const elapsed = started === null ? 0 : Math.max(0, Math.floor((now - started) / 1000));
  const label = `${Math.floor(elapsed / 60).toString().padStart(2, "0")}:${(elapsed % 60).toString().padStart(2, "0")}`;
  return <span className="clock"><strong className={elapsed >= 22 * 60 ? "warning" : ""}>{label}</strong>
    <button className="quiet" onClick={() => setStarted(Date.now())}>{started === null ? "Start timer" : "Restart timer"}</button></span>;
}

// `approval` is set for an approval sent to the approval tool: its outcome comes from Drasi.
type Feedback = { action: string; tone: "ok" | "expected" | "error"; message: string; data?: unknown; approval?: string; expectRejected?: boolean };
type Request = { path: string; body: unknown };

// The latest approval as Drasi reports it (tile-last-approval): 1 checking, 2 applied, 3 verified, 4 rejected.
function lastApproval(query: QueryState): { id: string; state: number | null; reason: string } | null {
  const row: Row | undefined = query.rows[0];
  return row ? { id: String(row.approval ?? ""), state: tileValue([row]), reason: typeof row.reason === "string" && row.reason ? row.reason : "the plan was out of date" } : null;
}

// What Drasi has decided about an approval the presenter sent, in the desk's words.
function approvalFeedback(feedback: Feedback, live: Live): Feedback {
  const latest = lastApproval(live.queries["tile-last-approval"]);
  const state = latest && latest.id === feedback.approval ? latest.state : null;
  const reason = latest?.reason ?? "";
  if (state === 4) return feedback.expectRejected
    ? { ...feedback, tone: "expected", message: `Rejected by Drasi, as intended: ${reason}.` }
    : { ...feedback, tone: "error", message: `Drasi rejected it: ${reason}. Select the proposed plan and approve again.` };
  if (state === 2 || state === 3) return feedback.expectRejected
    ? { ...feedback, tone: "error", message: "It was accepted, not rejected. Carry on: the plan is being applied." }
    : { ...feedback, tone: "ok", message: "Done: Drasi checked it and the new routes are applied." };
  return { ...feedback, tone: "ok", message: "Sent to the approval tool. Waiting for Drasi's check…" };
}

// The Ignite template's artwork and fonts (ops/template-assets.py), checked once when the desk
// opens: without them the projector falls back to a plain background and the system font.
function useTemplateAssets() {
  const [missing, setMissing] = useState<string[] | null>(null);
  useEffect(() => {
    void (async () => {
      try {
        const response = await fetch("/template/manifest.json", { cache: "no-store" });
        if (!response.ok) throw new Error("no manifest");
        const manifest = await response.json() as { required?: unknown };
        const required = Array.isArray(manifest.required) ? manifest.required.map(String) : [];
        const present = await Promise.all(required.map(file =>
          fetch(`/template/${file}`, { method: "HEAD", cache: "no-store" }).then(r => r.ok, () => false)));
        setMissing(required.filter((_, i) => !present[i]));
      } catch { setMissing(["all"]); }
    })();
  }, []);
  return missing;
}
const templateHelp = "Run python3 ops/template-assets.py in demo/ (see SETUP.md)";

// The projector window opened from the desk, so the desk can show whether it's still open.
let projector: Window | null = null;

export default function Presenter({ live, now, control }: { live: Live; now: number; control: StageControl }) {
  const { stage } = control;
  const health = live.health;
  const ready = !live.error && !live.drasiError && !!health && live.received !== null && now - live.received < 6000;
  const [busy, setBusy] = useState(false);
  const [sent, setFeedback] = useState<Feedback | null>(null);
  const feedback = sent?.approval ? approvalFeedback(sent, live) : sent;
  const [awaiting, setAwaiting] = useState<{ run_epoch: string; revision: number } | null>(null);
  const [connected, setConnected] = useState(false);
  const templateMissing = useTemplateAssets();
  // Set when Next is pressed before the beat's buttons: a second Next moves on anyway.
  const [nudged, setNudged] = useState<string | null>(null);
  const modelObserved = !awaiting || (health?.run_epoch === awaiting.run_epoch && health.revision >= awaiting.revision);
  useEffect(() => { if (awaiting && modelObserved) setAwaiting(null); }, [awaiting, modelObserved]);
  useEffect(() => { const t = setInterval(() => setConnected(!!projector && !projector.closed), 1000); return () => clearInterval(t); }, []);
  const canAct = ready && !busy && modelObserved && !stage.replay && !stage.fallback;

  // Drasi's plan, current only once Drasi has seen the scenario revision the AI service reports.
  const candidate = readPlan(live.queries["plan-results"].rows).plan;
  const proposalCurrent = !!candidate && !!health && live.queries["plan-results"].attached && candidate.state !== "unknown" &&
    candidate.run_epoch === health.run_epoch && candidate.revision === health.revision;
  const feasible = proposalCurrent && candidate.state === "feasible";
  const needsChange = feasible && candidate.moves > 0;
  // What runs now, as Drasi sees it (tile-running): a negative value means policy no longer allows that route.
  const running = Object.fromEntries(live.queries["tile-running"].rows.map(row =>
    [String(row.workload), `${String(row.deployment)}${typeof row.value === "number" && row.value < 0 ? " (not allowed)" : ""}`]));
  const selection = stage.selection;
  const stale = !!selection && selection.basis !== candidate?.basis;
  const overdue = live.queries["overdue-tasks"].rows
    .filter(row => row.workload === "support" && typeof row.id === "string")
    .sort((a, b) => Number(a.deadline_ms) - Number(b.deadline_ms));

  // Beats, skipping optional ones when short on time.
  const beatsOf = (cue: number) => notes[scenes[cue].id].beats
    .map((beat, index) => ({ beat, index })).filter(({ beat }) => !(stage.short && beat.optional));
  const cue = scenes[stage.cue];
  const visible = beatsOf(stage.cue);
  const position = Math.max(0, visible.findIndex(item => item.index === stage.beat));
  const beat: Beat = notes[cue.id].beats[stage.beat] ?? visible[0].beat;

  function goTo(cueIndex: number, beatIndex: number) {
    const target = notes[scenes[cueIndex].id].beats[beatIndex];
    let trace: string | null = null;
    if (target.show === "request") {
      trace = overdue[0] ? String(overdue[0].id) : null;
      if (!trace) return step(1, cueIndex, beatIndex); // nothing late to show: skip this beat
    }
    setFeedback(null);
    setNudged(null);
    // In recording mode, parts of the live demo show their recording instead.
    const recording = stage.fallback && (target.show === "ops" || target.show === "plan" || target.show === "request")
      ? target.recording ?? notes[scenes[cueIndex].id].recording ?? null : null;
    control.update({ cue: cueIndex, beat: beatIndex, view: target.show, trace, ...(stage.fallback ? { replay: recording } : {}) });
  }
  function step(direction: 1 | -1, fromCue = stage.cue, fromBeat = stage.beat) {
    const here = `${fromCue}/${fromBeat}`;
    const missing = direction === 1 && fromCue === stage.cue && fromBeat === stage.beat && !stage.fallback
      ? (beat.do ?? []).filter(id => !isDone(id)) : [];
    if (missing.length && nudged !== here) { setNudged(here); return; }
    if (direction === 1 && !sessionStorage.getItem(timerKey)) startTimer();
    const all = scenes.flatMap((_, c) => beatsOf(c).map(({ index }) => ({ c, b: index })));
    const at = all.findIndex(item => item.c === fromCue && item.b === fromBeat);
    const next = all[at + direction] ?? (at < 0 ? all[0] : null);
    if (next) goTo(next.c, next.b);
  }
  const stepRef = useRef(step);
  stepRef.current = step;
  useEffect(() => {
    const handle = (key: string) => {
      if ([" ", "PageDown", "ArrowRight", "ArrowDown", "Enter"].includes(key)) stepRef.current(1);
      if (["PageUp", "ArrowLeft", "ArrowUp", "Backspace"].includes(key)) stepRef.current(-1);
    };
    control.keyHandler.current = handle;
    const keyboard = (event: KeyboardEvent) => {
      if (event.target instanceof HTMLElement && event.target.closest("input,textarea,select,summary,button,a")) return;
      if ([" ", "PageDown", "ArrowRight", "ArrowDown", "PageUp", "ArrowLeft", "ArrowUp"].includes(event.key)) { event.preventDefault(); handle(event.key); }
    };
    window.addEventListener("keydown", keyboard);
    return () => window.removeEventListener("keydown", keyboard);
  }, [control.keyHandler]);
  function openProjector() {
    projector = window.open(`/present?view=${stage.view}`, "ignite-projector", "popup,width=1280,height=720");
    setConnected(!!projector);
  }
  const show = (view: StageView) => control.show(view);

  // Sends the requests in order; stops at the first that fails.
  async function run(action: string, requests: Request[], expectRejected = false) {
    if (!canAct) return;
    setBusy(true); setFeedback(null);
    try {
      let data: Row = {};
      for (const { path, body } of requests) data = rowSchema.parse(await post(path, body));
      if (typeof data.run_epoch === "string" && typeof data.revision === "number") setAwaiting({ run_epoch: data.run_epoch, revision: data.revision });
      if (requests[0].path === "/reset") control.update({ selection: null });
      // The approval tool only records the approval; Drasi decides whether it is applied or rejected.
      if (requests[0].path === "/approvals") setFeedback({ action, tone: "ok", message: "", data, approval: String(data.id), expectRejected });
      else setFeedback({ action, tone: "ok", message: "Done.", data });
    } catch (error) {
      const text = errorText(error);
      const reason = /"error":"([^"]+)"/.exec(text)?.[1] ?? text;
      setFeedback({ action, tone: "error", message: `Didn't work: ${reason}. Check the banner says READY, then try once more.` });
    } finally {
      if (requests[0].path === "/approvals") control.select(null);
      setBusy(false);
    }
  }
  const configure = (action: string, change: { capacity?: Record<string, number>; destinations?: Record<string, string[]> }, restore?: string) => {
    if (!health) return;
    const target = health.deployments.find(d => d.id === restore);
    const restoring: Request[] = target && (target.mode !== "ok" || !target.reporting)
      ? [{ path: "/control", body: { run_epoch: health.run_epoch, deployment: restore, action: "restore" } }] : [];
    void run(action, [...restoring, { path: "/configuration", body: { run_epoch: health.run_epoch, revision: health.revision, ...change } }]);
  };
  const euA = health?.deployments.find(d => d.id === "EU-A");
  const batch = health?.destinations.batch ?? [];
  const actions: Record<ActionId, { enabled: boolean; press: () => void; danger?: boolean; done?: boolean }> = {
    "start-traffic": { enabled: !health?.traffic?.running, done: !!health?.traffic?.running,
      press: () => health && void run("start-traffic", [{ path: "/traffic", body: { run_epoch: health.run_epoch, running: true } }]) },
    "stop-traffic": { enabled: !!health?.traffic?.running,
      press: () => health && void run("stop-traffic", [{ path: "/traffic", body: { run_epoch: health.run_epoch, running: false } }]) },
    "fail-eu-a": { enabled: euA?.mode === "ok", danger: true,
      press: () => health && void run("fail-eu-a", [{ path: "/control", body: { run_epoch: health.run_epoch, deployment: "EU-A", action: "fail" } }]) },
    "select-plan": { enabled: needsChange, done: !!selection && !stale,
      press: () => candidate && control.select({ approval_id: crypto.randomUUID(), candidate_id: candidate.candidate_id, basis: candidate.basis,
        run_epoch: candidate.run_epoch, revision: candidate.revision, assignments: candidate.assignments, selected_ms: Date.now() }) },
    "approve-plan": { enabled: !!selection,
      press: () => selection && void run("approve-plan", [{ path: "/approvals", body: { id: selection.approval_id, candidate_id: selection.candidate_id, basis: selection.basis, revision: selection.revision } }], beat.expect === "rejected") },
    "summaries-europe": { enabled: batch.includes("US-C"),
      press: () => configure("summaries-europe", { destinations: { batch: ["EU-A", "EU-B"] } }) },
    "summaries-anywhere": { enabled: !!health && !batch.includes("US-C"),
      press: () => configure("summaries-anywhere", { destinations: { batch: ["EU-A", "EU-B", "US-C"] } }) },
    "eu-a-half": { enabled: true, press: () => configure("eu-a-half", { capacity: { "EU-A": 50000 } }, "EU-A") },
    "eu-a-full": { enabled: true, press: () => configure("eu-a-full", { capacity: { "EU-A": 100000 } }, "EU-A") },
  };
  function isDone(id: ActionId) {
    switch (id) {
      case "start-traffic": return !!health?.traffic?.running;
      case "stop-traffic": return !health?.traffic?.running;
      case "fail-eu-a": return euA?.mode === "failed";
      case "select-plan": return !!selection || !needsChange;
      // A rejected approval is done once Drasi has answered it.
      case "approve-plan": return beat.expect === "rejected" ? feedback?.action === "approve-plan" && feedback.tone !== "ok" : !needsChange;
      case "summaries-europe": return !batch.includes("US-C");
      case "summaries-anywhere": return batch.includes("US-C");
      case "eu-a-half": return euA?.capacity_tpm === 50000 && euA.mode === "ok";
      case "eu-a-full": return euA?.capacity_tpm === 100000 && euA.mode === "ok";
    }
  }
  // Plain functions, not components: a component defined in here would be a new type on every
  // render (every second), so React would replace the button mid-click and lose the click.
  const actionButton = (id: ActionId, n?: number) => {
    const a = actions[id];
    const done = isDone(id);
    return <button className={`action ${a.danger ? "danger" : "primary"} ${done ? "done" : ""}`} disabled={!canAct || !a.enabled} onClick={a.press}>
      {n !== undefined && <span aria-hidden="true">{n}. </span>}{actionLabels[id]}{done && <span aria-hidden="true"> ✓</span>}</button>;
  };
  const result = (ids: readonly string[]) => feedback && ids.includes(feedback.action)
    ? <p className={`result ${feedback.tone}`} role="status">{feedback.message}</p> : null;
  const recording = notes[cue.id].recording;
  const nextItem = (() => {
    const all = scenes.flatMap((_, c) => beatsOf(c).map(({ index }) => ({ c, b: index })));
    const at = all.findIndex(item => item.c === stage.cue && item.b === stage.beat);
    return all[at + 1];
  })();

  return <div className="desk">
    <header className="desk-header">
      <b>PRESENTER DESK · DO NOT PROJECT</b>
      <span className={`badge ${ready ? "ok" : "bad"}`}>{ready ? "READY" : "NOT READY"}</span>
      <span className={`badge ${connected ? "ok" : "warn"}`}>{connected ? "Projector window connected" : "Projector window not opened from this desk"}</span>
      {!connected && <button onClick={openProjector}>Open projector window</button>}
      {!!templateMissing?.length && <span className="badge warn" title={templateHelp}>Ignite template assets missing</span>}
      <Clock now={now} />
    </header>
    {!ready && !stage.fallback && <p className="desk-alert">Drasi's answers are unavailable or out of date, so live buttons are off. <button onClick={() => control.update({ fallback: true })}>Use recordings for the rest of the talk</button></p>}
    {stage.fallback ? <p className="desk-alert">Recording mode. <button onClick={() => control.update({ fallback: false, replay: null })}>Back to the live demo</button></p>
      : stage.replay && <p className="desk-alert">A recording is on the projector. <button onClick={() => control.update({ replay: null })}>Back to live</button></p>}

    <main className="desk-main">
      <section className="now" aria-label="Now">
        <div className="now-head">
          <h1>{stage.cue + 1}. {cue.title} <small>{duration(cue.minutes)}</small></h1>
          <ol className="beat-dots" aria-label="Beats">{visible.map((item, i) => <li key={item.index} className={item.index === stage.beat ? "current" : i < position ? "past" : ""} />)}</ol>
          <label className="short"><input type="checkbox" checked={stage.short} onChange={event => control.update({ short: event.target.checked })} /> Short on time</label>
        </div>
        <div className="now-nav">
          <button className="big" onClick={() => step(-1)}>◀ Back</button>
          <button className="big primary" onClick={() => step(1)} disabled={!nextItem}>Next ▶</button>
          <p className="projector-line">Projector: <b>{viewLabel(stage.view)}</b>{beat.optional && <span className="optional"> · optional</span>}</p>
        </div>
        {nudged === `${stage.cue}/${stage.beat}` && <p className="result error" role="alert">You haven't pressed {(beat.do ?? []).filter(id => !isDone(id)).map(id => actionLabels[id]).join(" or ")} yet. Press it, or press Next again to move on without it.</p>}
        {stage.fallback && <p className="result expected">Recording mode: the projector shows recordings for the live parts. Say they're recordings; keep pressing Next.</p>}
        {beat.do && !stage.fallback && <div className="now-actions">
          {beat.do.map((id, i) => <span key={id}>{actionButton(id, beat.do!.length > 1 ? i + 1 : undefined)}</span>)}
          {beat.expect === "rejected" && <span className="hint">This approval is meant to be rejected.</span>}
          {result(beat.do)}
        </div>}
        <p className="say">{beat.say}</p>
        {beat.watch && <p className="watch"><b>Watch for</b> {beat.watch}</p>}
        {nextItem && <p className="up-next">Next: <b>{viewLabel(notes[scenes[nextItem.c].id].beats[nextItem.b].show)}</b> · {notes[scenes[nextItem.c].id].beats[nextItem.b].say.slice(0, 90)}…</p>}
        <p className="if-broken"><b>If it breaks</b> {notes[cue.id].ifBroken}
          {!stage.fallback && <button onClick={() => control.update({ fallback: true, replay: beat.recording ?? recording ?? null })}>Use recordings from here on</button>}</p>
        <p className="wrong-press"><b>Pressed the wrong button?</b> Back and Next never change the demo. Undo with the opposite button in <i>All demo controls</i>: Summaries: Europe + US undoes Europe only; EU-A back to 100k undoes 50k; to undo Fail EU-A, use Any deployment › EU-A › Restore.</p>
      </section>

      <aside className="live" aria-label="Live demo">
        <h2>Drasi's answers (as on the projector)</h2>
        <Answers live={live} />
        <h2>Live demo</h2>
        <ul className="deployments-state">{(health?.deployments ?? []).map(e => {
          const mode = e.reporting ? modeLabel[e.mode] ?? e.mode : "Not reporting";
          return <li key={e.id} className={e.mode === "ok" && e.reporting ? "ok" : "bad"}><b>{e.id}</b> {mode}<small>{e.capacity_tpm / 1000}k</small></li>;
        })}</ul>
        <p className="traffic">Traffic: {health?.traffic?.running ? "running" : "stopped"} · Summaries allowed in {batch.includes("US-C") ? "Europe + US" : "Europe only"}</p>
        <h2>Recovery plan</h2>
        <table className="plan-table">
          <thead><tr><th /><th>Support</th><th>Summaries</th></tr></thead>
          <tbody>
            <tr><th>Running now</th><td>{running.support ?? "?"}</td><td>{running.batch ?? "?"}</td></tr>
            <tr className={needsChange ? "proposed" : ""}><th>Proposed plan</th>{feasible
              ? <><td>{candidate.assignments.support}</td><td>{candidate.assignments.batch}</td></>
              : <td colSpan={2} className={proposalCurrent ? "no-plan" : ""} title={candidate?.reason}>{proposalCurrent ? "No valid plan" : "Unknown"}</td>}</tr>
            {selection && <tr className={stale ? "stale" : "selected"}><th>You selected{stale ? " (out of date)" : ""}</th>
              <td>{selection.assignments?.support ?? "?"}</td><td>{selection.assignments?.batch ?? "?"}</td></tr>}
          </tbody>
        </table>
        <p className="plan-status">{!proposalCurrent ? "The proposed plan is unavailable." : !feasible ? "No plan fits policy and capacity right now."
          : needsChange ? "The proposed plan needs your approval." : "What's running is the proposed plan."}</p>
        <ApprovalStatus live={live} />
      </aside>
    </main>

    <div className="desk-more">
      <details><summary>Jump to a part</summary>
        <div className="jump">{scenes.map((scene, c) => <button key={scene.id} className={c === stage.cue ? "current" : ""} onClick={() => goTo(c, beatsOf(c)[0].index)}>{c + 1}. {scene.title}</button>)}</div>
      </details>
      <details><summary>All demo controls</summary>
        <div className="controls">
          <div>{(["start-traffic", "stop-traffic", "fail-eu-a", "select-plan", "approve-plan", "summaries-europe", "summaries-anywhere", "eu-a-half", "eu-a-full"] as const).map(id => <span key={id}>{actionButton(id)}</span>)}</div>
          {result(Object.keys(actions))}
          <div><button disabled={!canAct} onClick={() => health && void run("submit", [{ path: "/tasks", body: { id: `${health.run_epoch}-support-${crypto.randomUUID()}`, run_epoch: health.run_epoch, workload: "support", content: pick(SUPPORT_QUESTIONS) } }])}>Submit one support request</button>
            <button disabled={!canAct} onClick={() => health && void run("submit", [{ path: "/tasks", body: { id: `${health.run_epoch}-batch-${crypto.randomUUID()}`, run_epoch: health.run_epoch, workload: "batch", content: pick(SUMMARY_REQUESTS) } }])}>Submit one summary request</button>
            {result(["submit"])}</div>
          <DeploymentControl canAct={canAct} run={run} health={health} />
        </div>
      </details>
      <details><summary>Show something on the projector</summary>
        <div className="jump">
          <button onClick={() => show("ops")}>Grafana · Operations</button><button onClick={() => show("plan")}>Grafana · Recovery plan</button>
          {slides.map(slide => <button key={slide.id} onClick={() => show(slide.id)}>{slide.title}</button>)}
          {recaps.map(recap => <button key={recap.id} onClick={() => show(recap.id)}>What Drasi did: {recap.after}</button>)}
          {overdue[0] && <button onClick={() => control.show("request", String(overdue[0].id))}>Oldest late request</button>}
        </div>
        <p className="muted">These open privately on this laptop: <ProductLink href={products.grafana}>Grafana</ProductLink> <ProductLink href={products.langfuse}>Langfuse</ProductLink> <ProductLink href={products.server}>Drasi Server</ProductLink></p>
      </details>
      <details><summary>Recordings (fallback)</summary>
        <p className="muted">A recording plays on the projector, labelled REPLAY · NOT LIVE. Say it's a recording. Live buttons are off while it plays. Back to live returns to the live demo where you left it; if the live demo isn't working, use <b>recording mode</b> instead: every remaining live part then shows its recording as you press Next.</p>
        <div className="jump">{replayClips.map(clip => <button key={clip.scene} disabled={!clip.src} onClick={() => control.update({ replay: clip.scene })}>{clip.scene}{clip.src ? "" : " (not recorded)"}</button>)}
          {stage.replay && <button className="primary" onClick={() => control.update({ replay: null })}>Back to live</button>}</div>
      </details>
      <details><summary>Questions to expect, and glossary</summary>
        <dl className="crib">{questions.map(([q, a]) => <div key={q}><dt>{q}</dt><dd>{a}</dd></div>)}</dl>
        <dl className="crib">{glossary.map(([t, m]) => <div key={t}><dt>{t}</dt><dd>{m}</dd></div>)}</dl>
      </details>
      <details><summary>Before and after the talk</summary>
        <BeforeAfter live={live} canAct={canAct} run={run} feedback={feedback} />
        <p className={templateMissing?.length ? "warning" : "muted"}>{templateMissing === null ? "Checking the Ignite template assets…"
          : templateMissing.length ? `Ignite template assets missing (${templateMissing.join(", ")}): the projector shows a plain background and the system font. ${templateHelp}.`
          : "✓ Ignite template assets present: artwork, survey QR code and Segoe UI."}</p>
      </details>
    </div>
  </div>;
}

// The same four answers as the projector's Drasi tiles, read from the same tile queries.
function Answers({ live }: { live: Live }) {
  const q = live.queries;
  const late = q["overdue-tasks"].rows.filter(row => row.workload === "support").length;
  const actionValue = tileValue(q["tile-action"].rows);
  const action = [["None", "ok"], ["Approve proposed plan", "warn"], ["No valid plan", "bad"], ["At risk", "warn"]][actionValue ?? -1] ?? ["Unknown", ""];
  const safeRow = q["tile-resilience"].rows[0];
  const safeValue = tileValue(q["tile-resilience"].rows);
  const safe = safeValue === 1 ? ["Yes", "ok"] : safeValue === 0 ? [`No: not if ${String(safeRow?.lost || "one")} fails`, "warn"]
    : safeValue === 2 ? ["Already one down", ""] : ["Unknown", ""];
  // No recovery row once attached means nothing to verify (contracts.md §9).
  const recoveryValue = q["tile-recovery"].attached && !q["tile-recovery"].rows.length ? 2 : tileValue(q["tile-recovery"].rows);
  const recovery = [["Awaiting evidence", "warn"], ["Verified", "ok"], ["Nothing to verify", ""]][recoveryValue ?? -1] ?? ["Unknown", ""];
  return <dl className="answers">
    <div><dt>Support overdue</dt><dd className={!q["overdue-tasks"].attached ? "" : late ? "bad" : "ok"}>{q["overdue-tasks"].attached ? late : "Unknown"}</dd></div>
    <div><dt>Action needed</dt><dd className={action[1]}>{action[0]}</dd></div>
    <div><dt>Recovery</dt><dd className={recovery[1]}>{recovery[0]}</dd></div>
    <div><dt>Safe if one fails?</dt><dd className={safe[1]}>{safe[0]}</dd></div>
  </dl>;
}

// The latest approval and its outcome, as Drasi reports it.
function ApprovalStatus({ live }: { live: Live }) {
  const query = live.queries["tile-last-approval"];
  if (!query.attached) return <p className="approval">Approval status unavailable.</p>;
  const latest = lastApproval(query);
  if (!latest) return <p className="approval">No plan approved yet.</p>;
  const recovery = live.queries["tile-recovery"];
  const verifiedNow = latest.state === 3 && tileValue(recovery.rows) === 1;
  const message = latest.state === 1 ? "Approved. Drasi is checking it against the current plan…"
    : latest.state === 2 ? "Applied. Waiting for evidence that recovery worked."
    : verifiedNow ? "Recovery verified ✓"
    : latest.state === 3 ? "Recovery was verified; the evidence has since expired."
    : latest.state === 4 ? `Last approval rejected: ${latest.reason}.`
    : "Approval status unknown.";
  return <p className={`approval ${verifiedNow ? "ok" : ""}`}>{message}</p>;
}

type Run = (action: string, requests: Request[], expectRejected?: boolean) => Promise<void>;
function DeploymentControl({ canAct, run, health }: { canAct: boolean; run: Run; health: Health | null }) {
  const [deployment, setDeployment] = useState("EU-A");
  const [action, setAction] = useState("restore");
  const labels: Record<string, string> = { restore: "Restore", fail: "Fail (HTTP 503)", throttle: "Over quota (HTTP 429)", "pause-reports": "Stop health reports", "resume-reports": "Resume health reports" };
  return <div className="deployment-control"><b>Any deployment</b>
    <select aria-label="Deployment" value={deployment} onChange={event => setDeployment(event.target.value)}>{["EU-A", "EU-B", "US-C"].map(id => <option key={id}>{id}</option>)}</select>
    <select aria-label="Change" value={action} onChange={event => setAction(event.target.value)}>{Object.entries(labels).map(([id, label]) => <option key={id} value={id}>{label}</option>)}</select>
    <button disabled={!canAct} onClick={() => health && void run("deployment", [{ path: "/control", body: { run_epoch: health.run_epoch, deployment, action } }])}>Apply to deployment</button>
  </div>;
}

function BeforeAfter({ live, canAct, run, feedback }: { live: Live; canAct: boolean; run: Run; feedback: Feedback | null }) {
  const [armed, setArmed] = useState(false);
  const health = live.health;
  if (!health) return <p>Waiting for the demo…</p>;
  const route = (id: string) => health.workloads.find(w => w.id === id)?.deployment ?? "?";
  const batch = health.destinations.batch ?? [];
  const starting = health.deployments.every(e => e.mode === "ok" && e.reporting && e.capacity_tpm === 100000)
    && route("support") === "EU-A" && route("batch") === "EU-B" && batch.includes("US-C");
  return <div className="before-after">
    <p className={starting ? "ok" : "warning"}><b>{starting ? "✓ Starting state" : "Not the starting state"}</b> · {health.deployments.map(e => `${e.id} ${modeLabel[e.mode] ?? e.mode} ${e.capacity_tpm / 1000}k`).join(" · ")} · Support → {route("support")} · Summaries → {route("batch")}</p>
    <button disabled={!canAct} onClick={() => setArmed(!armed)}>Reset scenario…</button>
    {armed && <span className="warning"> Restores the starting state and cancels waiting requests. Never during the talk. <button disabled={!canAct} onClick={() => { setArmed(false); void run("reset", [{ path: "/reset", body: { run_epoch: health.run_epoch, revision: health.revision } }]); }}>Confirm reset</button></span>}
    {feedback?.action === "reset" && <p className={`result ${feedback.tone}`}>{feedback.message}</p>}
    <details><summary>Full settings</summary><Json value={{ deployments: health.deployments, workloads: health.workloads, destinations: health.destinations }} /></details>
    <p className="muted">Workloads: {health.workloads.map(w => `${workloadName[w.id] ?? w.id} ${w.demand_tpm / 1000}k`).join(", ")}. Open the projector window from this desk, then move it to the projector and make it full screen.</p>
  </div>;
}
