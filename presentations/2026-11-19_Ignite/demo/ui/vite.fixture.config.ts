import { defineConfig, mergeConfig } from "vite";
import base from "./vite.config";
import { url, port } from "./settings.mjs";

// Isolated browser-test process only. Production never loads fixtures.
export default mergeConfig(base, defineConfig({
  preview: { port: port("UI_TEST"), proxy: {
    "/api": { target: url("UI_FIXTURE"), rewrite: (path: string) => path.replace(/^\/api/, "") },
    "/server": { target: url("UI_FIXTURE"), rewrite: (path: string) => path.replace(/^\/server/, "") },
    "/drasi-stream": { target: url("UI_FIXTURE"), rewrite: (path: string) => path.replace(/^\/drasi-stream/, "/drasi-stream") },
  } },
}));
