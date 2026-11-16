import { defineConfig, type Plugin, type ViteDevServer, type PreviewServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

const proxy = {
  "/api": {
    target: "http://127.0.0.1:18500",
    rewrite: (path: string) => path.replace(/^\/api/, ""),
  },
  "/server": {
    target: "http://127.0.0.1:18421",
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
  resolve: { dedupe: ["react", "react-dom", "reveal.js"] },
  server: {
    proxy,
    fs: {
      allow: [fileURLToPath(new URL("../", import.meta.url)), fileURLToPath(new URL("../../presentation", import.meta.url))],
      deny: ["**/.env", "**/.env.*", "**/*.pem", "**/*.key", "**/.native/**", "**/node_modules/**/.cache/**"],
    },
  },
  preview: { proxy },
  build: { sourcemap: false },
});
