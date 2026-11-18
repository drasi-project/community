// Runs the whole talk through the presenter desk, exactly as a presenter would: press each
// beat's buttons, press Next, and capture what the projector and the desk show.
//
//   node ui/scripts/walkthrough.mjs [capture|record] [output-folder]
//
// capture (default): a screenshot of the projector and the desk at every beat.
// record: also records the projector during the failure, recovery and resilience parts and
//         the policy beats, and replaces the fallback recordings in ui/public/replay.
// Run from demo/ with .env loaded, against the running demo. It resets the demo at the start
// and the end, and runs about 12 minutes.
import { chromium } from "@playwright/test";
import fs from "node:fs/promises";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";
import { settings, url } from "../settings.mjs";

const mode = process.argv[2] ?? "capture";
const ui = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const output = path.resolve(process.argv[3] ?? path.join(ui, "evidence", "walkthrough"));
const base = url("UI", "localhost"); // localhost, so the framed Mattermost gets its cookies
const api = async (route, body) => {
  const response = await fetch(`${url("WORKLOAD")}${route}`, body ? { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) } : undefined);
  return response.json();
};
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(label, check, seconds = 120) {
  for (let i = 0; i < seconds; i++) { if (await check()) return; await sleep(1000); }
  throw new Error(`Timed out waiting for: ${label}`);
}
const metric = async (query) => {
  const r = await (await fetch(`${url("PROMETHEUS", "localhost")}/api/v1/query?query=${encodeURIComponent(query)}`)).json();
  return r.data.result.map(x => ({ labels: x.metric, value: Number(x.value[1]) }));
};
// Drasi's answers as Grafana reads them: the Prometheus reaction's series (plan/contracts.md section 9).
const drasi = async (name, labels = "") => (await metric(`${name}${labels}`))[0]?.value;
const action = () => drasi("ignite_drasi_recommended_action");
const failover = () => drasi("ignite_drasi_failover_readiness");
const lastApproval = () => drasi("ignite_drasi_last_approval_state");

// What each beat waits for before its screenshot, keyed by "<part>/<beat>". Approving beats press
// the desk's backup, which presses Approve on the newest proposal card, as a click on it would.
const waits = {
  "normal/0": async () => until("traffic history", async () => (await api("/health")).traffic?.running, 10).then(() => sleep(15000)),
  "failure/0": () => until("Recommended action: the change", async () => await action() === 1, 60),
  "failure/2": () => sleep(5000),
  "failure/3": () => until("errors and alert", async () => await drasi("ignite_drasi_overdue_requests", '{workload="support"}') > 0, 60).then(() => sleep(15000)),
  "who/0": () => sleep(3000),
  "who/1": () => sleep(3000),
  "recovery/1": () => until("no valid plan", async () => await action() === 2, 30).then(() => sleep(6000)),
  "recovery/2": () => until("rejected", async () => await lastApproval() === 4, 30).then(() => sleep(6000)),
  "recovery/3": () => until("new proposal", async () => await action() === 1, 30).then(() => sleep(6000)),
  "recovery/5": () => until("applied", async () => [2, 3].includes(await lastApproval()), 30).then(() => sleep(6000)),
  "recovery/6": () => until("verified", async () => await drasi("ignite_drasi_change_status") === 4, 150).then(() => sleep(6000)),
  "recovery/7": () => sleep(30000),
  "recovery/8": () => sleep(3000),
  "resilience/0": () => until("not safe", async () => await failover() === 0, 30).then(() => sleep(8000)),
  "resilience/1": () => sleep(4000),
  "resilience/3": () => until("safe again", async () => await failover() === 1, 30).then(() => sleep(8000)),
};
const recordings = { failure: ["failure/0", "failure/4"], policy: ["recovery/1", "recovery/3"], recovery: ["recovery/5", "recovery/8"], resilience: ["resilience/0", "resilience/3"] };

await fs.mkdir(output, { recursive: true });
// The projector and recording pages run in background tabs: keep Chromium from throttling their
// timers, so Grafana keeps refreshing as it would in a visible projector window.
const browser = await chromium.launch({ args: ["--disable-background-timer-throttling",
  "--disable-backgrounding-occluded-windows", "--disable-renderer-backgrounding"] });
// One browser context, so the desk and the projector windows can talk to each other. The projector
// and its recordings are 1920x1080 (16:9), the size Ignite asks for.
const context = await browser.newContext({ viewport: { width: 1920, height: 1080 },
  ...(mode === "record" ? { recordVideo: { dir: path.join(output, "raw"), size: { width: 1920, height: 1080 } } } : {}) });
const desk = await context.newPage();
await desk.setViewportSize({ width: 1440, height: 900 });
const shots = [];
async function shot(name) {
  await desk.evaluate(() => window.scrollTo(0, 0));
  await desk.screenshot({ path: path.join(output, `${name}-desk.png`) });
  await projector.screenshot({ path: path.join(output, `${name}-projector.png`) });
  shots.push(name);
  console.log("captured", name);
}

