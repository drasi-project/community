import { defineConfig } from "@playwright/test";

export default defineConfig({
  tsconfig: "./tests/tsconfig.json",
  testDir: "./tests", testMatch: "live.spec.ts", workers: 1, fullyParallel: false,
  timeout: 240000, expect: { timeout: 20000 },
  use: { baseURL: "http://127.0.0.1:18580", viewport: { width: 1280, height: 720 }, actionTimeout: 15000, navigationTimeout: 20000, trace: "retain-on-failure" },
  reporter: [["list"]],
});
