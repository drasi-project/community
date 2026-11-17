import { defineConfig } from "@playwright/test";
import { url, port } from "./settings.mjs";

export default defineConfig({
  tsconfig: "./tests/tsconfig.json",
  testDir: "./tests", testMatch: "ui.spec.ts", fullyParallel: false, workers: 1,
  timeout: 30000, expect: { timeout: 10000 },
  use: { baseURL: url("UI_TEST"), viewport: { width: 1280, height: 720 }, trace: "retain-on-failure" },
  reporter: [["list"]],
  webServer: [
    { command: "node tests/fixture-server.mjs", port: port("UI_FIXTURE"), reuseExistingServer: false },
    { command: "npm run build && npm exec vite -- preview --config vite.fixture.config.ts", url: url("UI_TEST"), reuseExistingServer: false, timeout: 60000 },
  ],
});
