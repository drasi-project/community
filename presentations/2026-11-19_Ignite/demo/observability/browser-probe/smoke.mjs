import { chromium } from "playwright";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { port } from "../../ui/settings.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const env = Object.fromEntries((await fs.readFile(path.join(root, ".env"), "utf8"))
  .split("\n").filter(line => line && !line.startsWith("#"))
  .map(line => {
    const split = line.indexOf("=");
    return [line.slice(0, split), line.slice(split + 1).replace(/^'|'$/g, "")];
  }));
const evidence = JSON.parse(await fs.readFile(
  path.join(root, "observability/evidence", process.argv[2] || "smoke.json"), "utf8"));
const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH,
  headless: true,
});
try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
  await context.route("**/*", route => {
    const url = new URL(route.request().url());
    return ["localhost", "127.0.0.1", "::1"].includes(url.hostname)
      ? route.continue()
      : route.abort("blockedbyclient");
  });
  const page = await context.newPage();
  await page.goto(`http://localhost:${port("GRAFANA")}/login`);
  await page.getByLabel("Email or username").fill("admin");
  await page.getByLabel("Password", { exact: true }).fill(env.GRAFANA_ADMIN_PASSWORD);
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await page.waitForURL(url => !url.pathname.endsWith("/login"), { timeout: 30000 });
  await page.goto(`http://localhost:${port("GRAFANA")}/d/integration-probe`);
  await page.getByText("Completed HTTP requests by status", { exact: true }).waitFor();
  await page.getByText("Last real request failed (1=yes, 0=no)", { exact: true }).waitFor();
  await page.screenshot({ path: path.join(root, "observability/evidence/grafana.png"), fullPage: true });

  await page.goto(`http://localhost:${port("LANGFUSE")}/auth/sign-in`);
  await page.getByLabel("Email", { exact: true }).fill(env.LANGFUSE_ADMIN_EMAIL);
  await page.getByLabel(/^Password/).fill(env.LANGFUSE_ADMIN_PASSWORD);
  await page.getByRole("button", { name: /sign in/i }).click();
  await page.waitForURL(url => !url.pathname.includes("sign-in"), { timeout: 30000 });
  await page.goto(evidence.success_request.trace_url);
  await page.getByText("GET /probe", { exact: true }).first().waitFor({ timeout: 60000 });
  await page.getByRole("tab", { name: "Metadata", exact: true }).last().click();
  await page.getByText(evidence.success_request.request_id, { exact: false }).first().waitFor();
  await page.screenshot({ path: path.join(root, "observability/evidence/langfuse.png"), fullPage: true });
  console.log("Verified real Grafana dashboard and correlated Langfuse trace in isolated browser; screenshots saved.");
} finally {
  await browser.close();
}
