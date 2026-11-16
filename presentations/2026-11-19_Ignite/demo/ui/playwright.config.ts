import { defineConfig } from "@playwright/test";

export default defineConfig({
  tsconfig: "./tests/tsconfig.json",
  testDir: "./tests", testMatch: "ui.spec.ts", fullyParallel: false, workers: 1,
  timeout: 30000, expect: { timeout: 10000 },
  use: { baseURL: "http://127.0.0.1:18581", viewport: { width: 1280, height: 720 }, trace: "retain-on-failure" },
  reporter: [["list"]],
  webServer: [
    { command: "node tests/fixture-server.mjs", port: 18582, reuseExistingServer: false },
    { command: "npm run build && npm exec vite -- preview --host 127.0.0.1 --port 18581 --strictPort --config vite.fixture.config.ts", url: "http://127.0.0.1:18581", reuseExistingServer: false, timeout: 60000 },
  ],
});
