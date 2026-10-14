import type { DataContext, Objective, TravelNode } from "../../../../schemas/v1/exchange";
import type { Freshness, PlanningReport } from "../commands/contract";
export interface JointMember {
  profile_id: string; character: string; snapshot_id: string | null; freshness: Freshness | null;
}
export interface JointPlanningCatalog {
  context: DataContext; fixture: boolean; revision: string | null;
  participants: JointMember[]; nodes: TravelNode[]; objectives: Objective[];
}
export interface JointEstimate {
  objective: string; node: string; utilities: Record<string, number>;
  completion_ms: number; service_ms: number;
}
export interface JointPlanningRequest {
  party_id: string; participants: { profile_id: string; snapshot_id: string }[];
  revision: string; goal_profile: string; origin: string; stopping_node: string;
  budget_ms: number; stopping_service_ms: number; candidates: JointEstimate[]; excluded: string[];
}
export interface JointPlanningReport {
  request: JointPlanningRequest; context: DataContext; fixture: boolean; participants: JointMember[];
  filtered: { objective: string; reason: string }[]; assumptions: string[];
  result: {
    plan: PlanningReport["result"];
    assessments: { objective: string; ready: boolean; participants: { character: string; utility: number; readiness: { truth: "true" | "false" | "unknown"; reasons: unknown[] } }[] }[];
    unavailable_edges: { edge: string; character: string; truth: "true" | "false" | "unknown" }[];
  };
}
