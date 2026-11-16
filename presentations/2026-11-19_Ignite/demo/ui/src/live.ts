import { useEffect, useState } from "react";
import { contextSchema, decisionsSchema, errorText, healthSchema, request, rowSchema, type Context, type Health, type Row, type Decisions } from "./contracts";
import { z } from "zod";

export function useClock() {
  const [now, setNow] = useState(Date.now);
  useEffect(() => { const timer = setInterval(() => setNow(Date.now()), 250); return () => clearInterval(timer); }, []);
  return now;
}

export function useLive() {
  const [state, setState] = useState<{
    health: Health | null; context: Context | null; decisions: Decisions | null;
    error: string | null; decisionsError: string | null; received: number | null;
  }>({ health: null, context: null, decisions: null, error: "Connecting to local services", decisionsError: null, received: null });
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const controller = new AbortController();
    async function refresh() {
      const signal = AbortSignal.any([controller.signal, AbortSignal.timeout(5000)]);
      const [context, health, decisions] = await Promise.allSettled([
        request("/api/context", { signal }).then(data => contextSchema.parse(data)),
        request("/api/health", { signal }).then(data => healthSchema.parse(data)),
        request("/api/decisions", { signal }).then(data => decisionsSchema.parse(data)),
      ]);
      if (!active) return;
      const ok = context.status === "fulfilled" && health.status === "fulfilled";
      setState({
        context: context.status === "fulfilled" ? context.value : null,
        health: health.status === "fulfilled" ? health.value : null,
        decisions: decisions.status === "fulfilled" ? decisions.value : null,
        decisionsError: decisions.status === "rejected" ? errorText(decisions.reason) : null,
        error: ok ? null : [context, health].flatMap(result => result.status === "rejected" ? [errorText(result.reason)] : []).join("; "),
        received: ok ? Date.now() : null,
      });
      timer = setTimeout(refresh, 1500);
    }
    void refresh();
    return () => { active = false; clearTimeout(timer); controller.abort(); };
  }, []);
  return state;
}
export type Live = ReturnType<typeof useLive>;

const changesSchema = z.object({ results: z.array(z.object({
  type: z.enum(["ADD", "UPDATE", "DELETE"]), data: rowSchema.optional(), after: rowSchema.optional(),
})) });

export function applyFrame(current: Record<string, Row>, event: string, data: unknown, attached: boolean) {
  if (event === "snapshot") {
    if (attached) throw new Error("Duplicate initial snapshot");
    const replacement: Record<string, Row> = {};
    for (const row of z.array(rowSchema).parse(data)) {
      replacement[z.string().parse(row.id)] = row;
    }
    return replacement;
  }
  if (!attached) throw new Error("Query update arrived before initial snapshot");
  const next = { ...current };
  for (const change of changesSchema.parse(data).results) {
    const row = change.after ?? change.data;
    if (!row) throw new Error("Query difference is missing its row");
    const id = z.string().parse(row.id);
    if (change.type === "DELETE") delete next[id];
    else next[id] = row;
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
