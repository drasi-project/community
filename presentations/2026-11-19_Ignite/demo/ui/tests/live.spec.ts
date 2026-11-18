import { test, expect } from "@playwright/test";
import { healthSchema } from "../src/contracts";
import { products, dashboards } from "../../../presentation/scenes";

test("live projected slides contain no private controls", async ({ page }) => {
  const writes: string[] = [];
  page.on("request", r => { if (!["GET", "HEAD"].includes(r.method())) writes.push(r.url()); });
  for (const view of ["walkin", "title", "concepts", "drasi", "demo", "closing", "survey"]) {
    await page.goto(`/present?view=${view}&preview=1`);
    await expect(page.locator(`[data-slide="${view}"]`)).toBeVisible();
    await expect(page.locator(".desk, .live-banner, .controls-grid")).toHaveCount(0);
    await expect(page.locator("button:visible")).toHaveCount(0);
    await page.screenshot({ path: `evidence/live-${view}.png` });
  }
  expect(writes).toEqual([]);
});

test("the AI service, Drasi's answers, embedded Grafana and the operator's links are available", async ({ page, request }) => {
  const response = await request.get("/api/health");
  expect(response.ok()).toBe(true);
  healthSchema.parse(await response.json());
  await page.goto("/presenter");
  await expect(page.locator(".badge", { hasText: "READY" }).first()).toBeVisible();
  await page.getByText("Show something on the projector").click();
  await expect(page.getByRole("link", { name: "Grafana", exact: false }).first()).toHaveAttribute("href", products.grafana);
  const grafana = await request.get(dashboards.ops);
  expect(grafana.ok()).toBe(true);
  expect(grafana.headers()["x-frame-options"]).toBeUndefined();
  const query = await request.get("/server/api/v1/instances/ignite/queries/overdue-tasks");
  expect(query.ok()).toBe(true);
  expect((await query.json()).data.id).toBeTruthy();
});
