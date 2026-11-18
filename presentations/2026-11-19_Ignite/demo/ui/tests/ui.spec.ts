import { settings, url, port } from "../settings.mjs";
import { test, expect, type Page } from "@playwright/test";
import { applyFrame, applyChanges } from "../src/live";
import { healthSchema, readPlan } from "../src/contracts";
import { scenes, slides, recaps, allSlides, dashboards, incidentChannel, frame } from "../../../presentation/scenes";
import { notes } from "../../../presentation/notes";
import { presenterScript } from "../../../presentation/script";
import { replayClips } from "../../../presentation/replay";
import { displayQuery } from "../../../presentation/QuerySlide";
import { buildCodeSlides } from "../../../presentation/codeSlides";
import { unknownKeys, type Dashboard } from "../../../presentation/panels";
import fs from "node:fs";

const fixture = url("UI_FIXTURE");
const stageKey = settings.IGNITE_STORAGE_KEY;
test.beforeEach(async ({ request }) => { await request.post(`${fixture}/test/reset`); });
const mutations = async (request: import("@playwright/test").APIRequestContext) => (await (await request.get(`${fixture}/test/state`)).json()).mutations;
// Open the desk on a given part and beat.
async function desk(page: Page, cue: string, beat = 0, extra: object = {}) {
  const index = scenes.findIndex(scene => scene.id === cue);
  await page.addInitScript(([key, value]) => { if (!sessionStorage.getItem("seeded")) { localStorage.setItem(key, value); sessionStorage.setItem("seeded", "1"); } },
    [stageKey, JSON.stringify({ cue: index, beat, view: notes[cue as keyof typeof notes].beats[beat].show, trace: null, replay: null, short: false, ...extra })]);
  await page.goto("/presenter");
  await expect(page.getByRole("heading", { level: 1 })).toContainText(scenes[index].title);
}
const stubGrafana = (page: Page) => page.route(`http://localhost:${port("GRAFANA")}/**`, route => route.fulfill({ contentType: "text/html", body: "<h1>TEST Grafana</h1>" }));
// Mattermost's URL serves the fixture's stand-in channel (tests/mattermost-stand-in.html) and its posts.
const stubMattermost = (page: Page) => page.route(`http://localhost:${port("MATTERMOST")}/**`, async route => {
  const path = new URL(route.request().url()).pathname;
  const target = path.startsWith("/test-fixture/") ? `${fixture}/test/mattermost/${path.slice("/test-fixture/".length)}` : `${fixture}/test/mattermost/channel`;
  await route.fulfill({ response: await route.fetch({ url: target }) });
});
const posts = async (request: import("@playwright/test").APIRequestContext) =>
  (await (await request.get(`${fixture}/test/mattermost/posts`)).json()) as { id: string; root_id: string; props: { ignite_card: string; attachments: { title: string; actions?: unknown[] }[] } }[];
const failEuA = (request: import("@playwright/test").APIRequestContext) =>
  request.post(`${fixture}/control`, { data: { run_epoch: "TEST-FIXTURE-EPOCH", deployment: "EU-A", action: "fail" } });

