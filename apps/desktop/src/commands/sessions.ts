/** Separate local recorder transport. Exchange schemas remain v1. */
import type { DataContext, ItemKey, Observation_Level } from "../../../../schemas/v1/exchange";
export interface SessionTotals { observed_ms: number; estimated_ms: number; unclassified_ms: number; gap_ms: number }
export interface SessionSummary {
  session: string; character: string; context: DataContext; elapsed_ms: number;
  starting_level: Observation_Level; ending_level: Observation_Level;
  observed_quests: string[]; changed_equipment: ItemKey[];
  intervals: { start_ms: number; end_ms: number; quality: "observed" | "estimated" | "unclassified"; label: string }[];
  gaps: { start_ms: number; end_ms: number; reason: string }[]; methodology: string;
}
export type SessionClockFlag = { code: "utc_went_backward"; at_ms: number; utc_delta_ms: number } | { code: "utc_disagrees"; at_ms: number; monotonic_delta_ms: number; utc_delta_ms: number };
export interface SessionAnalytics {
  summary: SessionSummary;
  progression: { observed_level_change: number | null; unique_observed_quest_count: number; equipment_change_item_count: number };
  timing: { covered_ms: number; totals: SessionTotals; uncovered_ms: number; categories: Record<string, SessionTotals>; gap_reasons_ms: Record<string, number> };
  clock_flags: SessionClockFlag[];
  utc: { status: "mapped"; start_utc_ms: number } | { status: "uncertain"; reason: {code: "missing_or_untrusted_time" | "clock_changed" | "timeline_discontinuity"} };
}
export interface SessionEvent {
  session: string; sequence: number; elapsed_ms: number; utc?: number | null; clock_trust?: "trusted" | "uncertain";
  kind: {type:"session_start" | "recording_paused" | "recording_resumed" | "logout"} | {type:"level"; level:Observation_Level} | {type:"activity";label:string;quality:"observed" | "estimated" | "unclassified"} | {type:"quest_turn_in";quest:string} | {type:"equipment_changed";item:ItemKey};
}
export interface SessionBatch { character: string; context: DataContext; events: SessionEvent[] }
export interface SessionDocument { format: "anav-session-events"; version: 1; fixture: boolean; batch: SessionBatch }
export interface SessionImportReceipt { profile_id: string; inserted: number; duplicates: number; session_ids: string[] }
export interface SessionListEntry { session_id: string; analytics: SessionAnalytics }
export interface SessionDetail { fixture: boolean; batch: SessionBatch; analytics: SessionAnalytics }
