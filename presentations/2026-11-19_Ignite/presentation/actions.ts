import { post, readDecision, type Selection } from "../demo/ui/src/contracts";
import type { Live } from "../demo/ui/src/live";
import type { ActionId } from "./notes";

type Health = NonNullable<Live["health"]>;

// What each demo button sends to the AI service. Used by the presenter desk, and by the
// projector on its own when no desk is open.
export function request(id: ActionId, health: Health, selection: Selection | null): { path: string; body: unknown } | null {
  const configure = (change: (model: Health["model"]) => void) => {
    const model = structuredClone(health.model);
    change(model);
    return { path: "/configuration", body: { run_epoch: health.run_epoch, revision: health.revision, model } };
  };
  const euA = (budget: number) => configure(model => {
    const endpoint = model.endpoints.find(e => e.id === "EU-A")!;
    endpoint.mode = "ok"; endpoint.reporting = true; endpoint.budget_tpm = budget;
  });
  switch (id) {
    case "start-traffic": return { path: "/traffic", body: { run_epoch: health.run_epoch, running: true } };
    case "stop-traffic": return { path: "/traffic", body: { run_epoch: health.run_epoch, running: false } };
    case "fail-eu-a": return { path: "/control", body: { run_epoch: health.run_epoch, revision: health.revision, endpoint: "EU-A", action: "fail" } };
    case "select-plan": return null;
    case "approve-plan": return selection
      ? { path: "/approvals", body: { command_id: selection.command_id, candidate_id: selection.candidate_id, basis: selection.basis } } : null;
    case "summaries-europe": return configure(model => { model.policy!.destinations.batch = ["EU-A", "EU-B"]; });
    case "summaries-anywhere": return configure(model => { model.policy!.destinations.batch = ["EU-A", "EU-B", "US-C"]; });
    case "eu-a-half": return euA(50000);
    case "eu-a-full": return euA(100000);
  }
}

// A copy of the planner's current proposal, ready to approve.
export function selectPlan(live: Pick<Live, "context">): Selection | null {
  const candidate = readDecision(live.context, "allocation-results").data;
  if (!candidate?.valid) return null;
  return { command_id: crypto.randomUUID(), candidate_id: candidate.candidate_id, basis: candidate.basis,
    run_epoch: candidate.run_epoch, revision: candidate.revision, assignments: candidate.assignments, selected_ms: Date.now() };
}

// Press one button without the desk. Returns the selection to keep for a later approval.
export async function press(id: ActionId, live: Live, selection: Selection | null): Promise<Selection | null> {
  if (!live.health) return selection;
  if (id === "select-plan") return selectPlan(live) ?? selection;
  const sent = request(id, live.health, selection);
  if (sent) await post(sent.path, sent.body).catch(() => undefined);
  return id === "approve-plan" ? null : selection;
}
