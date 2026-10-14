import type { AppError, Result } from "./contract";

export type PlanningKind = "solo" | "joint";
export const PLANNING_JOB_SCHEMA_VERSION = 1;
export interface PlanningJobStart { job_schema_version: 1; job_id: string }
export interface PlanningJobStatus {
  job_schema_version: 1;
  job_id: string;
  kind: PlanningKind;
  state: "queued" | "running" | "succeeded" | "failed" | "canceled";
  cancel_requested: boolean;
  phase: "validate_inputs" | "participant_readiness" | "edge_assessment" | "route_expansion" | "candidate_assessment" | "selection" | null;
  work_units: number;
  result_available: boolean;
  error: AppError | null;
}
export interface PlanningJobResult { job_schema_version: 1; job_id: string; kind: PlanningKind; report_json: string }
export interface PlanningJobClient {
  startPlanningJob(kind: PlanningKind, text: string): Promise<Result<PlanningJobStart>>;
  getPlanningJob(jobId: string): Promise<Result<PlanningJobStatus>>;
  cancelPlanningJob(jobId: string): Promise<Result<null>>;
  takePlanningJobResult(jobId: string): Promise<Result<PlanningJobResult>>;
}
const canceled = (): Result<never> => ({ ok: false, error: { code: "OPERATION_CANCELED", message: "Planning request canceled.", retryable: false } });
const unavailable = (message: string): Result<never> => ({ ok: false, error: { code: "PROVIDER_UNAVAILABLE", message, retryable: false } });
function pause(signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal.aborted) { resolve(); return; }
    const stop = () => { clearTimeout(timer); signal.removeEventListener("abort", stop); resolve(); };
    const timer = setTimeout(stop, 250);
    signal.addEventListener("abort", stop, { once: true });
  });
}
/** Poll only the visible pending job. Abort handles cancellation before registration
 * finishes; bounded polling and backend deadlines keep failures inspectable. */
export async function runPlanningJob<T>(client: PlanningJobClient, kind: PlanningKind, text: string, signal: AbortSignal,
  onProgress: (status: PlanningJobStatus) => void, wait: (signal: AbortSignal) => Promise<void> = pause, onCompleted?: (jobId:string)=>void): Promise<Result<T>> {
  if (signal.aborted) return canceled();
  const started = await client.startPlanningJob(kind, text);
  if (!started.ok) return started;
  if (typeof started.value?.job_id !== "string" || started.value.job_id.length === 0 || started.value.job_id.length > 128) return unavailable("Planning job registration identity is invalid.");
  if (started.value.job_schema_version !== PLANNING_JOB_SCHEMA_VERSION) { void client.cancelPlanningJob(started.value.job_id); return unavailable("Planning job registration version is unsupported."); }
  const id = started.value.job_id;
  const cancel = () => { void client.cancelPlanningJob(id); };
  signal.addEventListener("abort", cancel, { once: true });
  try {
    if (signal.aborted) { await client.cancelPlanningJob(id); return canceled(); }
    for (let iteration = 0; iteration < 120; iteration += 1) {
      if (signal.aborted) return canceled();
      const response = await client.getPlanningJob(id);
      if (signal.aborted) return canceled();
      if (!response.ok) { cancel(); return response; }
      const status = response.value;
      if (status?.job_schema_version !== PLANNING_JOB_SCHEMA_VERSION || status.job_id !== id || status.kind !== kind || !Number.isSafeInteger(status.work_units) || status.work_units < 0) { cancel(); return unavailable("Planning job version, identity or progress is invalid."); }
      if (typeof status.cancel_requested !== "boolean" || typeof status.result_available !== "boolean" || !["queued", "running", "succeeded", "failed", "canceled"].includes(status.state) || ![null, "validate_inputs", "participant_readiness", "edge_assessment", "route_expansion", "candidate_assessment", "selection"].includes(status.phase)) { cancel(); return unavailable("Planning job status is unsupported."); }
      onProgress(status);
      if (status.state === "canceled") return status.error ? { ok: false, error: status.error } : canceled();
      if (status.state === "failed") {
        return { ok: false, error: status.error ?? { code: "PROVIDER_UNAVAILABLE", message: "Planning did not produce a valid result.", retryable: false } };
      }
      if (status.state === "succeeded") {
        if (!status.result_available) return unavailable("Planning result expired or was already retrieved.");
        const result = await client.takePlanningJobResult(id);
        if (signal.aborted) return canceled();
        if (!result.ok) return result;
        if (result.value?.job_schema_version !== PLANNING_JOB_SCHEMA_VERSION || result.value.job_id !== id || result.value.kind !== kind || typeof result.value.report_json !== "string" || result.value.report_json.length > 4 * 1024 * 1024 || new TextEncoder().encode(result.value.report_json).byteLength > 4 * 1024 * 1024) return unavailable("Planning result version, identity or byte size is invalid.");
        try { const parsed: unknown = JSON.parse(result.value.report_json); if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return unavailable("Planning result is not a report object."); onCompleted?.(id); return { ok: true, value: parsed as T }; }
        catch { return unavailable("Planning result could not be read."); }
      }
      await wait(signal);
    }
    cancel();
    return unavailable("Planning status polling limit reached.");
  } finally { signal.removeEventListener("abort", cancel); }
}
