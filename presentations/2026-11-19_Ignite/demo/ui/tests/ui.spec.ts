import { settings, url, port } from "../settings.mjs";
import { test, expect, type Page } from "@playwright/test";
import { applyFrame } from "../src/live";
import { isCurrent, decisionSchema, healthSchema } from "../src/contracts";
import { scenes, slides, recaps, allSlides, dashboards } from "../../../presentation/scenes";
import { notes } from "../../../presentation/notes";
import { presenterScript } from "../../../presentation/script";
import { replayClips } from "../../../presentation/replay";
import { displayQuery } from "../../../presentation/QuerySlide";
import fs from "node:fs";

const fixture = url("UI_FIXTURE");
const stageKey = settings.IGNITE_STORAGE_KEY;
test.beforeEach(async ({ request }) => { await request.post(`${fixture}/test/reset`); });
const mutations = async (request: import("@playwright/test").APIRequestContext) => (await (await request.get(`${fixture}/test/state`)).json()).mutations;
// Open the desk on a given part and beat.
async function desk(page: Page, cue: string, beat = 0, extra: object = {}) {
  const index = scenes.findIndex(scene => scene.id === cue);
  await page.addInitScript(([key, value]) => { if (!sessionStorage.getItem("seeded")) { localStorage.setItem(key, value); sessionStorage.setItem("seeded", "1"); } },
    [stageKey, JSON.stringify({ cue: index, beat, view: notes[cue as keyof typeof notes].beats[beat].show, trace: null, replay: null, selection: null, short: false, ...extra })]);
  await page.goto("/presenter");
  await expect(page.getByRole("heading", { level: 1 })).toContainText(scenes[index].title);
}
const stubGrafana = (page: Page) => page.route(`http://localhost:${port("GRAFANA")}/**`, route => route.fulfill({ contentType: "text/html", body: "<h1>TEST Grafana</h1>" }));

test("the talk is 22 minutes and every beat says something and shows something", () => {
  expect(scenes.reduce((sum, s) => sum + s.minutes, 0)).toBe(22);
  for (const scene of scenes) {
    const cue = notes[scene.id];
    expect(cue.beats.length).toBeGreaterThan(0);
    for (const beat of cue.beats) {
      expect(beat.say.trim()).not.toBe("");
      expect(["ops", "plan", "request", ...allSlides.map(s => s.id)]).toContain(beat.show);
    }
    expect(cue.ifBroken.trim()).not.toBe("");
  }
  for (const recap of recaps) expect(Object.values(notes).some(cue => cue.beats.some(beat => beat.show === recap.id))).toBe(true);
  expect(slides.map(s => s.id)).toEqual(["title", "concepts", "drasi", "query", "closing"]);
});

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

test("the query slide shows exactly the query Drasi runs", () => {
  const file = fs.readFileSync(new URL("../../drasi/overdue-tasks.cypher", import.meta.url), "utf8");
  const words = (text: string) => text.replace(/\s+/g, " ").trim();
  expect(words(displayQuery.map(([line]) => line).join(" "))).toBe(words(file));
});

test("Next on the desk moves through beats and changes what the projector shows", async ({ page, request }) => {
  await stubGrafana(page);
  await desk(page, "title");
  const projector = await page.context().newPage();
  await stubGrafana(projector);
  await projector.goto("/present");
  await expect(projector.locator(".title-slide")).toBeVisible();
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(projector.locator('[data-slide="concepts"]')).toBeVisible();
  for (let i = 0; i < 5; i++) await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Healthy service");
  await expect(projector.locator("iframe.stage-dashboard").first()).toBeVisible();
  await expect(projector.locator("iframe.stage-dashboard").first()).toHaveAttribute("src", dashboards.ops);
  await page.getByRole("button", { name: "◀ Back" }).click();
  await expect(projector.locator('[data-slide="drasi"]')).toBeVisible();
  expect(await mutations(request)).toEqual([]);
});