test("the talk is 22 minutes and every beat says something and shows something", () => {
  expect(scenes.reduce((sum, s) => sum + s.minutes, 0)).toBe(22);
  for (const scene of scenes) {
    const cue = notes[scene.id];
    expect(cue.beats.length).toBeGreaterThan(0);
    for (const beat of cue.beats) {
      expect(beat.say.trim()).not.toBe("");
      expect(["ops", "incident", "request", ...allSlides.map(s => s.id)]).toContain(beat.show);
    }
    expect(cue.ifBroken.trim()).not.toBe("");
  }
  for (const recap of recaps) expect(Object.values(notes).some(cue => cue.beats.some(beat => beat.show === recap.id))).toBe(true);
  expect(slides.map(s => s.id)).toEqual(["walkin", "title", "concepts", "concepts-rules", "concepts-questions", "setup", "drasi", "health", "query", "policy-config", "optimization-config", "closing", "survey"]);
  // The walk-in opens the talk, the scenario reveals a section per beat, and the survey is last.
  const shown = scenes.flatMap(scene => notes[scene.id].beats.map(beat => beat.show));
  expect(shown[0]).toBe("walkin");
  expect(notes.concepts.beats.map(beat => beat.show)).toEqual(["concepts", "concepts-rules", "concepts-questions", "setup"]);
  expect(shown.at(-1)).toBe("survey");
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

test("every highlight in the script names a panel on the dashboards", () => {
  const folder = new URL("../../observability/grafana/dashboards/", import.meta.url);
  const boards = fs.readdirSync(folder).filter(name => name.endsWith(".json"))
    .map(name => JSON.parse(fs.readFileSync(new URL(name, folder), "utf8")) as Dashboard);
  const keys = Object.values(notes).flatMap(cue => cue.beats.flatMap(beat => beat.highlight ?? []));
  expect(keys.length).toBeGreaterThan(0);
  expect(unknownKeys(boards, keys)).toEqual([]);
});

test("the scenario reveals its sections one desk beat at a time", async ({ page }) => {
  await desk(page, "concepts");
  const projector = await page.context().newPage();
  await projector.goto("/present");
  const slide = (id: string) => projector.locator(`[data-slide="${id}"]`);
  await expect(slide("concepts").locator(".deployments")).toBeVisible();
  await expect(slide("concepts").locator(".requirements")).toBeHidden();
  await expect(slide("concepts").locator(".questions")).toBeHidden();
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(slide("concepts-rules").locator(".requirements")).toBeVisible();
  await expect(slide("concepts-rules").locator(".questions")).toBeHidden();
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(slide("concepts-questions").locator(".questions")).toContainText("When something goes wrong");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(slide("setup")).toBeVisible();
});

test("a beat's highlight outlines its dashboard panels on the projector, above the caption line", async ({ page }) => {
  await stubGrafana(page);
  await desk(page, "normal", 0);
  const projector = await page.context().newPage();
  await stubGrafana(projector);
  await projector.goto("/present");
  // The dashboard first appears with no highlight; the top row is highlighted on the next beat.
  await expect(projector.locator("iframe.stage-dashboard").first()).toBeVisible();
  await expect(projector.locator(".panel-highlight")).toHaveCount(0);
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.locator(".say")).toContainText("The top row");
  const outline = projector.locator(".panel-highlight .outline");
  await expect(outline).toHaveCount(notes.normal.beats[1].highlight!.length);
  const box = await onSlide(projector, ".panel-highlight .outline");
  expect(box[0].bottom).toBeLessThanOrEqual(frame.captionTop);
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.locator(".say")).toContainText("The bottom row");
  await expect(outline).toHaveCount(notes.normal.beats[2].highlight!.length);
  for (let i = 0; i < 2; i++) await page.getByRole("button", { name: "◀ Back" }).click();
  await expect(projector.locator(".panel-highlight")).toHaveCount(0);
});

test("the configuration slides show the real files, and every numbered note marks a line", () => {
  const read = (path: string) => fs.readFileSync(new URL(`../../drasi/${path}`, import.meta.url), "utf8");
  const codeSlides = buildCodeSlides({ healthQuery: read("deployment-health.cypher"), tileQuery: read("tile-deployments.cypher"),
    rego: read("policy/ignite.rego"), optimization: JSON.parse(read("optimization/optimization.json")) });
  expect(codeSlides.health.panels[0].lines.join("\n")).toContain(read("deployment-health.cypher").split("\n").find(line => line.startsWith("MATCH"))!);
  expect(codeSlides["policy-config"].panels[0].lines).toContain("allowed_destination if input.target.id in input.destinations");
  for (const [id, slide] of Object.entries(codeSlides)) {
    const marks = slide.panels.flatMap(panel => panel.marks.filter(([text]) => panel.lines.some(line => line.includes(text))).map(([, n]) => n));
    expect(marks.sort((a, b) => a - b), id).toEqual(slide.notes.map((_, i) => i + 1));
  }
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
  await expect(projector.locator(".walkin-slide")).toBeVisible();
  // The walk-in beat's Start demo traffic isn't pressed here: the first Next only reminds.
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.getByRole("alert")).toContainText("Start demo traffic");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(projector.locator(".title-slide")).toBeVisible();
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(projector.locator('[data-slide="concepts"]')).toBeVisible();
  for (let i = 0; i < 6; i++) await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Healthy service");
  await expect(projector.locator("iframe.stage-dashboard").first()).toBeVisible();
  await expect(projector.locator("iframe.stage-dashboard").first()).toHaveAttribute("src", dashboards.ops);
  await page.getByRole("button", { name: "◀ Back" }).click();
  await expect(projector.locator('[data-slide="drasi"]')).toBeVisible();
  // Off-screen dashboards stay rendered (visibility, not display:none) so Grafana keeps refreshing them.
  await expect(projector.locator("iframe.stage-dashboard").first()).toBeHidden();
  await expect(projector.locator("iframe.stage-dashboard").first()).toHaveCSS("display", "block");
  expect(await mutations(request)).toEqual([]);
});

