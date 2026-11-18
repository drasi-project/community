// Browser view of demo/ports.env. Vite defines globalThis.__IGNITE_SETTINGS__;
// under Playwright's Node runner, importing ../settings.mjs first sets it.
declare global {
  // eslint-disable-next-line no-var
  var __IGNITE_SETTINGS__: Record<string, string> | undefined;
}
const values = globalThis.__IGNITE_SETTINGS__;
if (!values) throw new Error("Demo settings missing: load demo/ports.env through vite.config.ts or ui/settings.mjs");
const port = (name: string) => values[`IGNITE_${name}_PORT`];

export const settings = {
  storageKey: values.IGNITE_STORAGE_KEY,
  // Grafana and Langfuse are opened as localhost, as their root URLs are configured.
  grafana: `http://localhost:${port("GRAFANA")}`,
  langfuse: `http://localhost:${port("LANGFUSE")}`,
  // The incident channel (#incident-ai-support); /mattermost/sign-in on the AI service signs the browser in.
  mattermost: `http://localhost:${port("MATTERMOST")}`,
  mattermostSignIn: `http://localhost:${port("WORKLOAD")}/mattermost/sign-in`,
  server: `http://127.0.0.1:${port("DRASI_API")}`,
} as const;