test("keys pressed on the projector (a clicker) drive the desk", async ({ page }) => {
  await desk(page, "title");
  const projector = await page.context().newPage();
  await projector.goto("/present");
  await projector.keyboard.press("PageDown");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("The scenario");
  await expect(projector.locator('[data-slide="concepts"]')).toBeVisible();
  await projector.keyboard.press("PageUp");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Title");
});

test("short on time skips the optional policy beats", async ({ page }) => {
  await desk(page, "recovery", 1);
  await page.getByLabel("Short on time").check();
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.locator(".say")).toContainText("Now I approve the current plan");
});

test("the current beat's buttons sit beside its notes, and results appear there", async ({ page, request }) => {
  await desk(page, "failure");
  await page.locator(".now").getByRole("button", { name: "Fail EU-A" }).click();
  await expect(page.locator(".now .result")).toHaveText("Done.");
  expect(await mutations(request)).toEqual([{ path: "/control", input: { run_epoch: "TEST-FIXTURE-EPOCH", revision: 1, endpoint: "EU-A", action: "fail" } }]);
});

test("traffic starts only through an explicit action", async ({ page, request }) => {
  await desk(page, "normal");
  expect(await mutations(request)).toEqual([]);
  await page.locator(".now").getByRole("button", { name: "Start demo traffic" }).click();
  await expect(page.locator(".now").getByRole("button", { name: /Start demo traffic/ })).toBeDisabled();
  expect((await mutations(request)).map((m: { path: string }) => m.path)).toEqual(["/traffic"]);
});

test("a plan selected before a policy change is rejected, and the desk says that's expected", async ({ page, request }) => {
  await desk(page, "recovery", 1);
  await page.locator(".now").getByRole("button", { name: "Select proposed plan" }).click();
  await expect(page.locator(".plan-table tr.selected")).toContainText("EU-B");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await page.locator(".now").getByRole("button", { name: "Summaries: Europe only" }).click();
  await expect(page.locator(".plan-table tr.stale")).toContainText("out of date");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.locator(".now-actions .hint")).toContainText("meant to be rejected");
  await page.locator(".now").getByRole("button", { name: "Approve selected plan" }).click();
  await expect(page.locator(".now .result.expected")).toContainText("Rejected, as intended");
  const state = await mutations(request);
  expect(state.at(-1).input.basis).toBe("TEST-FIXTURE-BASIS-1");
});

test("approving clears the selection, so it can't be approved twice by accident", async ({ page, request }) => {
  await desk(page, "recovery", 6);
  const now = page.locator(".now");
  await now.getByRole("button", { name: "Select proposed plan" }).click();
  await now.getByRole("button", { name: "Approve selected plan" }).click();
  await expect(now.locator(".result")).toHaveText("Done.");
  await expect(now.getByRole("button", { name: "Approve selected plan" })).toBeDisabled();
  await expect(page.locator(".approval")).toHaveText("Approved. Applying the new routes…");
  expect((await mutations(request)).filter((m: { path: string }) => m.path === "/approvals")).toHaveLength(1);
});

test("switching to recordings puts the part's recording on the projector and hides live buttons", async ({ page, request }) => {
  await desk(page, "failure");
  const projector = await page.context().newPage();
  await projector.goto("/present");
  await page.locator(".if-broken").getByRole("button", { name: "Use recordings from here on" }).click();
  await expect(projector.getByText("REPLAY · NOT LIVE", { exact: true })).toBeVisible();
  await expect(page.locator(".now").getByRole("button", { name: "Fail EU-A" })).toHaveCount(0);
  await page.locator(".desk-alert").getByRole("button", { name: "Back to the live demo" }).click();
  await expect(projector.getByText("REPLAY · NOT LIVE", { exact: true })).toHaveCount(0);
  expect(await mutations(request)).toEqual([]);
});

