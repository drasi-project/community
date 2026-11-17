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

// Drasi's answers the desk shows, each read straight from Drasi Server (contracts.md §4, §9).
// A browser allows six connections per host, shared by the desk and projector windows, so only the
// two row sets that change with every event are attached as streams; the single-value tiles are
// read from Server's maintained results once a second, in one batch.
export const attachedQueries = ["overdue-tasks", "plan-results"] as const;
export const tileQueries = ["tile-action", "tile-recovery", "tile-resilience", "tile-running", "tile-last-approval"] as const;
export const deskQueries = [...attachedQueries, ...tileQueries] as const;
export type DeskQuery = typeof deskQueries[number];
export type QueryState = { rows: Row[]; error: string | null; attached: boolean };

const resultsSchema = z.object({ success: z.literal(true), data: z.array(rowSchema) });
const serverRoot = "/server/api/v1/instances/ignite";

// Current results of several queries, re-read every second. A query is `attached` while its
// latest read succeeded and is under three seconds old.
export function useResults<T extends string>(names: readonly T[]): Record<T, QueryState> {
  const [state, setState] = useState<{ results: Partial<Record<T, Row[]>>; errors: Partial<Record<T, string>>; read: Partial<Record<T, number>> }>(
    { results: {}, errors: {}, read: {} });
  const [now, setNow] = useState(Date.now);
  const key = names.join(",");
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const controller = new AbortController();
    async function refresh() {
      const signal = AbortSignal.any([controller.signal, AbortSignal.timeout(3000)]);
      const reads = await Promise.allSettled(names.map(name =>
        request(`${serverRoot}/queries/${encodeURIComponent(name)}/results`, { signal }).then(data => resultsSchema.parse(data).data)));
      if (!active) return;
      const at = Date.now();
      setState(previous => {
        const next = { results: { ...previous.results }, errors: { ...previous.errors }, read: { ...previous.read } };
        reads.forEach((read, i) => {
          const name = names[i];
          if (read.status === "fulfilled") { next.results[name] = read.value; next.read[name] = at; delete next.errors[name]; }
          else next.errors[name] = `Unknown / unavailable: ${errorText(read.reason)}`;
        });
        return next;
      });
      setNow(at);
      timer = setTimeout(refresh, 1000);
    }
    void refresh();
    const clock = setInterval(() => setNow(Date.now()), 1000);
    return () => { active = false; clearTimeout(timer); clearInterval(clock); controller.abort(); };
  }, [key]);
  return Object.fromEntries(names.map(name => {
    const read = state.read[name];
    const attached = read !== undefined && !state.errors[name] && now - read < 3000;
    return [name, { rows: attached ? state.results[name] ?? [] : [], error: attached ? null : state.errors[name] ?? "Reading current results", attached }];
  })) as Record<T, QueryState>;
}

export function useLive() {
  const { health, error, received } = useHealth();
  const queries: Record<DeskQuery, QueryState> = {
    "overdue-tasks": useQuery("overdue-tasks"), "plan-results": useQuery("plan-results"), ...useResults(tileQueries),
  };
  const detached = deskQueries.filter(name => !queries[name].attached);
  return {
    health, error, received, queries,
    drasiError: detached.length ? `Drasi Server: no current result for ${detached.join(", ")}` : null,
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
    const root = "/server/api/v1/instances/ignite";
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
