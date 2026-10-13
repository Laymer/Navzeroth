import type { DataContext, MapKey, Predicate, DurationRange } from "../../../../schemas/v1/exchange";
import type { PlanningReport, ProjectedPosition } from "./contract";
type Observation<T> = { status: "known"; value: T } | { status: "unknown" | "unsupported" | "restricted" | "not_applicable" };
export interface RouteInspection {
  context: DataContext; fixture: boolean; report: PlanningReport;
  frames: { id: string; map: MapKey; width_px: number; height_px: number }[];
  hops: {
    leg: number; stopping: boolean;
    hop: { edge: string; from: { id: string; map: MapKey }; to: { id: string; map: MapKey }; mode: string; status: string; cost_ms: number; expected_ms: Observation<number>; wait_ms: Observation<number>; service_ms: number; range: Observation<DurationRange>; stateful: boolean; consumes_resource: Observation<boolean>; cooldown_ms: Observation<number> };
    requirements: Observation<Predicate>; from: ProjectedPosition; to: ProjectedPosition;
    schematic_connection: boolean; geometry_gap: string | null;
  }[];
}