test("out-of-date evidence disables plan selection and live buttons", async ({ page, request }) => {
  await desk(page, "recovery", 1);
  await expect(page.locator(".now").getByRole("button", { name: "Select proposed plan" })).toBeEnabled();
  await request.post(`${fixture}/test/expire`);
  await expect(page.locator(".now").getByRole("button", { name: "Select proposed plan" })).toBeDisabled();
  await request.post(`${fixture}/test/offline`, { data: { value: true } });
  await expect(page.locator(".badge", { hasText: "NOT READY" })).toBeVisible();
});

test("reset needs a confirmation, and moving between beats sends no writes", async ({ page, request }) => {
  await desk(page, "title");
  for (let i = 0; i < 6; i++) await page.getByRole("button", { name: "Next ▶" }).click();
  expect(await mutations(request)).toEqual([]);
  await page.getByText("Before and after the talk").click();
  await page.getByRole("button", { name: "Reset scenario…" }).click();
  await page.getByRole("button", { name: "Confirm reset" }).click();
  await expect.poll(() => mutations(request)).toEqual([{ path: "/reset", input: { run_epoch: "TEST-FIXTURE-EPOCH", revision: 1 } }]);
});

test("the projector preview can't reach private notes or controls", async ({ page, request }) => {
  await page.goto("/presenter?preview=1");
  await expect(page.getByText("PRESENTER DESK", { exact: false })).toHaveCount(0);
  await expect(page.locator(".title-slide")).toBeVisible();
  expect(await mutations(request)).toEqual([]);
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

test("the query slide lists late requests live from Drasi", async ({ page, request }) => {
  await request.post(`${fixture}/test/query`, { data: { query: "overdue-tasks", rows: {
    a: { id: "a", run_epoch: "TEST-FIXTURE-EPOCH", workload: "support", trace_id: "abcdef1234567890", deadline_ms: Date.now() - 4000 },
    b: { id: "b", run_epoch: "TEST-FIXTURE-EPOCH", workload: "batch", trace_id: "ffff", deadline_ms: Date.now() - 4000 } } } });
  await page.goto("/present?view=query");
  await expect(page.locator(".query-live")).toContainText("1 support request is late right now");
  await expect(page.locator(".query-live")).toContainText("trace abcdef12");
});

for (const size of [{ width: 1280, height: 720 }, { width: 1920, height: 1080 }]) {
  test(`every slide and visual fits ${size.width}x${size.height}`, async ({ page }) => {
    await page.setViewportSize(size);
    for (const slide of allSlides) {
      await page.goto(`/present?view=${slide.id}&preview=1`);
      const content = page.locator(`[data-slide="${slide.id}"] > div`);
      await expect(content).toBeVisible();
      const fits = await content.evaluate(element => element.scrollHeight <= element.clientHeight + 2 && element.scrollWidth <= element.clientWidth + 2);
      expect(fits, slide.id).toBe(true);
      await page.screenshot({ path: `evidence/audience-${size.width}-${slide.id}.png` });
    }
  });
}

test("the desk fits a laptop screen without scrolling", async ({ browser }) => {
  for (const cue of ["failure", "recovery"] as const) {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await desk(page, cue, cue === "recovery" ? 7 : 0);
    const main = await page.locator(".desk-main").boundingBox();
    expect(main!.y + main!.height).toBeLessThanOrEqual(900);
    await page.close();
  }
});

test("the oldest late request is shown on the projector with its attempts", async ({ page, request }) => {
  await request.post(`${fixture}/test/query`, { data: { query: "overdue-tasks", rows: {
    "TEST-TASK": { id: "TEST-TASK", run_epoch: "TEST-FIXTURE-EPOCH", workload: "support", trace_id: "TEST-TRACE", deadline_ms: Date.now() - 4000 } } } });
  await page.route("**/api/tasks/TEST-TASK", route => route.fulfill({ json: {
    task: { id: "TEST-TASK", content: "My order hasn't arrived.", created_ms: Date.now() - 20000, deadline_ms: Date.now() - 5000, completed_ms: null, trace_id: "abcdef123456" },
    attempts: [503, 503, 503].map((status, i) => ({ endpoint: "EU-A", status, started_ms: Date.now() - 18000 + i * 3000 })) } }));
  await desk(page, "who", 0);
  const projector = await page.context().newPage();
  await projector.route("**/api/tasks/TEST-TASK", route => route.fulfill({ json: {
    task: { id: "TEST-TASK", content: "My order hasn't arrived.", created_ms: Date.now() - 20000, deadline_ms: Date.now() - 5000, completed_ms: null, trace_id: "abcdef123456" },
    attempts: [503, 503, 503].map((status, i) => ({ endpoint: "EU-A", status, started_ms: Date.now() - 18000 + i * 3000 })) } }));
  await projector.goto("/present");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(projector.locator(".request-slide h1")).toContainText("A customer has been waiting");
  await expect(projector.locator(".timeline")).toContainText("Sent to EU-A: failed (HTTP 503) × 3");
  await expect(projector.locator(".timeline")).toContainText("Drasi added it to the overdue list");
});

test("Next warns once when the beat's button hasn't been pressed", async ({ page, request }) => {
  await desk(page, "failure");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.getByRole("alert")).toContainText("You haven't pressed Fail EU-A yet");
  await expect(page.locator(".say")).toContainText("make EU-A");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.locator(".say")).toContainText("Look at the bottom row");
  expect(await mutations(request)).toEqual([]);
});

