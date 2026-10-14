import { describe, expect, it } from "vitest";
import { runPlanningJob, type PlanningJobClient, type PlanningJobStatus } from "./planningJobs";
import type { Result } from "./contract";
const status: PlanningJobStatus = { job_schema_version: 1, job_id: "job-a", kind: "solo", state: "running", cancel_requested: false, phase: "route_expansion", work_units: 5, result_available: false, error: null };
const ok = <T,>(value: T): Result<T> => ({ ok: true, value });
function fake(patch: Partial<PlanningJobClient> = {}) {
  const calls: string[] = [];
  const client: PlanningJobClient = {
    startPlanningJob: async () => { calls.push("start"); return ok({ job_schema_version: 1, job_id: "job-a" }); },
    getPlanningJob: async () => { calls.push("status"); return ok({ ...status, state: "succeeded", result_available: true }); },
    cancelPlanningJob: async () => { calls.push("cancel"); return ok(null); },
    takePlanningJobResult: async () => { calls.push("take"); return ok({ job_schema_version: 1, job_id: "job-a", kind: "solo", report_json: '{"selected":[0]}' }); },
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
    const controller = new AbortController(); let ready: ((v: Result<{ job_schema_version: 1; job_id: string }>) => void) | undefined;
    const { client, calls } = fake({ startPlanningJob: () => new Promise((resolve) => { ready = resolve; }) });
    const result = runPlanningJob(client, "solo", "request", controller.signal, () => undefined);
    controller.abort(); ready?.(ok({ job_schema_version: 1, job_id: "job-a" }));
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
it("rejects unsupported registration/status/result versions and invalid work counters", async () => {
  const patches: Partial<PlanningJobClient>[] = [
    { startPlanningJob: async () => ok({ job_schema_version: 2, job_id: "job-a" } as never) },
    { getPlanningJob: async () => ok({ ...status, job_schema_version: 2 } as never) },
    { getPlanningJob: async () => ok({ ...status, work_units: -1 }) },
    { getPlanningJob: async () => ok({ ...status, result_available: "unknown" } as never) },
    { getPlanningJob: async () => ok({ ...status, state: "succeeded", result_available: true }) , takePlanningJobResult: async () => ok({ job_schema_version: 2, job_id: "job-a", kind: "solo", report_json: "{}" } as never) },
  ];
  for (const patch of patches) {
    expect(await runPlanningJob(fake(patch).client, "solo", "request", new AbortController().signal, () => undefined)).toMatchObject({ ok: false, error: { code: "PROVIDER_UNAVAILABLE" } });
  }
});
it("reports queued and running checkpoints without invented completion percentages", async () => {
  let poll = 0; const progress: PlanningJobStatus[] = [];
  const { client } = fake({ getPlanningJob: async () => ok(poll++ === 0 ? { ...status, state: "queued", phase: null, work_units: 0 } : poll === 2 ? status : { ...status, state: "succeeded", result_available: true }) });
  expect(await runPlanningJob(client, "solo", "request", new AbortController().signal, (value) => progress.push(value), async () => undefined)).toEqual(ok({ selected: [0] }));
  expect(progress.map((value) => [value.state, value.work_units])).toEqual([["queued", 0], ["running", 5], ["succeeded", 5]]);
});
it("ignores a status response arriving after abort and maps terminal cancellation honestly", async () => {
  const controller = new AbortController(); let finish: ((value: Result<PlanningJobStatus>) => void) | undefined;
  const { client } = fake({ getPlanningJob: () => new Promise((resolve) => { finish = resolve; }) });const progress: PlanningJobStatus[] = [];
  const pending = runPlanningJob(client, "solo", "request", controller.signal, (value) => progress.push(value));await Promise.resolve();controller.abort();finish?.(ok({ ...status, state: "succeeded", result_available: true }));
  expect(await pending).toMatchObject({ ok: false, error: { code: "OPERATION_CANCELED" } });expect(progress).toEqual([]);
  const terminal = fake({ getPlanningJob: async () => ok({ ...status, state: "canceled" }) });
  expect(await runPlanningJob(terminal.client, "solo", "request", new AbortController().signal, () => undefined)).toMatchObject({ ok: false, error: { code: "OPERATION_CANCELED" } });
});
it("bounds UTF8 report bytes and rejects non-object JSON", async () => {
  for (const report_json of [JSON.stringify({ text: "é".repeat(2_100_000) }), "null", "[]"]) {
    const { client } = fake({ takePlanningJobResult: async () => ok({ job_schema_version: 1, job_id: "job-a", kind: "solo", report_json }) });
    expect(await runPlanningJob(client, "solo", "request", new AbortController().signal, () => undefined)).toMatchObject({ ok: false, error: { code: "PROVIDER_UNAVAILABLE" } });
  }
});
