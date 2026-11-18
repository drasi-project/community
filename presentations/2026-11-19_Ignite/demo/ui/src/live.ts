import { useEffect, useState } from "react";
import { errorText, healthSchema, request, rowSchema, type Health, type Row } from "./contracts";
import { z } from "zod";

export function useClock() {
  const [now, setNow] = useState(Date.now);
  useEffect(() => { const timer = setInterval(() => setNow(Date.now()), 250); return () => clearInterval(timer); }, []);
  return now;
}

// The AI service's state of the demo (the application and demo driver), polled.
function useHealth() {
  const [state, setState] = useState<{ health: Health | null; error: string | null; received: number | null }>(
    { health: null, error: "Connecting to the AI service", received: null });
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const controller = new AbortController();
    async function refresh() {
      const signal = AbortSignal.any([controller.signal, AbortSignal.timeout(5000)]);
      try {
        const health = healthSchema.parse(await request("/api/health", { signal }));
        if (active) setState({ health, error: null, received: Date.now() });
      } catch (error) {
        if (active) setState({ health: null, error: `AI service: ${errorText(error)}`, received: null });
      }
      if (active) timer = setTimeout(refresh, 1500);
    }
    void refresh();
    return () => { active = false; clearTimeout(timer); controller.abort(); };
  }, []);
  return state;
}

// Drasi's answers the desk shows, from Drasi's SSE reaction `desk-stream` (contracts.md §4):
// one event stream per window carries every output, and each output's snapshot is loaded once.
export const deskQueries = [
  "overdue-tasks", "plan-results", "tile-action", "tile-recovery", "tile-resilience", "tile-running", "tile-last-approval",
] as const;
export type DeskQuery = typeof deskQueries[number];
export type QueryState = { rows: Row[]; error: string | null; attached: boolean };

const streamRoot = "/drasi-stream";
// Heartbeats arrive every 10 s; two missed ones mean the stream is not current.
const heartbeatLimitMs = 25000;

const changeSchema = z.object({
  operation: z.enum(["ADD", "REPLACE", "PATCH", "DELETE"]),
  identity: z.object({ namespace: z.string(), id: z.string() }),
  after: rowSchema.optional(),
  removedProperties: z.array(z.string()).optional(),
});
const frameSchema = z.union([
  z.object({ type: z.literal("heartbeat"), ts: z.number() }),
  z.object({ output: z.object({ alias: z.string() }), sequence: z.number(), changes: z.array(changeSchema) }),
]);
export type StreamFrame = z.infer<typeof frameSchema>;
type Change = z.infer<typeof changeSchema>;

// Applies one output's changes to its rows, keyed by Drasi's canonical identity.
export function applyChanges(rows: Record<string, Row>, changes: readonly Change[]): Record<string, Row> {
  const next = { ...rows };
  for (const change of changes) {
    const key = `${change.identity.namespace}/${change.identity.id}`;
    switch (change.operation) {
      case "ADD": case "REPLACE":
        if (!change.after) throw new Error(`${change.operation} without its row`);
        next[key] = change.after; break;
      case "PATCH": {
        const row = { ...(next[key] ?? {}), ...(change.after ?? {}) };
        for (const name of change.removedProperties ?? []) delete row[name];
        next[key] = row; break;
      }
      case "DELETE": delete next[key]; break;
    }
  }
  return next;
}

