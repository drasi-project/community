import { defineConfig, mergeConfig } from "vite";
import base from "./vite.config";

// Isolated browser-test process only. Production never loads fixtures.
export default mergeConfig(base, defineConfig({
  preview: { proxy: {
    "/api": { target: "http://127.0.0.1:18582", rewrite: (path: string) => path.replace(/^\/api/, "") },
    "/server": { target: "http://127.0.0.1:18582", rewrite: (path: string) => path.replace(/^\/server/, "") },
  } },
}));