// Start from the starting state, with the desk on the first beat.
const health = await api("/health");
if (health.traffic?.running) await api("/traffic", { run_epoch: health.run_epoch, running: false });
const before = await api("/health");
await api("/reset", { run_epoch: before.run_epoch, revision: before.revision });
await desk.addInitScript(key => { if (!sessionStorage.getItem("seeded")) { localStorage.removeItem(key); sessionStorage.setItem("seeded", "1"); } }, settings.IGNITE_STORAGE_KEY);
await desk.goto(`${base}/presenter`);
await until("desk READY", async () => (await desk.locator(".badge", { hasText: /^READY$/ }).count()) > 0, 60);
// The projector is its own window; the desk drives it through the browser's channel.
const projector = await context.newPage();
await desk.getByText("Jump to a part").click();
await desk.getByRole("button", { name: "1. Title" }).click();
await desk.getByText("Jump to a part").click();
let recordingPage = null;
const evidence = { started: new Date().toISOString(), mode, beats: [], clips: [] };

let index = 0, clip = null;
for (;;) {
  const title = (await desk.locator(".now-head h1").innerText()).replace(/\s+\d.*$/, "");
  const part = (await desk.evaluate(key => JSON.parse(localStorage.getItem(key)), settings.IGNITE_STORAGE_KEY));
  const sceneId = ["title", "concepts", "drasi", "normal", "failure", "who", "recovery", "resilience", "closing"][part.cue];
  const key = `${sceneId}/${part.beat}`;
  if (index === 0) {
    // Sign the projector's browser in to Mattermost as oncall, as the presenter does before the talk.
    await projector.goto(`${url("WORKLOAD", "localhost")}/mattermost/sign-in`);
    await projector.goto(`${base}/present`);
  }
  for (const [name, [from]] of Object.entries(recordings)) if (mode === "record" && key === from) {
    recordingPage = await context.newPage();
    await recordingPage.goto(`${base}/present`);
    clip = { name, started: Date.now(), page: recordingPage };
  }
  // Press each of the beat's buttons in order, as the presenter would; some enable only after the previous one.
  const buttons = desk.locator(".now-actions button.action");
  for (let i = 0; i < await buttons.count(); i++) {
    const button = buttons.nth(i);
    if ((await button.innerText()).includes("✓")) continue;
    await until("button enabled", () => button.isEnabled(), 30);
    await button.click();
    await desk.waitForTimeout(1500);
  }
  await waits[key]?.();
  await desk.waitForTimeout(part.view === "ops" || part.view === "incident" ? 6000 : 1500);
  const name = `${String(index).padStart(2, "0")}-${key.replace("/", "-")}`;
  await shot(name);
  evidence.beats.push({ name, part: title, view: part.view, say: await desk.locator(".say").innerText(), result: await desk.locator(".now .result").innerText().catch(() => null) });
  if (clip && recordings[clip.name][1] === key) {
    const video = clip.page.video();
    await clip.page.close();
    const file = path.join(output, `${clip.name}.webm`);
    await video.saveAs(file);
    evidence.clips.push({ scene: clip.name, file, seconds: (Date.now() - clip.started) / 1000 });
    clip = null;
  }
  const next = desk.getByRole("button", { name: "Next ▶" });
  if (await next.isDisabled()) break;
  await next.click();
  index++;
  await desk.waitForTimeout(800);
}
const end = await api("/health");
if (end.traffic?.running) await api("/traffic", { run_epoch: end.run_epoch, running: false });
const last = await api("/health");
await api("/reset", { run_epoch: last.run_epoch, revision: last.revision });
await fs.writeFile(path.join(output, "walkthrough.json"), JSON.stringify(evidence, null, 2));

if (mode === "record") {
  const media = path.join(ui, "public", "replay");
  const captures = [];
  for (const item of evidence.clips) {
    const bytes = await fs.readFile(item.file);
    await fs.copyFile(item.file, path.join(media, `${item.scene}.webm`));
    const capturedAt = new Date().toISOString();
    captures.push({ scene: item.scene, capturedAt, runEpoch: last.run_epoch, surface: "grafana", duration_seconds: item.seconds,
      src: `/replay/${item.scene}.webm`, sha256: crypto.createHash("sha256").update(bytes).digest("hex"), bytes: bytes.length,
      provenance: `Recorded from the live demo ${capturedAt}. Simulated deployments; real requests, monitoring and Drasi.` });
  }
  await fs.writeFile(path.join(media, "captures.json"), JSON.stringify(captures, null, 2) + "\n");
  console.log("recordings replaced; run npm run build so the app serves them");
}
await browser.close();
console.log(`done: ${shots.length} beats captured in ${output}`);
