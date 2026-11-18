import { defineConfig, type Plugin, type ViteDevServer, type PreviewServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import fs from "node:fs";
import path from "node:path";
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
  // Drasi's SSE reaction (desk-stream): the desk's and projector's live answers.
  "/drasi-stream": {
    target: url("DRASI_SSE"),
    rewrite: (path: string) => path.replace(/^\/drasi-stream/, ""),
  },
};

function localActions(): Plugin {
  const middleware = (server: ViteDevServer | PreviewServer) => {
    server.middlewares.use((request, response, next) => {
      if (!["GET", "HEAD", "OPTIONS"].includes(request.method ?? "GET")) {
        const origin = request.headers.origin;
        if (request.url?.startsWith("/server") || request.url?.startsWith("/drasi-stream") ||
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

// The Ignite template's artwork and fonts (ops/template-assets.py, git-ignored), served from
// public/template as they are now, so they work without a rebuild; a missing file is a 404.
function templateAssets(): Plugin {
  const folder = fileURLToPath(new URL("./public/template/", import.meta.url));
  const types: Record<string, string> = { ".svg": "image/svg+xml", ".jpg": "image/jpeg", ".png": "image/png", ".woff": "font/woff", ".json": "application/json" };
  const middleware = (server: ViteDevServer | PreviewServer) => {
    server.middlewares.use("/template", (request, response) => {
      const name = path.basename(decodeURIComponent((request.url ?? "").split("?")[0]));
      const file = path.join(folder, name);
      if (!name || !(path.extname(name) in types) || !fs.existsSync(file)) {
        response.writeHead(404, { "Content-Type": "text/plain" });
        response.end("Template asset missing: run python3 ops/template-assets.py");
        return;
      }
      response.writeHead(200, { "Content-Type": types[path.extname(name)], "Cache-Control": "no-cache" });
      fs.createReadStream(file).pipe(response);
    });
  };
  return { name: "template-assets", configureServer: middleware, configurePreviewServer: middleware };
}

export default defineConfig({
  plugins: [localActions(), templateAssets(), react()],
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
