import { chromium, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { url } from "../settings.mjs";

const ui = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const demo = path.dirname(ui);
const out = path.join(demo, ".native/evidence/release");
const origin = url("UI");
const mode = process.argv[2];
await fs.mkdir(out, { recursive: true });

if (mode === "record") {
  const evidence = path.join(ui, "evidence/walkthrough");
  execFileSync(process.execPath, [path.join(ui, "scripts/walkthrough.mjs"), "record", evidence], { stdio: "inherit", env: process.env });
  const acceptance = JSON.parse(await fs.readFile(path.join(evidence, "walkthrough.json")));
  const crypto = await import("node:crypto");
  acceptance.native_build_sha256 = crypto.createHash("sha256").update(await fs.readFile(path.join(demo, ".native/build.json"))).digest("hex");
  await fs.writeFile(path.join(out, "recordings.json"), JSON.stringify(acceptance, null, 2) + "\n");
} else {
  if (!["offline", "reconnect"].includes(mode)) throw new Error("Use record|offline|reconnect");
  const browser = await chromium.launch();
  try {
    const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });
    const writes = [], blocked = [];
    await context.route("**/*", route => {
      const url = new URL(route.request().url());
      if (["localhost", "127.0.0.1", "[::1]"].includes(url.hostname)) return route.continue();
      blocked.push(url.origin); return route.abort("blockedbyclient");
    });
    context.on("request", r => { if (!["GET", "HEAD"].includes(r.method())) writes.push(r.url()); });
    const page = await context.newPage();
    if (mode === "reconnect") {
      await page.goto(`${origin}/inspect`);
      await page.getByLabel("Actual query").selectOption("endpoint-freshness");
      await expect(page.getByText(/^Attached ·/)).toBeVisible();
      await expect(page.locator(".row-card")).toHaveCount(3);
      const before = await page.locator(".row-card").allTextContents();
      await context.setOffline(true);
      await expect(page.locator(".live-banner")).toContainText("UNKNOWN", { timeout: 15000 });
      await context.setOffline(false);
      await page.reload();
      await page.getByLabel("Actual query").selectOption("endpoint-freshness");
      await expect(page.getByText(/^Attached ·/)).toBeVisible();
      await expect(page.locator(".row-card")).toHaveCount(3);
      await expect(page.locator(".live-banner")).toContainText("Context ready");
      const after = await page.locator(".row-card").allTextContents();
      expect(writes).toEqual([]);
      await fs.writeFile(path.join(out, "browser-reconnect.json"), JSON.stringify({
        passed: true, at: new Date().toISOString(), before, after, blocked,
        boundary: "private diagnostic query reconnect; no audience console or backend mutation",
      }, null, 2) + "\n");
    } else {
      await page.goto(`${origin}/present`);
      const desk = await context.newPage();
      await desk.goto(`${origin}/presenter`);
      const captures = JSON.parse(await fs.readFile(path.join(ui, "public/replay/captures.json")));
      const clips = [];
      for (const clip of captures) {
        expect(clip.surface).toBe("grafana");
        const start = Date.now();
        await desk.getByText("Recordings (fallback)").click();
        await desk.getByRole("button", { name: clip.scene, exact: true }).click();
        const video = page.locator("video");
        await expect(page.getByText("REPLAY · NOT LIVE", { exact: true })).toBeVisible();
        await video.evaluate(async element => { element.muted = true; await element.play(); });
        await expect.poll(() => video.evaluate(element => element.currentTime)).toBeGreaterThan(.5);
        const switchToPlayMs = Date.now() - start;
        expect(switchToPlayMs).toBeLessThan(15000);
        await desk.getByText("All demo controls").click();
        for (const name of ["Start demo traffic", "Stop demo traffic", "Fail EU-A", "Submit one support request", "Apply to deployment", "Approve selected plan"]) {
          await expect(desk.locator(".controls").getByRole("button", { name, exact: true })).toBeDisabled();
        }
        await desk.getByText("All demo controls").click();
        await expect.poll(() => video.evaluate(element => element.ended), { timeout: 150000 }).toBe(true);
        expect(await video.evaluate(element => element.error)).toBeNull();
        clips.push({ scene: clip.scene, switch_to_play_ms: switchToPlayMs, surface: clip.surface });
        await desk.locator(".desk-alert").getByRole("button", { name: "Back to live" }).click();
        await desk.getByText("Recordings (fallback)").click();
      }
      const external = await page.evaluate(async () => {
        try { await fetch("https://offline-boundary.invalid/probe"); return "unexpected success"; }
        catch { return "blocked"; }
      });
      expect(external).toBe("blocked");
      expect(writes).toEqual([]);
      await fs.writeFile(path.join(out, "offline-browser.json"), JSON.stringify({
        passed: true, at: new Date().toISOString(), clips, writes, blocked,
        boundary: "non-loopback browser requests blocked; not a machine-wide network disconnect",
      }, null, 2) + "\n");
    }
    console.log(`${mode} check passed without backend mutations.`);
  } finally { await browser.close(); }
}
