import { z } from "zod";

export const rowSchema = z.record(z.string(), z.unknown());
export type Row = z.infer<typeof rowSchema>;
export const taskSchema = z.object({ task: rowSchema, attempts: z.array(rowSchema) });
export type TaskDetail = z.infer<typeof taskSchema>;
export const endpointSchema = z.object({
  id: z.string(), region: z.string(), model: z.string(), budget_tpm: z.number(),
  mode: z.string(), reporting: z.boolean(),
});
export const modelSchema = z.object({
  endpoints: z.array(endpointSchema),
  workloads: z.array(z.object({
    id: z.string(), destination: z.string(), reservation_tpm: z.number(),
    model: z.string(), region: z.string(),
  })),
  policy: z.object({ revision: z.number(), destinations: z.record(z.string(), z.array(z.string())) }).optional(),
});
export const healthSchema = z.object({
  run_epoch: z.string(), revision: z.number(), model: modelSchema, pending: z.number(),
  continuous_context: z.enum(["ready", "unknown"]), streams_attached: z.boolean(),
  task_target_seconds: z.number(), report_expiry_seconds: z.number(),
  traffic: z.object({ running: z.boolean(), submitted: z.number(), ends_ms: z.number(), error: z.string().nullable(), interval_seconds: z.number() }).optional(),
});
export type Health = z.infer<typeof healthSchema>;
export const contextSchema = z.object({
  streams_attached: z.boolean(),
  queries: z.record(z.string(), z.record(z.string(), rowSchema)),
});
export type Context = z.infer<typeof contextSchema>;
export const decisionSchema = z.object({
  run_epoch: z.string(), revision: z.number(), policy_revision: z.number(),
  basis: z.string(), state: z.string(), reason: z.string(), model: modelSchema,
  evidence: z.record(z.string(), z.object({
    id: z.string(), run_epoch: z.string(), status: z.string(),
    expires_ms: z.number(), observed_ms: z.number(), sequence: z.number(),
  })),
  eligibility: z.record(z.string(), z.record(z.string(), z.object({ allowed: z.boolean(), reason: z.string() }))),
  assignments: z.record(z.string(), z.string()), candidate_id: z.string(), valid: z.boolean(),
  failure: z.string().nullable(),
});
export type Decision = z.infer<typeof decisionSchema>;
export const decisionsSchema = z.object({
  run_epoch: z.string(), revision: z.number(), streams_attached: z.boolean(),
  candidate_current: z.boolean(), approval: z.enum(["awaiting_approval", "unavailable"]),
  policy: decisionSchema.nullable(), allocation: decisionSchema.nullable(), resilience: decisionSchema.nullable(),
  recovery: z.record(z.string(), rowSchema),
  commands: z.array(z.object({
    id: z.string(), run_epoch: z.string(), candidate_id: z.string(), basis: z.string(),
    input_revision: z.number(), policy_revision: z.number(), decision: z.unknown(),
    status: z.enum(["committed", "gateway_applied", "recovery_confirmed", "rejected"]),
    committed_ms: z.number(), applied_ms: z.number().nullable(), applied_revision: z.number().nullable(),
    support_endpoint: z.string().nullable(), recovered_ms: z.number().nullable(), error: z.string().nullable(),
  })),
  audit: z.array(rowSchema),
  publications: z.array(z.object({
    id: z.string(), target: z.string(), attempts: z.number(), last_error: z.string().nullable(),
    next_attempt_ms: z.number(),
  })),
  observation_window_ms: z.number(), success_freshness_ms: z.number(),
});
export type Decisions = z.infer<typeof decisionsSchema>;
export type Selection = Pick<Decision, "candidate_id" | "basis" | "run_epoch" | "revision"> & {
  command_id: string; assignments?: Record<string, string>; selected_ms?: number;
};
export const selectionSchema = z.object({
  candidate_id: z.string(), basis: z.string(), run_epoch: z.string(), revision: z.number(), command_id: z.string(),
  assignments: z.record(z.string(), z.string()).optional(), selected_ms: z.number().optional(),
});

export function readDecision(context: Context | null, name: string): { data: Decision | null; error: string | null } {
  const row = context?.queries[name]?.active;
  if (!row) return { data: null, error: `${name}: no maintained result` };
  try {
    const data = decisionSchema.parse(typeof row.payload === "string" ? JSON.parse(row.payload) : row.payload);
    if (row.run_epoch !== data.run_epoch || row.revision !== data.revision || row.basis !== data.basis) {
      throw new Error("Result envelope and payload disagree");
    }
    return { data, error: null };
  } catch (error) {
    return { data: null, error: `${name}: invalid result: ${errorText(error)}` };
  }
}

export function isCurrent(decision: Decision | null, health: Health | null, now: number): boolean {
  return !!decision && !!health && health.continuous_context === "ready" &&
    decision.run_epoch === health.run_epoch && decision.revision === health.revision &&
    decision.state !== "unknown" &&
    decision.model.endpoints.every(endpoint => {
      const evidence = decision.evidence[endpoint.id];
      return !!evidence && evidence.run_epoch === health.run_epoch &&
        evidence.expires_ms > now && evidence.observed_ms <= now &&
        ["ok", "failed", "throttled"].includes(evidence.status);
    });
}

export function rows(context: Context | null, query: string): Row[] {
  return Object.values(context?.queries[query] ?? {});
}
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
