import { test, expect } from "@playwright/test";
import { applyFrame } from "../src/live";
import { isCurrent, decisionSchema, healthSchema } from "../src/contracts";
import { scenes, slides, products, duration, timeLabel } from "../../../presentation/scenes";
import fs from "node:fs";
import { notes } from "../../../presentation/notes";
import { replayClips } from "../../../presentation/replay";

const fixture = "http://127.0.0.1:18582";
test.beforeEach(async ({ request }) => { await request.post(`${fixture}/test/reset`); });

test("three audience slides frame a 22-minute private cue list", () => {
  expect(slides.map(s => s.id)).toEqual(["title", "concepts", "closing"]);
  expect(scenes.reduce((sum, s) => sum + s.minutes, 0)).toBe(22);
  for (const scene of scenes) {
    for (const value of Object.values(notes[scene.id])) expect(value.trim()).not.toBe("");
  }
  const toolMinutes = scenes.filter(s => s.url.startsWith("http://localhost:13000")).reduce((n, s) => n + s.minutes, 0);
  expect(toolMinutes).toBeGreaterThanOrEqual(15);
});

test("title contains no concepts, dashboards, instructions or backend requests", async ({ page, request }) => {
  const loaded: string[] = [];
  page.on("request", r => loaded.push(r.url()));
  await page.addInitScript(() => localStorage.setItem("ignite-presentation-stage-v1", JSON.stringify({
    index: 5, vertical: 0, view: "operations", display: "demo", replay: null, selection: null, short: false,
  })));
  await page.goto("/present");
  await expect(page.locator(".title-slide")).toBeVisible();
  await expect(page.locator(".title-slide h1")).toContainText("Monitoring");
  const visible = await page.locator("body").innerText();
  for (const text of ["Sources", "Continuous queries", "Start with measured", "Open real", "Candidate", "Context ready", "Full demo"]) {
    expect(visible).not.toContain(text);
  }
  expect(loaded.some(url => /\/api\/|\/server\/|\/(notes|Presenter|Runbook)-/.test(url))).toBe(false);
  expect((await (await request.get(`${fixture}/test/state`)).json()).mutations).toEqual([]);
});

test("second slide leads with the scenario, then Drasi, and navigates directly to actual Grafana", async ({ page, request }) => {
  await page.route("http://localhost:13000/**", route => route.fulfill({ contentType: "text/html", body: "<h1>TEST Grafana navigation target</h1>" }));
  await page.goto("/present");
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("heading", { name: "Keep customer support running", exact: true })).toBeVisible();
  await expect(page.getByText("When EU-A fails, customers stop getting answers.", { exact: true })).toBeVisible();
  await expect(page.locator(".scenario-question li")).toHaveText(["Who is affected right now?", "What's the fastest fix that stays within company policy?", "How do we know it's really fixed?"]);
  for (const role of ["Sources", "Continuous models", "Reactions"]) await expect(page.locator(".drasi-roles b", { hasText: role })).toBeVisible();
  const link = page.getByRole("link", { name: "Live demo · Grafana", exact: false });
  await expect(link).toHaveAttribute("href", products.grafana);
  await link.click();
  await expect(page).toHaveURL(products.grafana);
  expect((await (await request.get(`${fixture}/test/state`)).json()).mutations).toEqual([]);
});

test("closing deep link shows only the outro and useful resource", async ({ page }) => {
  await page.goto("/present?slide=closing");
  await expect(page.getByRole("heading", { name: "From change to action", exact: true })).toBeVisible();
  await expect(page.locator(".closing-copy li")).toHaveCount(3);
  await expect(page.locator(".closing-lead")).toContainText("Keep your observability tools.");
  await expect(page.locator(".live-banner,.view-tabs,.controls-grid")).toHaveCount(0);
});

test("private cues do not change projected slides; explicit slide selection does", async ({ page: desk, request }) => {
  await desk.goto("/presenter");
  const audience = await desk.context().newPage();
  await audience.goto("/present");
  await expect(audience.locator(".title-slide")).toBeVisible();
  await desk.getByRole("button", { name: "Next cue", exact: true }).click();
  await expect(desk.getByText(notes.concepts.say, { exact: true })).toBeVisible();
  await expect(audience.locator(".title-slide")).toBeVisible();
  await desk.getByText("Show a slide in the projected window", { exact: true }).click();
  await desk.getByRole("button", { name: "The scenario", exact: true }).click();
  await expect(audience.locator(".concepts-slide")).toBeVisible();
  expect((await (await request.get(`${fixture}/test/state`)).json()).mutations).toEqual([]);
});

