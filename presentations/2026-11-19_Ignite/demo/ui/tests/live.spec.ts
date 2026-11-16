import { test, expect } from "@playwright/test";
import { healthSchema } from "../src/contracts";
import { products } from "../../../presentation/scenes";

test("live projected slides contain no private controls or monitoring facade", async ({ page }) => {
  const writes: string[] = [];
  page.on("request", r => { if (!["GET", "HEAD"].includes(r.method())) writes.push(r.url()); });
  await page.goto("/present?preview=1");
  await expect(page.locator(".title-slide")).toBeVisible();
  await expect(page.locator(".view-tabs,.live-banner,.controls-grid")).toHaveCount(0);
  await page.screenshot({ path: "evidence/live-title.png" });
  await page.getByRole("button", { name: "Next slide", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Keep customer support running", exact: true })).toBeVisible();
  await expect(page.getByRole("link", { name: "Live demo · Grafana", exact: false })).toHaveAttribute("href", products.grafana);
  await page.screenshot({ path: "evidence/live-concepts.png" });
  await page.goto("/present?slide=closing");
  await expect(page.locator(".closing-slide")).toBeVisible();
  await page.screenshot({ path: "evidence/live-closing.png" });
  expect(writes).toEqual([]);
});

test("actual Drasi context and product links are available to the private operator", async ({ page, request }) => {
  const response = await request.get("/api/health");
  expect(response.ok()).toBe(true);
  const health = healthSchema.parse(await response.json());
  expect(health.continuous_context).toBe("ready");
  await page.goto("/presenter");
  await expect(page.getByRole("button", { name: "Submit support request", exact: true })).toBeEnabled();
  await expect(page.getByRole("link", { name: "Grafana · Policy and recovery", exact: false })).toHaveAttribute("href", products.decisions);
  const graph = await request.get("/server/api/v1/instances/ignite/computation");
  expect(graph.ok()).toBe(true);
  const query = await request.get("/server/api/v1/instances/ignite/queries/overdue-tasks");
  expect(query.ok()).toBe(true);
  expect((await query.json()).data.id).toBeTruthy();
});
