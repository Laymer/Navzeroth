import type { DataContext, QuestProgress } from "../../../../schemas/v1/exchange";
import type { Freshness } from "../commands/contract";
import type { JointPlanningRequest } from "./contract";
export interface ProposalGuard { proposal_id: string; revision: number }
export interface ParticipantAgreement {
  profile_id: string; expected_snapshot_id: string; current_snapshot_id: string | null;
  accepted: boolean; accepted_at_utc_ms: number | null; freshness: Freshness | null;
  reasons: string[]; progress_changes: { quest: string; previous: QuestProgress | null; current: QuestProgress | null }[];
}
export interface PartyProposalReport {
  party_id: string; schema_version: number; guard: ProposalGuard; proposer_profile_id: string;
  context: DataContext; fixture: boolean; revision: string;
  selected_objectives: string[]; expected_total_ms: number; created_at_utc_ms: number | null;
  state: "awaiting_acceptance" | "agreed" | "invalidated";
  reasons: string[]; participants: ParticipantAgreement[]; request: JointPlanningRequest;
}
