// Renders presentation/diagrams/*.svg to PNG at 3840x2160, in Segoe UI when ops/template-assets.py
// has extracted it (otherwise in the system font):  node ui/scripts/render-diagrams.mjs  (from demo/)
import { chromium } from "@playwright/test";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { existsSync, readFileSync } from "node:fs";

const ui = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const diagrams = path.resolve(ui, "../../presentation/diagrams");
const fonts = path.join(ui, "public/template");
const face = (weight, file) => existsSync(path.join(fonts, file))
  ? `@font-face { font-family: "Segoe UI"; font-weight: ${weight}; src: url(data:font/woff;base64,${readFileSync(path.join(fonts, file)).toString("base64")}) format("woff"); }` : "";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 2 });
for (const file of (await fs.readdir(diagrams)).filter(name => name.endsWith(".svg"))) {
  const svg = await fs.readFile(path.join(diagrams, file), "utf8");
  await page.setContent(`<style>${face(400, "segoeui.woff")} ${face("600 700", "segoeui-semibold.woff")} body { margin: 0; }</style>${svg}`);
  await page.evaluate(async () => { await Promise.all([...document.fonts].map(font => font.load())); await document.fonts.ready; });
  await page.locator("svg").first().screenshot({ path: path.join(diagrams, file.replace(/\.svg$/, ".png")) });
  console.log("rendered", file);
}
await browser.close();
