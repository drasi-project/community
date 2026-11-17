// The demo's ports, names and keys, from demo/ports.env (one place for all of them).
// Node side: Vite and Playwright configs, scripts and tests import this module.
// Browser side: Vite passes the values in as globalThis.__IGNITE_SETTINGS__ and
// src/settings.ts reads them. Importing this module also sets that global, so
// browser modules work under Playwright's Node runner.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const demo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export function load(file = process.env.IGNITE_SETTINGS || "ports.env") {
  const values = {};
  for (const raw of fs.readFileSync(path.resolve(demo, file), "utf8").split("\n")) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const at = line.indexOf("=");
    if (at < 0) throw new Error(`demo settings: expected KEY=VALUE, got ${line}`);
    const key = line.slice(0, at).trim();
    // An exported variable of the same name wins, as it does for compose.
    values[key] = process.env[key] || line.slice(at + 1).trim();
  }
  return values;
}

export const settings = load();
export const port = name => Number(settings[`IGNITE_${name}_PORT`]);
export const url = (name, host = "127.0.0.1") => `http://${host}:${port(name)}`;
globalThis.__IGNITE_SETTINGS__ ??= settings;
