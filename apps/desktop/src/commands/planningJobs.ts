import type { AppError, Result } from "./contract";

export type PlanningKind = "solo" | "joint";
export interface PlanningJobStatus {
  job_id: string;
  kind: PlanningKind;
  state: "queued" | "running" | "succeeded" | "failed" | "canceled";
  cancel_requested: boolean;
  phase: "validate_inputs" | "participant_readiness" | "edge_assessment" | "route_expansion" | "candidate_assessment" | "selection" | null;
  work_units: number;
  result_available: boolean;
  error: AppError | null;
}
export interface PlanningJobResult { job_id: string; kind: PlanningKind; report_json: string }
export interface PlanningJobClient {
  startPlanningJob(kind: PlanningKind, text: string): Promise<Result<{ job_id: string }>>;
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
  onProgress: (status: PlanningJobStatus) => void, wait: (signal: AbortSignal) => Promise<void> = pause): Promise<Result<T>> {
  if (signal.aborted) return canceled();
  const started = await client.startPlanningJob(kind, text);
  if (!started.ok) return started;
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
      if (status.job_id !== id || status.kind !== kind) { cancel(); return unavailable("Planning job identity changed."); }
      onProgress(status);
      if (status.state === "failed" || status.state === "canceled") {
        return { ok: false, error: status.error ?? { code: "PROVIDER_UNAVAILABLE", message: "Planning did not produce a valid result.", retryable: false } };
      }
      if (status.state === "succeeded") {
        if (!status.result_available) return unavailable("Planning result expired or was already retrieved.");
        const result = await client.takePlanningJobResult(id);
        if (signal.aborted) return canceled();
        if (!result.ok) return result;
        if (result.value.job_id !== id || result.value.kind !== kind || result.value.report_json.length > 4 * 1024 * 1024) return unavailable("Planning result identity or size is invalid.");
        try { return { ok: true, value: JSON.parse(result.value.report_json) as T }; }
        catch { return unavailable("Planning result could not be read."); }
      }
      await wait(signal);
    }
    cancel();
    return unavailable("Planning status polling limit reached.");
  } finally { signal.removeEventListener("abort", cancel); }
}
