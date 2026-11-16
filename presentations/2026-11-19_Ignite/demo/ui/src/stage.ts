import { useEffect, useRef, useState } from "react";
import { z } from "zod";
import { scenes, shortRoute, type SlideId } from "../../../presentation/scenes";
import { selectionSchema, type Selection } from "./contracts";

const slideSchema = z.enum(["title", "concepts", "closing"]);
const schema = z.object({
  index: z.number().int().min(0).max(scenes.length - 1),
  slide: slideSchema, replay: z.string().nullable(),
  selection: selectionSchema.nullable(), short: z.boolean(),
});
const sharedSchema = z.object({ type: z.literal("stage"), slide: slideSchema, replay: z.string().nullable() });
export type Stage = z.infer<typeof schema>;
const initial: Stage = { index: 0, slide: "title", replay: null, selection: null, short: false };
const key = "ignite-presentation-stage-v2";

export function useStage(preview: boolean, presenter = false) {
  const requested = slideSchema.safeParse(new URLSearchParams(location.search).get("slide"));
  const [stage, setStage] = useState<Stage>(() => {
    if (!presenter || preview) return { ...initial, slide: requested.success ? requested.data : "title" };
    try { return schema.parse(JSON.parse(localStorage.getItem(key) ?? "{}")); }
    catch { return initial; }
  });
  const channel = useRef<BroadcastChannel | null>(null);
  const current = useRef(stage);
  current.current = stage;
  useEffect(() => {
    if (preview) return;
    const bus = new BroadcastChannel(key);
    const publish = () => bus.postMessage({ type: "stage", slide: current.current.slide, replay: current.current.replay });
    channel.current = bus;
    bus.onmessage = event => {
      if (event.data?.type === "request-stage") {
        if (presenter) publish();
        return;
      }
      const parsed = sharedSchema.safeParse(event.data);
      if (parsed.success) {
        const { slide, replay } = parsed.data;
        const next = { ...current.current, slide, replay };
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
      if ("slide" in patch || "replay" in patch) channel.current?.postMessage({ type: "stage", slide: next.slide, replay: next.replay });
    }
  }
  function go(index: number) { update({ index: Math.max(0, Math.min(scenes.length - 1, index)) }); }
  function advance(direction: number) {
    const route = scenes.map((scene, index) => ({ scene, index })).filter(({ scene }) => !current.current.short || shortRoute.includes(scene.id));
    const candidates = route.filter(item => direction > 0 ? item.index > current.current.index : item.index < current.current.index);
    go((direction > 0 ? candidates[0] : candidates.at(-1))?.index ?? current.current.index);
  }
  return {
    stage, update, go, advance,
    showSlide: (slide: SlideId) => update({ slide }),
    select: (selection: Selection) => update({ selection }),
  };
}
export type StageControl = ReturnType<typeof useStage>;
