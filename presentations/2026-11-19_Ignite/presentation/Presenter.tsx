import { useEffect, useState } from "react";
import { scenes, products, slides, timeLabel, duration } from "./scenes";
import { notes } from "./notes";
import { replayClips } from "./replay";
import { isCurrent, post, readDecision, errorText, rowSchema, type Row } from "../demo/ui/src/contracts";
import { Panel, ProductLink, Json } from "../demo/ui/src/views";
import type { Live } from "../demo/ui/src/live";
import type { StageControl } from "../demo/ui/src/stage";

function Clock({ now }: { now: number }) {
  const [started, setStarted] = useState<number | null>(() => {
    const saved = sessionStorage.getItem("ignite-timer");
    const value = saved ? Number(saved) : NaN;
    return Number.isFinite(value) ? value : null;
  });
  const elapsed = started === null ? 0 : Math.max(0, Math.floor((now - started) / 1000));
  const label = `${Math.floor(elapsed / 60).toString().padStart(2, "0")}:${(elapsed % 60).toString().padStart(2, "0")}`;
  return <div className="clock"><strong className={elapsed >= 22 * 60 ? "warning" : ""}>{label}</strong><span>22 min route + 3 min reserve</span>
    <button onClick={() => { const time = Date.now(); sessionStorage.setItem("ignite-timer", String(time)); setStarted(time); }}>{started === null ? "Start timer" : "Restart timer"}</button></div>;
}

function ReadyBanner({ live, now }: { live: Live; now: number }) {
  const ready = !live.error && live.health?.continuous_context === "ready" && live.context?.streams_attached &&
    live.received !== null && now - live.received < 6000;
  return <div className={`live-banner ${ready ? "" : "warning"}`} role="status">
    <strong>{ready ? "READY · The live demo is working" : "NOT READY · Drasi's answers are unavailable or out of date"}</strong>
    <span>{ready ? "Controls are available." : "Controls are disabled. See Troubleshooting in SETUP.md, or use a recording."}</span>
    {live.error && <span className="error-detail">{live.error}</span>}
  </div>;
}

function ApprovalStatus({ live }: { live: Live }) {
  if (!live.decisions) return <p>Approval status unavailable.</p>;
  const command = live.decisions.commands[0];
  if (!command) return <p>No plan approved in this scenario yet.</p>;
  const recovery = live.decisions.recovery[command.id];
  const verifiedNow = live.decisions.streams_attached && recovery?.run_epoch === live.health?.run_epoch &&
    recovery?.applied_revision === live.health?.revision && recovery?.state === "confirmed" &&
    typeof recovery?.expires_ms === "number" && recovery.expires_ms > Date.now();
  const message = command.status === "rejected" ? `Rejected: ${command.error ?? "the plan was out of date"}`
    : command.status === "committed" ? "Approved. Applying the new routes…"
    : verifiedNow ? "Recovery verified: real support requests are succeeding on the new routes."
    : command.status === "recovery_confirmed" ? "Recovery was verified. Not currently confirmed."
    : "New routes applied. Waiting for evidence that recovery worked.";
  return <div className="command-summary">
    <p><strong className={command.status === "rejected" ? "warning" : ""}>{message}</strong></p>
    <details><summary>Details</summary><Json value={live.decisions} /></details>
  </div>;
}

