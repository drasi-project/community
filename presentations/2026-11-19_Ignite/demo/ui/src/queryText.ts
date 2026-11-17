// The bundled source of every continuous query in demo/drasi, by query name (file name).
// Globbed, so the list follows whatever queries the graph ships.
const files = import.meta.glob("../../drasi/*.cypher", { query: "?raw", import: "default", eager: true }) as Record<string, string>;

export const queryText: Record<string, string> = Object.fromEntries(Object.entries(files)
  .map(([path, text]) => [path.split("/").at(-1)!.replace(/\.cypher$/, ""), text])
  .sort(([a], [b]) => a.localeCompare(b)));
