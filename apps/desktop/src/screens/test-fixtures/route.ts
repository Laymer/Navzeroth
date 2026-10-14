import type { RouteInspection } from "../../commands/routes";
import type { PlanningReport } from "../../commands/contract";
/** Synthetic projections only; no game geometry or verified calibration claim. */
export function syntheticRoute(count = 103): RouteInspection {
  const context = { build: { edition: "wow_forever" as const, build: "UNVERIFIED_DEMO", interface: "UNVERIFIED_DEMO" }, namespace: "demo" as const };
  const map = { context, map_id: "synthetic-map", floor: "ground", instance: null };
  const report: PlanningReport = { context, fixture: true, request: { profile_id: "p", snapshot_id: "s", revision: "r", goal_profile: "manual-utility-v1", origin: "n0", stopping_node: "tail", budget_ms: 100000, stopping_service_ms: 0, candidates: [], excluded: [] }, assumptions: [], filtered: [], result: { selected: [], stopping: { total_ms: 0 }, expected_total_ms: 0, estimated: false, termination: "trace_limit", alternatives: [], trace: [{ decision: "incomplete_synthetic_trace" }] } };
  const frame = { id: "synthetic-frame", map, width_px: 1000, height_px: 1000 };
  return { context, fixture: true, report, frames: [frame, { ...frame, id: "synthetic-other-floor", map: { ...map, floor: "upper" } }], hops: Array.from({ length: count }, (_, i) => ({
    leg: i, stopping: i === count - 1, hop: { edge: `e${i}`, from: { id: `n${i}`, map }, to: { id: `n${i + 1}`, map }, mode: "walking", status: "estimated", cost_ms: i === 0 ? 0 : 10, expected_ms: { status: "known", value: 0 }, wait_ms: { status: "unknown" }, service_ms: 0, range: { status: "unsupported" }, stateful: false, consumes_resource: { status: "known", value: false }, cooldown_ms: { status: "restricted" } },
    requirements: { status: "not_applicable" }, from: { outcome: "placed", image: frame.id, pixel: { x: 0, y: 100 }, notices: [], calibration_evidence: [], calibration_confidence: null }, to: { outcome: "placed", image: frame.id, pixel: { x: 100, y: 200 }, notices: [], calibration_evidence: [], calibration_confidence: null }, schematic_connection: true, geometry_gap: null,
  })) };
}

export function required<T>(value: T | null | undefined): T { if (value == null) throw new Error("Missing synthetic fixture entry"); return value; }