function Controls({ live, now, control }: { live: Live; now: number; control: StageControl }) {
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<{ ok: boolean; message: string; data?: Row } | null>(null);
  const [endpoint, setEndpoint] = useState("EU-A");
  const [action, setAction] = useState("fail");
  const [resetArmed, setResetArmed] = useState(false);
  const [awaitingModel, setAwaitingModel] = useState<{ run_epoch: string; revision: number } | null>(null);
  const health = live.health;
  const candidate = readDecision(live.context, "allocation-results").data;
  const modelObserved = !awaitingModel || (health?.run_epoch === awaitingModel.run_epoch && health.revision >= awaitingModel.revision);
  useEffect(() => { if (awaitingModel && modelObserved) setAwaitingModel(null); }, [awaitingModel, modelObserved]);
  const canAct = !!health && !live.error && !busy && modelObserved && !control.stage.replay && live.received !== null && now - live.received < 6000;
  async function run(label: string, path: string, body: unknown) {
    if (!canAct) return;
    setBusy(true); setFeedback(null);
    try {
      const data = rowSchema.parse(await post(path, body));
      if (typeof data.run_epoch === "string" && typeof data.revision === "number") {
        setAwaitingModel({ run_epoch: data.run_epoch, revision: data.revision });
      }
      if (path === "/reset") control.update({ selection: null });
      setFeedback({ ok: true, message: `${label}: done.`, data });
    } catch (error) {
      setFeedback({ ok: false, message: `${label} failed: ${errorText(error)}. Check the approval status below before trying again.` });
    } finally { setBusy(false); setResetArmed(false); }
  }
  function submit(workload: string) {
    if (!health) return;
    void run(`Submit ${workload}`, "/tasks", {
      id: `${health.run_epoch}-${workload}-${crypto.randomUUID()}`, run_epoch: health.run_epoch,
      workload, content: workload === "support" ? "Synthetic support case: explain the next troubleshooting step." : "Summarize this synthetic public document about service operations.",
    });
  }
  const selection = control.stage.selection;
  const traceUrl = typeof feedback?.data?.trace_url === "string" && feedback.data.trace_url.startsWith("http://localhost:13001/project/ignite-probe/traces/") ? feedback.data.trace_url : null;
  return <div className="controls-grid">
    <Panel title="Traffic and faults">
      <p className="muted">These buttons change the live demo.</p>
      <fieldset disabled={!canAct}>
        <div className="actions">
          <button disabled={health?.traffic?.running} onClick={() => health && void run("Start demo traffic", "/traffic", { run_epoch: health.run_epoch, running: true })}>Start demo traffic</button>
          <button disabled={!health?.traffic?.running} onClick={() => health && void run("Stop demo traffic", "/traffic", { run_epoch: health.run_epoch, running: false })}>Stop demo traffic</button>
          <span>{health?.traffic?.running ? "Traffic running: one support and one summary request every 5 seconds" : "Traffic stopped"}</span>
        </div>
        {health?.traffic?.error && <p className="warning">{health.traffic.error}</p>}
        <div className="actions"><button onClick={() => submit("support")}>Submit support request</button><button onClick={() => submit("batch")}>Submit batch request</button></div>
        <div className="actions"><label>Deployment<select value={endpoint} onChange={event => setEndpoint(event.target.value)}>{["EU-A", "EU-B", "US-C"].map(id => <option key={id}>{id}</option>)}</select></label>
          <label>Action<select value={action} onChange={event => setAction(event.target.value)}>{["fail", "throttle", "restore", "pause-reports", "resume-reports"].map(item => <option key={item}>{item}</option>)}</select></label>
          <button onClick={() => health && void run(`${action} ${endpoint}`, "/control", { run_epoch: health.run_epoch, revision: health.revision, endpoint, action })}>Apply to deployment</button></div>
      </fieldset>
      <div className="actions"><button disabled={!canAct || !candidate?.valid || !isCurrent(candidate, health, now)} onClick={() => candidate && control.select({
        command_id: crypto.randomUUID(), candidate_id: candidate.candidate_id, basis: candidate.basis, run_epoch: candidate.run_epoch, revision: candidate.revision,
      })}>Select proposed plan</button>
        <button className="primary" disabled={!canAct || !selection} onClick={() => selection && void run("Approve selected plan", "/approvals", {
          command_id: selection.command_id, candidate_id: selection.candidate_id, basis: selection.basis,
        })}>Approve selected plan</button></div>
      {selection && <div className="retained">{selection.basis !== candidate?.basis
        ? <p className="warning"><b>STALE:</b> Drasi's proposal has changed since you selected it. Approving this selection will be rejected.</p>
        : <p><b>Plan selected.</b> It matches Drasi's current proposal.</p>}
        <small>Approving again retries the same approval. Selecting again starts a new one.</small></div>}
      {!canAct && <p className="warning">{control.stage.replay ? "A recording is playing: live controls are disabled." : busy ? "Working…" : !modelObserved ? "Waiting for the change to take effect…" : "Controls are disabled: the demo is not ready."}</p>}
      {!modelObserved && !busy && health && <button onClick={() => setAwaitingModel(null)}>Stop waiting</button>}
    </Panel>
    <Panel title="Scenario settings">
      <p className="muted">These change the policy and capacity. Drasi works out the results.</p>
      <fieldset disabled={!canAct || !health?.model.policy}>
        <div className="fixture-buttons">
          <button onClick={() => {
            if (!health?.model.policy) return;
            const model = structuredClone(health.model);
            model.policy!.destinations.batch = ["EU-A", "EU-B"];
            void run("Restrict batch destinations", "/configuration", { run_epoch: health.run_epoch, revision: health.revision, model });
          }}>Batch policy: Europe only</button>
          <button onClick={() => {
            if (!health?.model.policy) return;
            const model = structuredClone(health.model);
            model.policy!.destinations.batch = ["EU-A", "EU-B", "US-C"];
            void run("Restore batch destinations", "/configuration", { run_epoch: health.run_epoch, revision: health.revision, model });
          }}>Batch policy: Europe + US</button>
          <button onClick={() => {
            if (!health) return;
            const model = structuredClone(health.model);
            for (const endpoint of model.endpoints) { endpoint.mode = "ok"; endpoint.reporting = true; if (endpoint.id === "EU-B") endpoint.budget_tpm = 50000; }
            for (const workload of model.workloads) workload.destination = workload.id === "support" ? "EU-A" : "US-C";
            void run("Apply healthy-fragile fixture inputs", "/configuration", { run_epoch: health.run_epoch, revision: health.revision, model });
          }}>Healthy-fragile inputs · EU-B 50k</button>
          <button onClick={() => {
            if (!health) return;
            const model = structuredClone(health.model);
            const endpoint = model.endpoints.find(endpoint => endpoint.id === "EU-B");
            if (endpoint) endpoint.budget_tpm = 100000;
            void run("Restore EU-B budget", "/configuration", { run_epoch: health.run_epoch, revision: health.revision, model });
          }}>Restore EU-B capacity · 100k</button>
        </div>
      </fieldset>
      <details><summary>Show current settings</summary><Json value={health?.model ?? "unknown"} /></details>
      <button disabled={!canAct} onClick={() => setResetArmed(!resetArmed)}>Reset scenario...</button>
      {resetArmed && <div className="warning"><p>Reset restores the starting state and cancels waiting requests. History is kept. Never reset during the talk.</p><button disabled={!canAct} onClick={() => health && void run("Reset scenario", "/reset", { run_epoch: health.run_epoch, revision: health.revision })}>Confirm reset</button><button onClick={() => setResetArmed(false)}>Cancel reset</button></div>}
    </Panel>
    {feedback && <section className={`action-feedback ${feedback.ok ? "" : "warning"}`} role="status"><strong>{feedback.message}</strong>{traceUrl && <ProductLink href={traceUrl}>Open returned task trace</ProductLink>}<details><summary>Details</summary><Json value={feedback.data ?? feedback.message} /></details></section>}
  </div>;
}

export default function Presenter({ live, now, control }: { live: Live; now: number; control: StageControl }) {
  const scene = scenes[control.stage.index];
  const note = notes[scene.id];
  const ready = live.health?.continuous_context === "ready" && !live.error && live.context?.streams_attached;
  const [confirmLive, setConfirmLive] = useState(false);
  return <div className="presenter">
    <header className="app-header"><b>PRESENTER DESK · DO NOT PROJECT</b><ProductLink href="/present">Open slides</ProductLink></header>
    <ReadyBanner live={live} now={now} />
    <div className="presenter-top">
      <Panel title={`Cue ${control.stage.index + 1} of ${scenes.length} · ${scene.title}`}>
        <Clock now={now} />
        <p><b>Audience sees: {scene.tool}</b> · {duration(scene.minutes)}</p>
        <div className="actions"><button onClick={() => control.advance(-1)}>Previous cue</button><button className="primary" onClick={() => control.advance(1)}>Next cue</button></div>
        <label>Cue<select aria-label="Cue" value={control.stage.index} onChange={event => control.go(Number(event.target.value))}>{scenes.map((item, index) => <option key={item.id} value={index}>{timeLabel(item.start)} · {item.title}</option>)}</select></label>
        <label><input type="checkbox" checked={control.stage.short} onChange={event => control.update({ short: event.target.checked })} /> Short on time: skip the policy change</label>
        <p className="muted">Changing cue only changes these notes. It never changes the projected screen or the demo.</p>
        <div className="actions"><ProductLink href={products.grafana}>Grafana · Operations</ProductLink><ProductLink href={products.decisions}>Grafana · Policy and recovery</ProductLink><ProductLink href={products.langfuse}>Langfuse</ProductLink><ProductLink href={products.server}>Drasi Server</ProductLink></div>
        <details><summary>Show a slide in the projected window</summary><div className="actions">{slides.map(slide => <button key={slide.id} onClick={() => control.showSlide(slide.id)}>{slide.title}</button>)}</div></details>
      </Panel>
      <Panel title="What to do">
        <blockquote>{note.say}</blockquote>
        <dl className="notes"><dt>Do</dt><dd>{note.do}</dd><dt>Point at</dt><dd>{note.point}</dd><dt>What happens</dt><dd>{note.happens}</dd><dt>If it breaks</dt><dd>{note.ifBroken}</dd><dt>Next</dt><dd>{note.next}</dd></dl>
      </Panel>
    </div>
    <Controls live={live} now={now} control={control} />
    <Panel title="Recordings (fallback)">
      <p className="muted">Choose a recording, then open the slides in the projected window (Grafana's Closing slide link). The recording plays there, labelled REPLAY · NOT LIVE. Live controls are disabled while a recording is selected.</p>
      <div className="actions">{replayClips.map(clip => <button key={clip.scene} onClick={() => { control.update({ replay: clip.scene }); setConfirmLive(false); }}>{clip.scene}: {clip.src ? "recorded replay" : "media unavailable"}</button>)}</div>
      {control.stage.replay && <><p className="warning">A recording is selected. Live controls are disabled.</p><label><input type="checkbox" checked={confirmLive} onChange={event => setConfirmLive(event.target.checked)} /> I've checked the live demo is ready (the banner at the top says READY)</label><button disabled={!ready || !confirmLive} onClick={() => { control.update({ replay: null }); setConfirmLive(false); }}>Return to verified live state</button></>}
    </Panel>
    <Panel title="Approval status"><ApprovalStatus live={live} /></Panel>
  </div>;
}
