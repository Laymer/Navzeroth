import { describe, expect, it } from "vitest";
import { runPlanningJob, type PlanningJobClient, type PlanningJobStatus } from "./planningJobs";
import type { Result } from "./contract";
const status: PlanningJobStatus = { job_id: "job-a", kind: "solo", state: "running", cancel_requested: false, phase: "route_expansion", work_units: 5, result_available: false, error: null };
const ok = <T,>(value: T): Result<T> => ({ ok: true, value });
function fake(patch: Partial<PlanningJobClient> = {}) {
  const calls: string[] = [];
  const client: PlanningJobClient = {
    startPlanningJob: async () => { calls.push("start"); return ok({ job_id: "job-a" }); },
    getPlanningJob: async () => { calls.push("status"); return ok({ ...status, state: "succeeded", result_available: true }); },
    cancelPlanningJob: async () => { calls.push("cancel"); return ok(null); },
    takePlanningJobResult: async () => { calls.push("take"); return ok({ job_id: "job-a", kind: "solo", report_json: '{"selected":[0]}' }); },
    ...patch,
  };
  return { client, calls };
}
describe("visible planning jobs", () => {
  it("retrieves a terminal result once with matching job identity", async () => {
    const { client, calls } = fake(); const progress: PlanningJobStatus[] = [];
    expect(await runPlanningJob(client, "solo", "request", new AbortController().signal, (p) => progress.push(p))).toEqual(ok({ selected: [0] }));
    expect(calls).toEqual(["start", "status", "take"]); expect(progress[0]?.work_units).toBe(5);
  });
  it("cancels a job whose registration response arrives after navigation abort", async () => {
    const controller = new AbortController(); let ready: ((v: Result<{ job_id: string }>) => void) | undefined;
    const { client, calls } = fake({ startPlanningJob: () => new Promise((resolve) => { ready = resolve; }) });
    const result = runPlanningJob(client, "solo", "request", controller.signal, () => undefined);
    controller.abort(); ready?.(ok({ job_id: "job-a" }));
    expect(await result).toMatchObject({ ok: false, error: { code: "OPERATION_CANCELED" } }); expect(calls).toEqual(["cancel"]);
  });
  it("aborts running work without retrieving or displaying a late result", async () => {
    const controller = new AbortController(); const { client, calls } = fake({ getPlanningJob: async () => ok(status) });
    const result = await runPlanningJob(client, "solo", "request", controller.signal, () => undefined, async () => { controller.abort(); });
    expect(result).toMatchObject({ ok: false, error: { code: "OPERATION_CANCELED" } }); expect(calls).toEqual(["start", "cancel"]);
  });
  it("bounds polling and asks the backend to cancel on expiry", async () => {
    let polls = 0; const { client, calls } = fake({ getPlanningJob: async () => { polls++; return ok(status); } });
    const result = await runPlanningJob(client, "solo", "request", new AbortController().signal, () => undefined, async () => undefined);
    expect(result).toMatchObject({ ok: false, error: { code: "PROVIDER_UNAVAILABLE" } }); expect(polls).toBe(120); expect(calls.at(-1)).toBe("cancel"); expect(calls).not.toContain("take");
  });
  it("refuses unrelated job results and preserves backend unknown/failure", async () => {
    const { client } = fake({ getPlanningJob: async () => ok({ ...status, job_id: "other" }) });
    expect(await runPlanningJob(client, "solo", "request", new AbortController().signal, () => undefined)).toMatchObject({ ok: false, error: { code: "PROVIDER_UNAVAILABLE" } });
    const failed = fake({ getPlanningJob: async () => ok({ ...status, state: "failed", error: { code: "SNAPSHOT_OUTDATED", message: "Snapshot changed", retryable: false } }) });
    expect(await runPlanningJob(failed.client, "solo", "request", new AbortController().signal, () => undefined)).toMatchObject({ ok: false, error: { code: "SNAPSHOT_OUTDATED" } });
  });
});
