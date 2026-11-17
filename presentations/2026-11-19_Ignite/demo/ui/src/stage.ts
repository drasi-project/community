import { useEffect, useRef, useState } from "react";
import { settings } from "./settings";
import { z } from "zod";
import { scenes, allSlides, type SlideId, type StageView } from "../../../presentation/scenes";
import { selectionSchema, type Selection } from "./contracts";

const slideIds = allSlides.map(slide => slide.id) as [SlideId, ...SlideId[]];
export const viewSchema = z.enum([...slideIds, "ops", "plan", "request"] as [StageView, ...StageView[]]);
const schema = z.object({
  cue: z.number().int().min(0).max(scenes.length - 1),
  beat: z.number().int().min(0),
  view: viewSchema, trace: z.string().nullable(), replay: z.string().nullable(),
  selection: selectionSchema.nullable(), short: z.boolean(),
  // Recording mode: the rest of the talk uses recordings instead of the live demo.
  fallback: z.boolean().default(false),
});
// What the desk tells the projector, and what the projector tells the desk.
const showSchema = z.object({ type: z.literal("stage"), view: viewSchema, trace: z.string().nullable(), replay: z.string().nullable() });
const keySchema = z.object({ type: z.literal("key"), key: z.string(), id: z.string() });
const ackSchema = z.object({ type: z.literal("key-ack"), id: z.string() });
export type Stage = z.infer<typeof schema>;
const initial: Stage = { cue: 0, beat: 0, view: "title", trace: null, replay: null, selection: null, short: false, fallback: false };
const key = settings.storageKey;

// The view requested in the address (?view=… or ?slide=…), if any.
function requestedView() {
  const params = new URLSearchParams(location.search);
  return viewSchema.safeParse(params.get("view") ?? params.get("slide"));
}

export function useStage(preview: boolean, presenter = false) {
  const requested = requestedView();
  const [stage, setStage] = useState<Stage>(() => {
    if (!presenter || preview) return { ...initial, view: requested.success ? requested.data : "title" };
    try { return schema.parse(JSON.parse(localStorage.getItem(key) ?? "{}")); }
    catch { return initial; }
  });
  const channel = useRef<BroadcastChannel | null>(null);
  // Keys sent to the desk and not yet acknowledged.
  const pending = useRef(new Set<string>());
  const current = useRef(stage);
  current.current = stage;
  // The desk sets this to handle keys forwarded from the projector.
  const keyHandler = useRef<((pressed: string) => void) | undefined>(undefined);
  useEffect(() => {
    if (preview) return;
    const bus = new BroadcastChannel(key);
    const publish = () => bus.postMessage({ type: "stage", view: current.current.view, trace: current.current.trace, replay: current.current.replay });
    channel.current = bus;
    bus.onmessage = event => {
      if (event.data?.type === "request-stage") { if (presenter) publish(); return; }
      const pressed = keySchema.safeParse(event.data);
      if (pressed.success) {
        if (presenter && keyHandler.current) { keyHandler.current(pressed.data.key); bus.postMessage({ type: "key-ack", id: pressed.data.id }); }
        return;
      }
      const ack = ackSchema.safeParse(event.data);
      if (ack.success) { pending.current.delete(ack.data.id); return; }
      const shown = showSchema.safeParse(event.data);
      if (shown.success && !presenter) {
        const next = { ...current.current, ...shown.data };
        current.current = next;
        setStage(next);
      }
    };
    if (!presenter) {
      if (requested.success) publish();
      else bus.postMessage({ type: "request-stage" });
    }
    return () => { bus.close(); channel.current = null; };
  }, [preview, presenter]);
  function update(patch: Partial<Stage>) {
    const next = schema.parse({ ...current.current, ...patch });
    current.current = next;
    setStage(next);
    if (!preview) {
      if (presenter) localStorage.setItem(key, JSON.stringify(next));
      if ("view" in patch || "replay" in patch || "trace" in patch)
        channel.current?.postMessage({ type: "stage", view: next.view, trace: next.trace, replay: next.replay });
    }
  }
  return {
    stage, update, keyHandler,
    show: (view: StageView, trace: string | null = null) => update({ view, trace }),
    select: (selection: Selection | null) => update({ selection }),
    // The projector forwards the presenter's keys (clicker, arrows) to the desk. If no desk answers
    // within half a second, onUnanswered runs, so the slides still work with no desk open.
    sendKey: (pressed: string, onUnanswered: () => void) => {
      const id = crypto.randomUUID();
      pending.current.add(id);
      channel.current?.postMessage({ type: "key", key: pressed, id });
      setTimeout(() => { if (pending.current.delete(id)) onUnanswered(); }, 500);
    },
  };
}
export type StageControl = ReturnType<typeof useStage>;