// The stream for some outputs: open /events first and buffer, load each snapshot, then apply the
// buffered frames after each snapshot's position and continue live (the sink's snapshot handover).
// Any error or silence reconnects and reloads every snapshot.
export function useDrasiStream<T extends string>(aliases: readonly T[]) {
  type State = { rows: Partial<Record<T, Row[]>>; connected: boolean; heartbeat: number | null; error: string | null };
  const [state, setState] = useState<State>({ rows: {}, connected: false, heartbeat: null, error: "Connecting to Drasi" });
  const [now, setNow] = useState(Date.now);
  const key = aliases.join(",");
  useEffect(() => {
    let active = true;
    let retry: ReturnType<typeof setTimeout>;
    let watchdog: ReturnType<typeof setTimeout>;
    let controller: AbortController;
    const wanted = new Set<string>(aliases);
    async function connect() {
      controller = new AbortController();
      const signal = controller.signal;
      setState({ rows: {}, connected: false, heartbeat: null, error: "Connecting to Drasi" });
      const touch = () => { clearTimeout(watchdog); watchdog = setTimeout(() => controller.abort("No Drasi stream bytes for 35 seconds"), 35000); };
      const rows: Record<string, Record<string, Row>> = {};
      const positions: Record<string, number> = {};
      let buffered: StreamFrame[] | null = [];
      let heartbeat: number | null = null;
      const publish = () => active && setState({
        rows: Object.fromEntries(Object.entries(rows).map(([alias, map]) => [alias, Object.values(map)])) as Partial<Record<T, Row[]>>,
        connected: true, heartbeat, error: null,
      });
      const apply = (frame: StreamFrame) => {
        if ("type" in frame) { heartbeat = Date.now(); return; }
        const alias = frame.output.alias;
        if (!wanted.has(alias) || !(alias in rows) || frame.sequence <= positions[alias]) return;
        rows[alias] = applyChanges(rows[alias], frame.changes);
        positions[alias] = frame.sequence;
      };
      try {
        touch();
        const response = await fetch(`${streamRoot}/events`, { signal, headers: { Accept: "text/event-stream" } });
        if (!response.ok || !response.body) throw new Error(`Drasi stream HTTP ${response.status}`);
        heartbeat = Date.now();
        const reader = response.body.getReader();
        const snapshots = Promise.all(aliases.map(async alias => {
          const snapshot = await fetch(`${streamRoot}/snapshot/${encodeURIComponent(alias)}`, { signal });
          if (!snapshot.ok) throw new Error(`${alias} snapshot HTTP ${snapshot.status}`);
          const position = Number(snapshot.headers.get("x-drasi-output-position"));
          if (!Number.isFinite(position)) throw new Error(`${alias} snapshot has no position`);
          return { alias, position, changes: z.array(changeSchema).parse(await snapshot.json()) };
        })).then(loaded => {
          for (const { alias, position, changes } of loaded) { rows[alias] = applyChanges({}, changes); positions[alias] = position; }
          for (const frame of buffered ?? []) apply(frame);
          buffered = null;
          publish();
        });
        snapshots.catch(error => controller.abort(errorText(error)));
        const decoder = new TextDecoder();
        let buffer = "";
        while (active) {
          const chunk = await reader.read();
          if (chunk.done) throw new Error(signal.aborted ? String(signal.reason) : "Drasi stream disconnected");
          touch();
          buffer = (buffer + decoder.decode(chunk.value, { stream: true })).replace(/\r\n/g, "\n");
          if (buffer.length > 4_000_000) throw new Error("Drasi stream frame exceeds 4 MB");
          let boundary: number;
          let changed = false;
          while ((boundary = buffer.indexOf("\n\n")) !== -1) {
            const lines = buffer.slice(0, boundary).split("\n"); buffer = buffer.slice(boundary + 2);
            const data = lines.filter(line => line.startsWith("data:")).map(line => line.slice(5).trimStart()).join("\n");
            if (!data) continue;
            const frame = frameSchema.parse(JSON.parse(data));
            if (buffered) buffered.push(frame);
            else { apply(frame); changed = true; }
          }
          if (changed) publish();
        }
      } catch (error) {
        if (active) {
          setState({ rows: {}, connected: false, heartbeat: null, error: `Unknown / disconnected: ${signal.aborted && signal.reason ? String(signal.reason) : errorText(error)}` });
          retry = setTimeout(connect, 2000);
        }
      } finally { clearTimeout(watchdog); }
    }
    void connect();
    const clock = setInterval(() => setNow(Date.now()), 1000);
    return () => { active = false; clearTimeout(retry); clearTimeout(watchdog); clearInterval(clock); controller?.abort(); };
  }, [key]);
  const current = state.connected && state.heartbeat !== null && now - state.heartbeat < heartbeatLimitMs;
  const queries = Object.fromEntries(aliases.map(alias => {
    const rows = state.rows[alias];
    const attached = current && rows !== undefined;
    return [alias, { rows: attached ? rows : [], attached, error: attached ? null : state.error ?? (current ? "Loading snapshot" : "Drasi stream is silent") }];
  })) as Record<T, QueryState>;
  return { queries, connected: current, error: current ? null : state.error ?? "Drasi stream is silent" };
}

export function useLive() {
  const { health, error, received } = useHealth();
  const stream = useDrasiStream(deskQueries);
  const missing = deskQueries.filter(name => !stream.queries[name].attached);
  return {
    health, error, received, queries: stream.queries,
    drasiError: !stream.connected ? `Drasi: ${stream.error}` : missing.length ? `Drasi: no snapshot yet for ${missing.join(", ")}` : null,
  };
}
export type Live = ReturnType<typeof useLive>;

const diffSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("ADD"), data: rowSchema }),
  z.object({ type: z.literal("DELETE"), data: rowSchema }),
  z.object({ type: z.literal("UPDATE"), before: rowSchema.optional(), after: rowSchema, data: rowSchema.optional() }),
  z.object({ type: z.literal("aggregation"), before: rowSchema.nullish(), after: rowSchema }),
  z.object({ type: z.literal("noop") }),
]);
const changesSchema = z.object({ results: z.array(diffSchema) });

// Rows with an `id` are keyed by it; others (aggregates, tile queries) by their whole content,
// so a delete or the "before" of an update removes exactly the row it names.
export function rowKey(row: Row): string {
  if (typeof row.id === "string") return row.id;
  return JSON.stringify(Object.keys(row).sort().map(key => [key, row[key]]));
}

export function applyFrame(current: Record<string, Row>, event: string, data: unknown, attached: boolean) {
  if (event === "snapshot") {
    if (attached) throw new Error("Duplicate initial snapshot");
    const replacement: Record<string, Row> = {};
    for (const row of z.array(rowSchema).parse(data)) replacement[rowKey(row)] = row;
    return replacement;
  }
  if (!attached) throw new Error("Query update arrived before initial snapshot");
  const next = { ...current };
  for (const change of changesSchema.parse(data).results) {
    switch (change.type) {
      case "ADD": next[rowKey(change.data)] = change.data; break;
      case "DELETE": delete next[rowKey(change.data)]; break;
      case "UPDATE": case "aggregation":
        if (change.before) delete next[rowKey(change.before)];
        next[rowKey(change.after)] = change.after;
        break;
      case "noop": break;
    }
  }
  return next;
}

// One selected query uses one ordered attachment, never snapshot + unrelated stream.
export function useQuery(name: string) {
  const [state, setState] = useState<{ rows: Row[]; error: string | null; attached: boolean; events: number; lastEvent: number | null }>({
    rows: [], error: "Attaching snapshot-first", attached: false, events: 0, lastEvent: null,
  });
  useEffect(() => {
    let active = true;
    let retry: ReturnType<typeof setTimeout>;
    let watchdog: ReturnType<typeof setTimeout>;
    let controller: AbortController;
    const root = "/server/api/v1/instances/ai-ops";
    async function attach() {
      controller = new AbortController();
      const signal = controller.signal;
      setState({ rows: [], error: "Attaching snapshot-first", attached: false, events: 0, lastEvent: null });
      const touch = () => { clearTimeout(watchdog); watchdog = setTimeout(() => controller.abort("No Server stream bytes for 35 seconds"), 35000); };
      try {
        touch();
        const response = await fetch(`${root}/queries/${encodeURIComponent(name)}/attach`, { signal });
        if (!response.ok || !response.body) throw new Error(`Query attachment HTTP ${response.status}`);
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = "", attached = false, current: Record<string, Row> = {}, events = 0;
        while (active) {
          const chunk = await reader.read();
          if (chunk.done) throw new Error("Query stream disconnected");
          touch();
          buffer = (buffer + decoder.decode(chunk.value, { stream: true })).replace(/\r\n/g, "\n");
          if (buffer.length > 4_000_000) throw new Error("Query frame exceeds 4 MB");
          let boundary: number;
          while ((boundary = buffer.indexOf("\n\n")) !== -1) {
            const frame = buffer.slice(0, boundary); buffer = buffer.slice(boundary + 2);
            const lines = frame.split("\n");
            const event = lines.find(line => line.startsWith("event:"))?.slice(6).trim() ?? "message";
            const data = lines.filter(line => line.startsWith("data:")).map(line => line.slice(5).trimStart()).join("\n");
            if (!data) continue;
            current = applyFrame(current, event, JSON.parse(data), attached);
            attached = true;
            if (active) setState({ rows: Object.values(current), error: null, attached, events: ++events, lastEvent: Date.now() });
          }
        }
      } catch (error) {
        if (active) {
          setState({ rows: [], error: `Unknown / disconnected: ${errorText(error)}`, attached: false, events: 0, lastEvent: null });
          retry = setTimeout(attach, 2000);
        }
      } finally { clearTimeout(watchdog); }
    }
    void attach();
    return () => { active = false; clearTimeout(retry); clearTimeout(watchdog); controller?.abort(); };
  }, [name]);
  return state;
}
