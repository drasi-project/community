import { z } from "zod";

export const rowSchema = z.record(z.string(), z.unknown());
export type Row = z.infer<typeof rowSchema>;
export const taskSchema = z.object({ task: rowSchema, attempts: z.array(rowSchema) });
export type TaskDetail = z.infer<typeof taskSchema>;

// The AI service's own view of the demo (contracts.md §10, demo controls). It is the application
// and the demo driver: it never reports Drasi's answers.
export const healthSchema = z.object({
  run_epoch: z.string(), revision: z.number(), pending: z.number(),
  task_target_seconds: z.number(), report_expiry_seconds: z.number(),
  traffic: z.object({ running: z.boolean(), submitted: z.number(), ends_ms: z.number(), error: z.string().nullable(), interval_seconds: z.number() }).optional(),
  deployments: z.array(z.object({
    id: z.string(), region: z.string(), model: z.string(), capacity_tpm: z.number(),
    mode: z.string(), reporting: z.boolean(),
  })),
  workloads: z.array(z.object({
    id: z.string(), data_class: z.string(), region_requirement: z.string(), model: z.string(),
    demand_tpm: z.number(), deployment: z.string(),
  })),
  destinations: z.record(z.string(), z.array(z.string())),
});
export type Health = z.infer<typeof healthSchema>;

// Drasi's `Plan` element, as the plan-results query returns it (contracts.md §5).
export const planSchema = z.object({
  id: z.string(), state: z.enum(["feasible", "infeasible", "unknown"]),
  candidate_id: z.string(), basis: z.string(), revision: z.number(), run_epoch: z.string(),
  assignments: z.union([z.string(), z.record(z.string(), z.string())])
    .transform(value => z.record(z.string(), z.string()).parse(typeof value === "string" ? JSON.parse(value || "{}") : value)),
  moves: z.number(), reason: z.string(),
}).passthrough();
export type Plan = z.infer<typeof planSchema>;
export function readPlan(rows: Row[]): { plan: Plan | null; error: string | null } {
  const row = rows.find(r => r.id === "plan");
  if (!row) return { plan: null, error: "plan-results: no Plan row" };
  const parsed = planSchema.safeParse(row);
  return parsed.success ? { plan: parsed.data, error: null } : { plan: null, error: `plan-results: ${errorText(parsed.error)}` };
}

// A tile query's single value (contracts.md §9): a number, or null for Unknown.
export function tileValue(rows: Row[], match: (row: Row) => boolean = () => true): number | null {
  const value = rows.find(match)?.value;
  return typeof value === "number" && value >= 0 ? value : null;
}

// The plan the presenter took to the approval tool. The ID is the approval's ID.
export const selectionSchema = z.object({
  approval_id: z.string(), candidate_id: z.string(), basis: z.string(), run_epoch: z.string(), revision: z.number(),
  assignments: z.record(z.string(), z.string()).optional(), selected_ms: z.number().optional(),
});
export type Selection = z.infer<typeof selectionSchema>;

export function text(value: unknown): string {
  return value == null ? "unknown" : typeof value === "object" ? JSON.stringify(value) : String(value);
}
export function errorText(error: unknown): string {
  if (error instanceof z.ZodError) return error.issues.slice(0, 3).map(issue => `${issue.path.join(".")}: ${issue.message}`).join("; ");
  return error instanceof Error ? error.message : String(error);
}

export async function request(path: string, init?: RequestInit): Promise<unknown> {
  const response = await fetch(path, { ...init, signal: init?.signal ?? AbortSignal.timeout(6000) });
  const body = await response.text();
  if (!response.ok) throw new Error(`${response.status} ${body || response.statusText}`);
  try { return JSON.parse(body); }
  catch { throw new Error(`${path}: response was not JSON`); }
}
export const post = (path: string, body: unknown) =>
  request(`/api${path}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
