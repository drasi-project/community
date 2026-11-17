import { defineConfig, type Plugin, type ViteDevServer, type PreviewServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import { settings, url, port } from "./settings.mjs";

const proxy = {
  "/api": {
    target: url("WORKLOAD"),
    rewrite: (path: string) => path.replace(/^\/api/, ""),
  },
  "/server": {
    target: url("DRASI_API"),
    rewrite: (path: string) => path.replace(/^\/server/, ""),
  },
};

function localActions(): Plugin {
  const middleware = (server: ViteDevServer | PreviewServer) => {
    server.middlewares.use((request, response, next) => {
      if (!["GET", "HEAD", "OPTIONS"].includes(request.method ?? "GET")) {
        const origin = request.headers.origin;
        if (request.url?.startsWith("/server") ||
            request.headers["sec-fetch-site"] === "cross-site" ||
            (origin && origin !== `http://${request.headers.host}`)) {
          response.writeHead(403, { "Content-Type": "text/plain" });
          response.end("Local same-origin presenter actions only");
          return;
        }
      }
      next();
    });
  };
  return { name: "local-presenter-actions", configureServer: middleware, configurePreviewServer: middleware };
}

export default defineConfig({
  plugins: [localActions(), react()],
  // ports.env, read by src/settings.ts. It holds no secrets.
  define: { "globalThis.__IGNITE_SETTINGS__": JSON.stringify(settings) },
  resolve: { dedupe: ["react", "react-dom", "reveal.js", "qrcode"] },
  server: {
    host: "127.0.0.1", port: port("UI"), strictPort: true,
    proxy,
    fs: {
      allow: [fileURLToPath(new URL("../", import.meta.url)), fileURLToPath(new URL("../../presentation", import.meta.url))],
      deny: ["**/.env", "**/.env.*", "**/*.pem", "**/*.key", "**/.native/**", "**/node_modules/**/.cache/**"],
    },
  },
  preview: { host: "127.0.0.1", port: port("UI"), strictPort: true, proxy },
  build: { sourcemap: false },
});