test("recording mode shows each live part's recording as the presenter presses Next", async ({ page, request }) => {
  await desk(page, "normal", 2);
  const projector = await page.context().newPage();
  await projector.goto("/present");
  await page.getByRole("button", { name: "Use recordings from here on" }).click();
  await expect(page.getByText("Recording mode: the projector shows recordings")).toBeVisible();
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.locator(".now-actions")).toHaveCount(0);
  await expect(projector.getByText("REPLAY · NOT LIVE", { exact: true })).toBeVisible();
  await expect(projector.locator(".replay-screen h1")).toHaveText(replayClips.find(c => c.scene === "failure")!.title);
  await page.locator(".desk-alert").getByRole("button", { name: "Back to the live demo" }).click();
  await expect(projector.getByText("REPLAY · NOT LIVE", { exact: true })).toHaveCount(0);
  expect(await mutations(request)).toEqual([]);
});

test("a rejected approval clears the selection, and the first Next starts the talk timer", async ({ page }) => {
  await desk(page, "recovery", 1);
  const now = page.locator(".now");
  await now.getByRole("button", { name: "Select proposed plan" }).click();
  await page.getByRole("button", { name: "Next ▶" }).click();
  await now.getByRole("button", { name: "Summaries: Europe only" }).click();
  await expect(now.getByRole("button", { name: /Summaries: Europe only/ })).toContainText("✓");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await now.getByRole("button", { name: "Approve selected plan" }).click();
  await expect(now.locator(".result.expected")).toBeVisible();
  await expect(page.locator(".plan-table tr.stale, .plan-table tr.selected")).toHaveCount(0);
  await expect(page.locator(".clock strong")).not.toHaveText("00:00", { timeout: 5000 });
});

test("with no desk open, the clicker still steps through the slides", async ({ page }) => {
  await stubGrafana(page);
  await page.goto("/present");
  await expect(page.locator(".title-slide")).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await expect(page.locator('[data-slide="concepts"]')).toBeVisible();
  await page.keyboard.press("PageDown");
  await expect(page.locator('[data-slide="drasi"]')).toBeVisible();
  await page.keyboard.press("PageUp");
  await expect(page.locator('[data-slide="concepts"]')).toBeVisible();
});

test("a slow click on a desk button still works while the desk refreshes", async ({ page, request }) => {
  await desk(page, "failure");
  const button = page.locator(".now").getByRole("button", { name: "Fail EU-A" });
  const box = (await button.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.waitForTimeout(2500); // the desk re-renders at least twice in this time
  await page.mouse.up();
  await expect.poll(async () => (await mutations(request)).map((m: { path: string }) => m.path)).toEqual(["/control"]);
});