test("keys pressed on the projector (a clicker) drive the desk", async ({ page }) => {
  await desk(page, "title");
  const projector = await page.context().newPage();
  await projector.goto("/present");
  await projector.keyboard.press("PageDown"); // the reminder to start demo traffic
  await expect(page.getByRole("alert")).toContainText("Start demo traffic");
  await projector.keyboard.press("PageDown");
  await expect(projector.locator(".title-slide")).toBeVisible();
  await projector.keyboard.press("PageDown");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("The scenario");
  await expect(projector.locator('[data-slide="concepts"]')).toBeVisible();
  await projector.keyboard.press("PageUp");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Title");
});

test("short on time skips the optional policy beats", async ({ page }) => {
  await desk(page, "recovery", 2);
  await page.getByLabel("Short on time").check();
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.locator(".say")).toContainText("Now I approve the current proposal");
  await expect(page.locator(".click")).toContainText("Approve on the newest orange Proposed change card");
});

test("the current beat's buttons sit beside its notes, and results appear there", async ({ page, request }) => {
  await desk(page, "failure");
  await page.locator(".now").getByRole("button", { name: "Fail EU-A" }).click();
  await expect(page.locator(".now .result")).toHaveText("Done.");
  expect(await mutations(request)).toEqual([{ path: "/control", input: { run_epoch: "TEST-FIXTURE-EPOCH", deployment: "EU-A", action: "fail" } }]);
});

test("traffic starts only through an explicit action, on the walk-in beat before the talk", async ({ page, request }) => {
  const traffic = scenes.flatMap(scene => notes[scene.id].beats.filter(beat => beat.do?.includes("start-traffic")).map(beat => beat.show));
  expect(traffic).toEqual(["walkin"]);
  await desk(page, "title", 0);
  expect(await mutations(request)).toEqual([]);
  await page.locator(".now").getByRole("button", { name: "Start demo traffic" }).click();
  await expect(page.locator(".now").getByRole("button", { name: /Start demo traffic/ })).toBeDisabled();
  expect((await mutations(request)).map((m: { path: string }) => m.path)).toEqual(["/traffic"]);
});

test("the desk shows how long demo traffic has been running", async ({ page }) => {
  await page.route("**/api/health", async route => {
    const health = await (await route.fetch()).json();
    await route.fulfill({ json: { ...health, traffic: { ...health.traffic, running: true, ends_ms: Date.now() + 90 * 60 * 1000 - 125000 } } });
  });
  await desk(page, "title");
  await expect(page.locator(".traffic")).toContainText(/Traffic: running for 2 min \d+ s/);
  await expect(page.locator(".now").getByRole("button", { name: /Start demo traffic/ })).toBeDisabled();
});

test("approving the out-of-date proposal is rejected by Drasi, and the desk says that's expected", async ({ page, request }) => {
  await failEuA(request);
  await desk(page, "recovery", 3);
  await expect.poll(async () => (await posts(request)).map(p => p.props.ignite_card)).toEqual(["failed", "proposal"]);
  await page.locator(".now").getByRole("button", { name: "Summaries: Europe only" }).click();
  await expect(page.locator(".now").getByRole("button", { name: /Summaries: Europe only/ })).toContainText("✓");
  await expect.poll(async () => (await posts(request)).at(-1)?.props.attachments[0].title).toBe("Out of date: the facts changed");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.locator(".now-actions .hint")).toContainText("meant to be rejected");
  await expect(page.locator(".click")).toContainText("above the grey Out of date card");
  await expect(page.locator(".now .result").last()).toHaveText("Waiting for Approve on the out-of-date proposal card.");
  // The desk's backup presses Approve on the newest proposal card, which is the out-of-date one.
  await page.locator(".now").getByRole("button", { name: "Approve the proposed change" }).click();
  await expect(page.locator(".now .result.expected")).toContainText("Rejected by Drasi, as intended: The plan changed since it was approved");
  await expect(page.locator(".now").getByRole("button", { name: /Approve the proposed change/ })).toContainText("✓");
  await expect(page.locator(".approval")).toHaveText("Last approval rejected: The plan changed since it was approved.");
  const state = await (await request.get(`${fixture}/test/state`)).json();
  expect(state.mutations.at(-1)).toEqual({ path: "/mattermost/approve-latest", input: { run_epoch: "TEST-FIXTURE-EPOCH" } });
  const proposal = (await posts(request)).find(p => p.props.ignite_card === "proposal")!;
  expect(state.approvals[0]).toMatchObject({ id: `mm-${proposal.id}`, candidate_id: "TEST-FIXTURE-CANDIDATE-1", basis: "TEST-FIXTURE-BASIS-1", revision: 1, status: "rejected" });
  // Drasi's answer replies in the approved card's thread.
  await expect.poll(async () => (await posts(request)).at(-1)).toMatchObject({ root_id: proposal.id, props: { ignite_card: "outcome", attachments: [{ title: "Rejected by Drasi: nothing was changed" }] } });
  // Once policy allows the US again, a new proposal follows and Next isn't held back.
  await page.getByRole("button", { name: "Next ▶" }).click();
  await page.locator(".now").getByRole("button", { name: "Summaries: Europe + US" }).click();
  await expect.poll(async () => (await posts(request)).filter(p => p.props.ignite_card === "proposal")).toHaveLength(2);
});

