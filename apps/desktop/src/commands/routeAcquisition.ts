import type { Result, PlanningRequest, PlanningReport, PlanningCatalog } from "./contract";
import type { NodeRef, ItemKey } from "../../../../schemas/v1/exchange";
import type { GearAcquisition, GearObservation, GearReport, GearRequest, GearReason } from "./gear";
export interface RouteOpportunityInput {
  source: string; target: ItemKey; node: NodeRef | null; leg_index: number;
  formation_ms: GearObservation<number>; attempt_ms: GearObservation<number>;
  available_copper: GearObservation<string>; max_attempts: number;
  independent_player_attempts: boolean; cost_per_attempt: boolean;
}
export interface RouteAcquisitionBaseline {
  schema_version: 1; route_fingerprint: string; planning_report: PlanningReport;
  legs: {index: number; stopping: boolean; route: {start: NodeRef; hops: {to: NodeRef}[]; total_ms: number}}[];
}
export interface RouteAcquisitionRequest {
  schema_version: 1; planning: PlanningRequest; expected_route_fingerprint: string;
  opportunities: RouteOpportunityInput[]; gear: GearRequest | null;
}
/** Existing Rust RouteSearch wire facts; no geometry or live position is inferred. */
export interface AcquisitionRouteTrace {
  outcome: { status: "complete"; route: AcquisitionRoutePath } | { status: "incomplete"; target: string; reached: AcquisitionRoutePath; reachable: string[] };
  exclusions: { edge: string; reason: GearReason }[];
}
export interface AcquisitionRoutePath {
  start: NodeRef; total_ms: number;
  hops: { edge: string; from: NodeRef; to: NodeRef; mode: string; status: string; cost_ms: number; stateful: boolean; consumes_resource: GearObservation<boolean>; cooldown_ms: GearObservation<number> }[];
}
export interface RouteAcquisitionReport {
  schema_version: 1; baseline: RouteAcquisitionBaseline; remaining_budget_ms: number;
  opportunities: {input: RouteOpportunityInput; detour: {status:"complete"; incremental_ms:number; baseline_leg_ms:number; outbound:AcquisitionRouteTrace; rejoin:AcquisitionRouteTrace} | {status:"unknown"; reason:"missing_manual_binding"|"outbound_incomplete"|"rejoin_incomplete"|"unsafe_stateful_action";outbound:AcquisitionRouteTrace|null;rejoin:AcquisitionRouteTrace|null}; assessment: GearAcquisition; reward_accounting_complete:boolean; total_with_reservation_ms:number|null}[];
  gear: GearReport|null; assumptions:string[];
}
export interface RouteAcquisitionClient {
  getPlanningCatalog(profileId:string):Promise<Result<PlanningCatalog>>;
  previewAcquisitionRoute(text:string):Promise<Result<RouteAcquisitionBaseline>>;
  assessRouteAcquisition(text:string):Promise<Result<RouteAcquisitionReport>>;
}
