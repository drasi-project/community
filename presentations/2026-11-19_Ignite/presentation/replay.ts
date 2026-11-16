export type ReplayClip = {
  scene: string; title: string; src: string | null;
  capturedAt: string | null; runEpoch: string | null; provenance: string;
};
const scenes: readonly ReplayClip[] = [
  { scene: "failure", title: "HTTP failure and unchanged-task deadline", src: null, capturedAt: null, runEpoch: null, provenance: "Unavailable: no qualified recording captured." },
  { scene: "recovery", title: "Approved allocation and observed recovery", src: null, capturedAt: null, runEpoch: null, provenance: "Unavailable: no qualified recording captured." },
  { scene: "policy", title: "Obsolete approval rejected", src: null, capturedAt: null, runEpoch: null, provenance: "Unavailable: no qualified recording captured." },
  { scene: "resilience", title: "Healthy but fragile, then restored headroom", src: null, capturedAt: null, runEpoch: null, provenance: "Unavailable: no qualified recording captured." },
];
export const replayClips: readonly ReplayClip[] = scenes.map(clip => {
  const recorded: readonly (Omit<ReplayClip, "title"> & { surface?: string })[] = captures;
  const capture = recorded.find(item => item.scene === clip.scene && item.surface === "grafana");
  return capture ? { ...clip, ...capture } : clip;
});
import captures from "../demo/ui/public/replay/captures.json" with { type: "json" };