test("pressing Approve on the projected card applies the change, and the desk reports Drasi's answer", async ({ page, request }) => {
  await failEuA(request);
  await desk(page, "recovery", 6);
  const projector = await page.context().newPage();
  await stubMattermost(projector);
  await projector.goto("/present");
  const channel = projector.frameLocator("iframe.stage-incident");
  const now = page.locator(".now");
  await expect(now.locator(".result").last()).toHaveText("Waiting for Approve on the newest proposal card.");
  await channel.locator('[data-card="proposal"]').last().getByRole("button", { name: "Approve" }).click();
  await expect(now.locator(".result").last()).toHaveText("Done: Drasi checked it and the new routes are applied.");
  await expect(now.getByRole("button", { name: /Approve the proposed change/ })).toBeDisabled();
  await expect(now.getByRole("button", { name: /Approve the proposed change/ })).toContainText("✓");
  await expect(page.locator(".approval")).toHaveText("Applied. Waiting for evidence that recovery worked.");
  await expect(page.locator(".plan-table tr").nth(1)).toContainText("EU-B");
  await expect(page.locator(".answers")).toContainText("Awaiting evidence");
  // The card shows who approved it; Drasi's outcomes reply in its thread.
  const card = channel.locator('[data-card="proposal"]').last();
  await expect(card).toContainText("Approved by @oncall at");
  await expect(card.getByRole("button")).toHaveCount(0);
  await expect(channel.locator('[data-card="outcome"]')).toContainText("Applied: Move summaries to US-C and support to EU-B");
  await request.post(`${fixture}/test/recover`);
  await expect(page.locator(".approval")).toHaveText("Recovery verified ✓");
  await expect(channel.locator('[data-card="outcome"]').last()).toContainText("Verified: support has recovered");
  const state = await (await request.get(`${fixture}/test/state`)).json();
  expect(state.mutations.map((m: { path: string }) => m.path)).toEqual(["/control", "/mattermost/actions"]);
  // With nothing left to approve, the backup has nothing to press.
  expect((await request.post(`${fixture}/mattermost/approve-latest`, { data: { run_epoch: "TEST-FIXTURE-EPOCH" } })).status()).toBe(404);
});

test("the projected incident view frames the channel above the caption line, kept loaded once shown", async ({ page, request }) => {
  await stubGrafana(page);
  await stubMattermost(page);
  await failEuA(request);
  await page.setViewportSize({ width: 1920, height: 1080 });
  await desk(page, "failure", 3);
  const projector = await page.context().newPage();
  await stubGrafana(projector);
  await stubMattermost(projector);
  await projector.setViewportSize({ width: 1920, height: 1080 });
  await projector.goto("/present");
  const iframe = projector.locator("iframe.stage-incident");
  await expect(iframe).toBeVisible();
  await expect(iframe).toHaveAttribute("src", incidentChannel);
  expect(incidentChannel).toBe(`http://localhost:${port("MATTERMOST")}/ai-platform/channels/incident-ai-support`);
  const box = (await iframe.boundingBox())!;
  for (const [measured, expected] of [[box.x, 0], [box.y, 0], [box.width, 1920], [box.height, frame.captionTop]]) expect(measured).toBeCloseTo(expected, 0);
  await expect(projector.frameLocator("iframe.stage-incident").locator('[data-card="failed"]')).toContainText("Deployment failed: EU-A");
  await expect(page.locator(".projector-line")).toContainText("Mattermost · #incident-ai-support");
  await page.getByRole("button", { name: "◀ Back" }).click();
  await expect(iframe).toBeHidden();
  await expect(iframe).toHaveCSS("display", "block");
});