test("preview cannot load private notes or controls", async ({ page, request }) => {
  await page.goto("/presenter?preview=1");
  await expect(page.getByText("Read-only preview", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Start demo traffic" })).toHaveCount(0);
  await page.keyboard.press("ArrowRight");
  expect((await (await request.get(`${fixture}/test/state`)).json()).mutations).toEqual([]);
});

test("presenter desk shows each cue's plain notes and there is no runbook page", async ({ page }) => {
  await page.goto("/presenter");
  await expect(page.getByText(notes.title.say, { exact: true })).toBeVisible();
  await expect(page.getByText("Audience sees: Title slide", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Next cue", exact: true }).click();
  await page.getByRole("button", { name: "Next cue", exact: true }).click();
  await expect(page.getByText(notes.normal.happens, { exact: true })).toBeVisible();
  await page.goto("/runbook");
  await expect(page.locator(".runbook-scene")).toHaveCount(0);
});

// PRESENTER.md's run of show and script are generated from notes.ts.
// Regenerate with: npm run sync-presenter
function presenterScript() {
  const rows = scenes.map(scene => `| ${timeLabel(scene.start)} | ${scene.title} | ${scene.tool} | ${notes[scene.id].summary} |`);
  const run = ["| Time | Cue | Audience sees | You |", "|---|---|---|---|", ...rows, "| 22:00 | Questions | Closing slide | Take questions |"].join("\n");
  const cues = scenes.map((scene, index) => {
    const note = notes[scene.id];
    return [`### ${index + 1}. ${scene.title} (${duration(scene.minutes)}) · ${scene.tool}`, "",
      `- **Say:** ${note.say}`, `- **Do:** ${note.do}`, `- **Point at:** ${note.point}`,
      `- **What happens:** ${note.happens}`, `- **If it breaks:** ${note.ifBroken}`, `- **Next:** "${note.next}"`].join("\n");
  }).join("\n\n");
  return `<!-- generated from presentation/notes.ts: do not edit; run npm run sync-presenter in demo/ui -->\n\n## Run of show\n\n${run}\n\n## The script\n\nThe same notes appear cue by cue on the presenter desk, under **What to do**. The wording is a guide, not a script to read aloud.\n\n${cues}\n\n<!-- end generated -->`;
}

test("PRESENTER.md matches the presenter desk notes", () => {
  const file = new URL("../../../PRESENTER.md", import.meta.url);
  const doc = fs.readFileSync(file, "utf8");
  const start = doc.indexOf("<!-- generated from presentation/notes.ts");
  const endMarker = "<!-- end generated -->";
  const end = doc.indexOf(endMarker);
  expect(start).toBeGreaterThan(0);
  expect(end).toBeGreaterThan(start);
  const current = doc.slice(start, end + endMarker.length);
  if (process.env.UPDATE_PRESENTER_DOC === "1") fs.writeFileSync(file, doc.replace(current, presenterScript()));
  else expect(current).toBe(presenterScript());
});

test("traffic starts only through an explicit private action", async ({ page, request }) => {
  await page.goto("/presenter");
  expect((await (await request.get(`${fixture}/test/state`)).json()).mutations).toEqual([]);
  await page.getByRole("button", { name: "Start demo traffic", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stop demo traffic", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Stop demo traffic", exact: true }).click();
  const mutations = (await (await request.get(`${fixture}/test/state`)).json()).mutations;
  expect(mutations.map((m: { path: string }) => m.path)).toEqual(["/traffic", "/traffic"]);
  expect(mutations.map((m: { input: { running: boolean } }) => m.input.running)).toEqual([true, false]);
});

test("stale candidate retains original approval basis", async ({ page, request }) => {
  await page.goto("/presenter");
  await page.getByRole("button", { name: "Select proposed plan" }).click();
  await page.getByRole("button", { name: "Batch policy: Europe only", exact: true }).click();
  await expect(page.getByText("proposal has changed since you selected it", { exact: false })).toBeVisible();
  await page.getByRole("button", { name: "Approve selected plan", exact: true }).click();
  await expect(page.locator(".action-feedback > strong")).toContainText("409");
  const state = await (await request.get(`${fixture}/test/state`)).json();
  expect(state.mutations[1].input.basis).toBe("TEST-FIXTURE-BASIS-1");
});

test("approval retries retain command identity and do not imply recovery", async ({ page, request }) => {
  await page.goto("/presenter");
  await page.getByRole("button", { name: "Select proposed plan" }).click();
  await page.getByRole("button", { name: "Approve selected plan", exact: true }).click();
  await expect(page.getByText("Approve selected plan: done.", { exact: true })).toBeVisible();
  const retry = page.waitForResponse(response => response.url().endsWith("/api/approvals") && response.request().method() === "POST");
  await page.getByRole("button", { name: "Approve selected plan", exact: true }).click();
  expect((await (await retry).json()).duplicate).toBe(true);
  const state = await (await request.get(`${fixture}/test/state`)).json();
  expect(state.mutations[0].input.command_id).toBe(state.mutations[1].input.command_id);
  await expect(page.getByText("Approved. Applying the new routes…", { exact: true })).toBeVisible();
  await expect(page.getByText("Recovery verified", { exact: false })).toHaveCount(0);
});

test("replay is labeled and private actions stay disabled until live state is checked", async ({ page: desk, request }) => {
  await desk.goto("/presenter");
  const clip = replayClips.find(c => c.scene === "failure")!;
  await desk.getByRole("button", { name: `failure: ${clip.src ? "recorded replay" : "media unavailable"}`, exact: true }).click();
  await expect(desk.getByRole("button", { name: "Start demo traffic" })).toBeDisabled();
  await expect(desk.getByRole("button", { name: "Submit support request" })).toBeDisabled();
  const audience = await desk.context().newPage();
  await audience.goto("/present");
  await expect(audience.getByText("REPLAY · NOT LIVE", { exact: true })).toBeVisible();
  if (clip.src) await expect(audience.locator("video")).toHaveAttribute("src", clip.src);
  else await expect(audience.getByText("No current recording is available.", { exact: true })).toBeVisible();
  await expect(desk.getByRole("button", { name: "Return to verified live state" })).toBeDisabled();
  await desk.getByLabel("I've checked the live demo is ready", { exact: false }).check();
  await desk.getByRole("button", { name: "Return to verified live state" }).click();
  await expect(audience.locator(".title-slide")).toBeVisible();
  expect((await (await request.get(`${fixture}/test/state`)).json()).mutations).toEqual([]);
});

test("expired/disconnected evidence disables current candidate selection", async ({ page, request }) => {
  await page.goto("/presenter");
  await expect(page.getByRole("button", { name: "Select proposed plan" })).toBeEnabled();
  await request.post(`${fixture}/test/expire`);
  await expect(page.getByRole("button", { name: "Select proposed plan" })).toBeDisabled();
  await request.post(`${fixture}/test/offline`, { data: { value: true } });
  await expect(page.getByRole("button", { name: "Start demo traffic" })).toBeDisabled();
  await expect(page.getByText("NOT READY · Drasi's answers are unavailable or out of date", { exact: true })).toBeVisible();
});

test("reset remains explicit and cue navigation sends no writes", async ({ page, request }) => {
  await page.goto("/presenter");
  await page.getByRole("button", { name: "Reset scenario...", exact: true }).click();
  await page.getByRole("button", { name: "Cancel reset" }).click();
  await page.getByRole("button", { name: "Next cue", exact: true }).click();
  expect((await (await request.get(`${fixture}/test/state`)).json()).mutations).toEqual([]);
  await page.getByRole("button", { name: "Reset scenario...", exact: true }).click();
  await page.getByRole("button", { name: "Confirm reset" }).click();
  expect((await (await request.get(`${fixture}/test/state`)).json()).mutations).toEqual([{ path: "/reset", input: { run_epoch: "TEST-FIXTURE-EPOCH", revision: 1 } }]);
});

test("local proxy rejects cross-origin commands and Server admin writes", async ({ request }) => {
  expect((await request.post("/api/reset", { headers: { Origin: "https://not-local.example" }, data: {} })).status()).toBe(403);
  expect((await request.post("/server/api/v1/instances/ignite/queries", { data: {} })).status()).toBe(403);
});

test("snapshot-first reducer preserves explicit delta semantics", () => {
  const snapshot = applyFrame({ old: { id: "old" } }, "snapshot", [{ id: "a" }], false);
  expect(snapshot).toEqual({ a: { id: "a" } });
  expect(applyFrame(snapshot, "change", { results: [{ type: "DELETE", data: { id: "a" } }] }, true)).toEqual({});
  expect(() => applyFrame({}, "change", { results: [] }, false)).toThrow("before initial");
});

test("current-state guard rejects mixed epochs and expired reports", async ({ request }) => {
  const health = healthSchema.parse(await (await request.get(`${fixture}/health`)).json());
  const d = decisionSchema.parse((await (await request.get(`${fixture}/decisions`)).json()).allocation);
  expect(isCurrent(d, health, Date.now())).toBe(true);
  expect(isCurrent({ ...d, run_epoch: "other" }, health, Date.now())).toBe(false);
  expect(isCurrent(d, health, Date.now() + 7200000)).toBe(false);
});

for (const size of [{ width: 1280, height: 720 }, { width: 1920, height: 1080 }]) {
  test(`all three audience slides fit ${size.width}x${size.height}`, async ({ page }) => {
    await page.setViewportSize(size);
    for (const slide of slides) {
      await page.goto(`/present?slide=${slide.id}`);
      const content = page.locator(`[data-slide="${slide.id}"]`);
      await expect(content).toBeVisible();
      const bounds = await content.evaluate(element => ({
        w: element.clientWidth, sw: element.scrollWidth, h: element.clientHeight, sh: element.scrollHeight,
      }));
      expect(bounds.sw).toBeLessThanOrEqual(bounds.w + 2);
      expect(bounds.sh).toBeLessThanOrEqual(bounds.h + 2);
      await page.screenshot({ path: `evidence/audience-${size.width}-${slide.id}.png` });
    }
  });
}
