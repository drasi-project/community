// Simple line icons for the slides and "how it works" cards (24-unit grid, drawn in currentColor).
const paths: Record<string, string> = {
  pg: "M4 6c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3zm0 0v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3",
  otel: "M2 12h4l3-7 4 14 3-7h6",
  query: "M10 4a6 6 0 1 0 0 12 6 6 0 0 0 0-12zm4.5 10.5L20 20",
  clock: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zm0 4v5l3.5 2",
  model: "M12 3l8 3v6c0 4.5-3.4 8-8 9-4.6-1-8-4.5-8-9V6zm-3.5 9l2.5 2.5 4.5-5",
  prometheus: "M4 20V10m5 10V4m5 16v-7m5 7V7M2 20h20",
  mattermost: "M4 4h16v12H9l-5 4zM8 9h8M8 12h5",
  api: "M8 6l-6 6 6 6m8-12l6 6-6 6M14 4l-4 16",
  k8s: "M12 2l8.7 5v10L12 22l-8.7-5V7zm0 5a5 5 0 1 0 0 10 5 5 0 0 0 0-10z",
  app: "M3 4h18v16H3zM3 8h18",
  server: "M4 3h16v7H4zm0 11h16v7H4zM8 6.5h.01M8 17.5h.01",
  user: "M12 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM4 21c0-4.4 3.6-7 8-7s8 2.6 8 7",
  poll: "M20 12a8 8 0 1 1-2.3-5.7M20 4v5h-5",
  bolt: "M13 2L4 14h7l-1 8 9-12h-7z",
  check: "M4 12l5 5L20 6",
  cross: "M5 5l14 14M19 5L5 19",
  code: "M8 6l-6 6 6 6m8-12l6 6-6 6",
  ghost: "M5 21V10a7 7 0 0 1 14 0v11l-2.3-2-2.4 2-2.3-2-2.3 2-2.4-2zM9 10h.01M15 10h.01",
};
export function Icon({ name, size = 28 }: { name: string; size?: number }) {
  return <svg className="icon" width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor"
    strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d={paths[name] ?? paths.app} /></svg>;
}