test("the desk links the projector's Mattermost sign-in", async ({ page }) => {
  await desk(page, "title");
  await page.getByText("Before and after the talk").click();
  await expect(page.getByRole("link", { name: "Sign the projector in to Mattermost" })).toHaveAttribute("href", `http://localhost:${port("WORKLOAD")}/mattermost/sign-in`);
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

test("out-of-date evidence disables approval and live buttons", async ({ page, request }) => {
  await failEuA(request);
  await desk(page, "recovery", 6);
  await expect(page.locator(".now").getByRole("button", { name: "Approve the proposed change" })).toBeEnabled();
  await request.post(`${fixture}/test/expire`);
  await expect(page.locator(".now").getByRole("button", { name: "Approve the proposed change" })).toBeDisabled();
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
  await expect(page.locator(".walkin-slide")).toBeVisible();
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
  // Tile queries have no id: rows are matched by content, and aggregates replace their "before".
  let tile = applyFrame({}, "snapshot", [{ workload: "support", value: 1 }], false);
  tile = applyFrame(tile, "change", { results: [{ type: "aggregation", before: { workload: "support", value: 1 }, after: { workload: "support", value: 2 } }, { type: "noop" }] }, true);
  tile = applyFrame(tile, "change", { results: [{ type: "UPDATE", before: { workload: "support", value: 2 }, after: { workload: "support", value: 3 }, data: { workload: "support", value: 3 } }] }, true);
  expect(Object.values(tile)).toEqual([{ workload: "support", value: 3 }]);
  tile = applyFrame(tile, "change", { results: [{ type: "DELETE", data: { workload: "support", value: 3 } }] }, true);
  expect(tile).toEqual({});
});

test("stream changes apply by Drasi identity: add, replace, patch and delete", () => {
  const id = (n: string) => ({ namespace: "drasi.output/x", id: n });
  let rows = applyChanges({}, [{ operation: "ADD", identity: id("a"), after: { value: 1, lost: "EU-A" } }]);
  rows = applyChanges(rows, [{ operation: "REPLACE", identity: id("a"), after: { value: 2, lost: "EU-B" } }]);
  rows = applyChanges(rows, [{ operation: "PATCH", identity: id("a"), after: { value: 3 }, removedProperties: ["lost"] }]);
  expect(Object.values(rows)).toEqual([{ value: 3 }]);
  rows = applyChanges(rows, [{ operation: "ADD", identity: id("b"), after: { value: 9 } }, { operation: "DELETE", identity: id("a") }]);
  expect(Object.values(rows)).toEqual([{ value: 9 }]);
  expect(() => applyChanges({}, [{ operation: "ADD", identity: id("c") }])).toThrow("without its row");
});

test("the fixture speaks the agreed AI service and Drasi shapes", async ({ request }) => {
  const health = healthSchema.parse(await (await request.get(`${fixture}/health`)).json());
  expect(health.destinations.batch).toContain("US-C");
  const { plan, error } = readPlan([{ id: "plan", state: "feasible", candidate_id: "c", basis: "b", revision: 1, run_epoch: "e", assignments: "{\"support\":\"EU-B\"}", moves: 1, reason: "r" }]);
  expect(error).toBeNull();
  expect(plan?.assignments).toEqual({ support: "EU-B" });
  expect(readPlan([{ id: "plan", state: "maybe" }]).plan).toBeNull();
});

test("the desk shows Drasi's answers from Drasi Server and never asks the AI service for them", async ({ page, request }) => {
  const asked: string[] = [];
  page.on("request", r => asked.push(new URL(r.url()).pathname));
  await desk(page, "failure");
  const answers = page.locator(".answers");
  await expect(answers.locator("div", { hasText: "Recommended action" })).toContainText("None");
  await expect(answers).toContainText("Failover readiness");
  await expect(answers.locator("div", { hasText: "Failover readiness" })).toContainText("Yes");
  await expect(answers).toContainText("Nothing to verify");
  await page.locator(".now").getByRole("button", { name: "Fail EU-A" }).click();
  await expect(answers).toContainText("Already one down");
  await expect(answers.locator("div", { hasText: "Recommended action" })).toContainText("Move summaries to US-C and support to EU-B");
  await request.post(`${fixture}/test/expire`);
  await expect(answers.locator("div", { hasText: "Recommended action" })).toContainText("Unknown");
  await expect(page.locator(".plan-table")).toContainText("Unknown");
  // One event stream from Drasi's SSE reaction, each output's snapshot once, and no polling of Drasi.
  expect(asked.filter(path => path === "/drasi-stream/events")).toHaveLength(1);
  for (const query of ["plan-results", "overdue-tasks", "tile-action", "tile-resilience", "tile-recovery", "tile-last-approval", "tile-running"])
    expect(asked.filter(path => path === `/drasi-stream/snapshot/${query}`)).toHaveLength(1);
  expect(asked.filter(path => path.startsWith("/server/"))).toEqual([]);
  expect(asked.filter(path => /^\/api\/(decisions|context)/.test(path))).toEqual([]);
  expect(asked.filter(path => path.startsWith("/api/")).every(path => path === "/api/health" || path.startsWith("/api/control"))).toBe(true);
});

test("Drasi's stream unavailable means NOT READY", async ({ page }) => {
  await page.route("**/drasi-stream/**", route => route.fulfill({ status: 503, body: "down" }));
  await desk(page, "recovery", 6);
  await expect(page.locator(".badge", { hasText: "NOT READY" })).toBeVisible();
  await expect(page.locator(".now").getByRole("button", { name: "Approve the proposed change" })).toBeDisabled();
  await expect(page.locator(".approval")).toHaveText("Approval status unavailable.");
});

test("the query slide lists late requests live from Drasi", async ({ page, request }) => {
  await request.post(`${fixture}/test/query`, { data: { query: "overdue-tasks", rows: {
    a: { id: "a", run_epoch: "TEST-FIXTURE-EPOCH", workload: "support", trace_id: "abcdef1234567890", deadline_ms: Date.now() - 4000 },
    b: { id: "b", run_epoch: "TEST-FIXTURE-EPOCH", workload: "batch", trace_id: "ffff", deadline_ms: Date.now() - 4000 } } } });
  await page.goto("/present?view=query");
  await expect(page.locator(".query-live")).toContainText("1 support request is late right now");
  await expect(page.locator(".query-live")).toContainText("trace abcdef12");
});

// Where an element sits on the template's 1920x1080 slide, whatever the window size.
async function onSlide(page: Page, selector: string) {
  return page.evaluate(selector => {
    const stage = document.querySelector(".stage-frame")!.getBoundingClientRect();
    const scale = stage.width / 1920;
    return [...document.querySelectorAll(selector)].map(element => {
      const box = element.getBoundingClientRect();
      return { name: `${element.tagName}.${element.className}`, left: (box.left - stage.left) / scale, top: (box.top - stage.top) / scale,
        right: (box.right - stage.left) / scale, bottom: (box.bottom - stage.top) / scale, width: box.width, height: box.height };
    }).filter(box => box.width > 0 && box.height > 0);
  }, selector);
}

for (const size of [{ width: 1280, height: 720 }, { width: 1920, height: 1080 }]) {
  test(`every slide and visual fits ${size.width}x${size.height}, inside the margins and above the captions`, async ({ page }) => {
    await page.setViewportSize(size);
    for (const slide of allSlides) {
      await page.goto(`/present?view=${slide.id}&preview=1`);
      const content = page.locator(`[data-slide="${slide.id}"] > div`);
      await expect(content).toBeVisible();
      const fits = await content.evaluate(element => element.scrollHeight <= element.clientHeight + 2 && element.scrollWidth <= element.clientWidth + 2
        && [...element.querySelectorAll(".body")].every(body => body.scrollHeight <= body.clientHeight + 2));
      expect(fits, slide.id).toBe(true);
      // Template artwork is full bleed; everything else stays inside the 92 px margins and above y 1002.
      for (const box of await onSlide(page, `[data-slide="${slide.id}"] > div *:not(.artwork):not(svg *)`)) {
        expect(box.bottom, `${slide.id} ${box.name}`).toBeLessThanOrEqual(frame.captionTop + 1);
        expect(box.left, `${slide.id} ${box.name}`).toBeGreaterThanOrEqual(92 - 1);
        expect(box.right, `${slide.id} ${box.name}`).toBeLessThanOrEqual(frame.width - 92 + 1);
      }
      await page.screenshot({ path: `evidence/audience-${size.width}-${slide.id}.png` });
    }
  });
}

test("the stage is the template's 16:9 slide, 1:1 at 1920x1080, and dashboards stop at the caption line", async ({ page }) => {
  await stubGrafana(page);
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto("/present?view=ops&preview=1");
  expect(await page.locator(".stage-frame").boundingBox()).toEqual({ x: 0, y: 0, width: 1920, height: 1080 });
  const dashboard = (await page.locator("iframe.stage-dashboard").first().boundingBox())!;
  for (const [measured, expected] of [[dashboard.x, 0], [dashboard.y, 0], [dashboard.width, 1920], [dashboard.height, frame.captionTop]])
    expect(measured).toBeCloseTo(expected, 1);
  await page.setViewportSize({ width: 1440, height: 900 });
  await expect.poll(async () => (await page.locator(".stage-frame").boundingBox())!.width).toBeCloseTo(1440, 0);
  const letterboxed = (await page.locator(".stage-frame").boundingBox())!;
  expect(letterboxed.width / letterboxed.height).toBeCloseTo(16 / 9, 3);
  expect(letterboxed.y).toBeCloseTo((900 - 810) / 2, 0);
});

test("without the template assets the stage still works, and the desk says they're missing", async ({ page }) => {
  await page.route("**/template/**", route => route.fulfill({ status: 404, body: "missing" }));
  await page.goto("/present?view=survey&preview=1");
  await expect(page.locator(".survey-copy h1")).toHaveText("How did we do?");
  await expect(page.locator("img.survey-qr")).toHaveCount(0);
  await page.goto("/present?view=walkin&preview=1");
  await expect(page.locator(".walkin-name")).toContainText("Ignite");
  await desk(page, "title");
  await expect(page.locator(".badge", { hasText: "Ignite template assets missing" })).toBeVisible();
});

test("with the template assets present, the desk doesn't flag them", async ({ page }) => {
  await page.route("**/template/manifest.json", route => route.fulfill({ json: { required: ["walkin.svg", "survey-qr.png"] } }));
  await page.route("**/template/*.{svg,png}", route => route.fulfill({ status: 200, contentType: "image/svg+xml", body: "<svg xmlns='http://www.w3.org/2000/svg'/>" }));
  await desk(page, "title");
  await page.getByText("Before and after the talk").click();
  await expect(page.getByText("Ignite template assets present")).toBeVisible();
  await expect(page.locator(".badge", { hasText: "Ignite template assets missing" })).toHaveCount(0);
});

test("the desk fits a laptop screen without scrolling", async ({ browser }) => {
  for (const cue of ["failure", "recovery"] as const) {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await desk(page, cue, cue === "recovery" ? 6 : 0);
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
  await desk(page, "who", 1);
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

test("the first Next starts the talk timer", async ({ page }) => {
  await desk(page, "recovery", 0);
  await expect(page.locator(".clock strong")).toHaveText("00:00");
  await page.getByRole("button", { name: "Next ▶" }).click();
  await expect(page.locator(".clock strong")).not.toHaveText("00:00", { timeout: 5000 });
});

test("with no desk open, the clicker still steps through the slides", async ({ page }) => {
  await stubGrafana(page);
  await page.goto("/present");
  await expect(page.locator(".walkin-slide")).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await expect(page.locator(".title-slide")).toBeVisible();
  await page.keyboard.press("PageDown");
  await expect(page.locator('[data-slide="concepts"]')).toBeVisible();
  await page.keyboard.press("PageUp");
  await expect(page.locator(".title-slide")).toBeVisible();
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

test("the incident channel follows the talk: failure, overdue, proposal, out of date, rejected, applied, verified, at risk, restored", async ({ browser, request }) => {
  test.slow();
  const context = await browser.newContext();
  const page = await context.newPage();
  await page.setViewportSize({ width: 1440, height: 900 });
  await stubGrafana(page);
  await desk(page, "failure", 0);
  const projector = await context.newPage();
  await projector.setViewportSize({ width: 1920, height: 1080 });
  await stubGrafana(projector);
  await stubMattermost(projector);
  await projector.goto("/present");
  const channel = projector.frameLocator("iframe.stage-incident");
  const card = (name: string) => channel.locator(`[data-card="${name}"]`);
  const next = () => page.getByRole("button", { name: "Next ▶" }).click();
  const press = (name: string) => page.locator(".now").getByRole("button", { name }).click();
  const shot = async (n: number, stage: string) => {
    await projector.waitForTimeout(400);
    await projector.screenshot({ path: `evidence/incident-${String(n).padStart(2, "0")}-${stage}.png` });
    await page.screenshot({ path: `evidence/desk-${String(n).padStart(2, "0")}-${stage}.png` });
  };

  await press("Fail EU-A");
  await expect(page.locator(".now").getByRole("button", { name: /Fail EU-A/ })).toContainText("✓");
  await next(); await next(); await next();
  await expect(page.locator(".projector-line")).toContainText("Mattermost · #incident-ai-support");
  await expect(card("failed")).toContainText("Deployment failed: EU-A");
  await expect(card("proposal")).toContainText("Proposed change: Move summaries to US-C and support to EU-B");
  await shot(1, "failure");
  await request.post(`${fixture}/test/query`, { data: { query: "overdue-tasks", rows: Object.fromEntries([1, 2, 3].map(n =>
    [`late-${n}`, { id: `late-${n}`, run_epoch: "TEST-FIXTURE-EPOCH", workload: "support", trace_id: `trace${n}`, deadline_ms: Date.now() - n * 1000 }])) } });

  for (let i = 0; i < 4; i++) await next();
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Who is affected");
  await expect(card("overdue")).toContainText("Support requests overdue");
  await shot(2, "overdue");

  await next(); await next(); await next();
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Recovery");
  await expect(card("proposal").getByRole("button", { name: "Approve" })).toBeVisible();
  await shot(3, "proposal");

  await next(); await next(); // the policy and optimization slides
  await next();
  await press("Summaries: Europe only");
  await expect(page.locator(".now").getByRole("button", { name: /Summaries: Europe only/ })).toContainText("✓");
  await expect(card("superseded")).toContainText("Out of date: the facts changed");
  await shot(4, "out-of-date");

  await next();
  await card("proposal").first().getByRole("button", { name: "Approve" }).click();
  await expect(page.locator(".now .result.expected")).toContainText("Rejected by Drasi, as intended");
  await expect(card("outcome")).toContainText("Rejected by Drasi: nothing was changed");
  await shot(5, "rejected");

  await next();
  await press("Summaries: Europe + US");
  await expect(page.locator(".now").getByRole("button", { name: /Summaries: Europe \+ US/ })).toContainText("✓");
  await expect(card("proposal")).toHaveCount(2);
  await shot(6, "new-proposal");

  await next(); await next();
  await expect(page.locator(".click")).toContainText("newest orange Proposed change card");
  await card("proposal").last().getByRole("button", { name: "Approve" }).click();
  await expect(page.locator(".now .result").last()).toHaveText("Done: Drasi checked it and the new routes are applied.");
  await expect(card("outcome").last()).toContainText("Applied: Move summaries to US-C and support to EU-B");
  await shot(7, "applied");

  await request.post(`${fixture}/test/recover`);
  await next(); await next(); await next();
  await expect(card("outcome").last()).toContainText("Verified: support has recovered");
  await expect(page.locator(".approval")).toHaveText("Recovery verified ✓");
  await shot(8, "verified");

  await next(); await next();
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Healthy but fragile");
  await press("EU-A back at 50k");
  await expect(page.locator(".now").getByRole("button", { name: /EU-A back at 50k/ })).toContainText("✓");
  await next();
  await expect(card("at-risk")).toContainText("Failover at risk: losing EU-B leaves support nowhere to run");
  await expect(page.locator(".answers")).toContainText("No: not if EU-B fails");
  await shot(9, "at-risk");

  await next(); await next();
  await press("EU-A back to 100k");
  await expect(card("restored")).toContainText("Failover restored");
  await expect(page.locator(".answers")).toContainText("Yes");
  await shot(10, "restored");
  await context.close();
});

test("the stage moves from 127.0.0.1 to localhost, so the framed Mattermost gets its cookies", async ({ page }) => {
  for (const path of ["/presenter?x=1#here", "/present?view=title&preview=1"]) {
    await page.goto(`${url("UI_TEST")}${path}`);
    await expect(page).toHaveURL(`${url("UI_TEST", "localhost")}${path}`);
  }
  await expect(page.locator(".title-slide")).toBeVisible();
});
