// Application DTOs for explicit gear assumptions; exchange schemas remain v1.
import type { AcquisitionSource, ConfigurationItem, DataContext, ItemDefinition, ItemKey, ObservationMetadata, Observation_EquipmentSlot } from "../../../../schemas/v1/exchange";
export type GearObservation<T> = ({ status: "known"; value: T } | { status: "unknown" | "unsupported" | "restricted" | "not_applicable" }) & ObservationMetadata;
export interface GearCatalog {
  snapshot_id: string | null;
  revision: string | null;
  fixture: boolean;
  context: DataContext;
  equipment: Record<string, Observation_EquipmentSlot>;
  items: ItemDefinition[];
  acquisitions: AcquisitionSource[];
}
export interface GearEffortInput {
  incremental_travel_ms: GearObservation<number>;
  formation_ms: GearObservation<number>;
  attempt_ms: GearObservation<number>;
  stopping_ms: GearObservation<number>;
  available_copper: GearObservation<string>;
  session_budget_ms: number;
  max_attempts: number;
  independent_player_attempts: boolean;
  cost_per_attempt: boolean;
}
export interface GearInput {
  item: ItemKey;
  category: GearObservation<string>;
  acquisition: { source: string; effort: GearEffortInput } | null;
}
export type WeightSource = { type: "user_preference" | "synthetic_test" } | { type: "documented_model" | "permitted_guide"; source: string };
export interface GearRequest {
  profile_id: string;
  snapshot_id: string;
  revision: string;
  model: { role: string; version: "gear-model-v1"; weight_source: WeightSource; weights: Record<string,number>; known_unmodeled: string[] };
  layout: { slots: string[]; hands: { main_hand: string; off_hand: string } | null };
  class: GearObservation<string>;
  proficiency: { classes: Record<string,{allowed: string[]; disallowed: string[]}> };
  candidates: GearInput[];
}
export type GearReason = { code: string; [field: string]: unknown };
export interface GearScore {
  label: string;
  model_version: string;
  role: string;
  weight_source: WeightSource;
  contributions: { attribute: string; value: number; weight: number; contribution: number }[];
  total: number;
  unmodeled_effects: GearReason[];
  profile_unmodeled: string[];
  certainty: "complete" | "reduced" | "low";
  certainty_reasons: GearReason[];
}
export type GearEligibility = { outcome: "eligible" } | { outcome: "future_progression"; required_level: number } | { outcome: "ineligible" | "unknown"; reasons: GearReason[] };
export interface GearConfiguration { rank: number; configuration: { validity: "valid" | "uncertain"; items: Record<string,ConfigurationItem>; uncertainties: GearReason[] }; score: GearScore }
export interface GearRanking {
  ranking: "theoretical" | "obtainable" | "practical";
  label: string;
  scope_note: string;
  model_version: string;
  role: string;
  valid: GearConfiguration[];
  uncertain: GearConfiguration[];
  rejected: { items: Record<string,ItemKey>; reasons: GearReason[] }[];
  excluded: { item: ItemKey; eligibility: GearEligibility }[];
  top_is_definitive: boolean;
}
export interface GearAcquisition {
  source: AcquisitionSource;
  effort: Omit<GearEffortInput, "independent_player_attempts" | "cost_per_attempt"> & { balance_basis: string; probability_model: string; cost_schedule: string };
  target: ItemKey;
  status: "guaranteed" | "uncertain" | "unavailable";
  reasons: GearReason[];
  requirements: { truth: "true" | "false" | "unknown" } | null;
  feasible_attempts: number | null;
  budgeted_effort_ms: number | null;
  budgeted_cost_copper: string | null;
  success_probability: number | null;
  expected_attempts: number | null;
  credited_reward: ItemKey | null;
}
export interface GearReport {
  request: GearRequest;
  fixture: boolean;
  context: DataContext;
  current: { slots: Record<string,{state: "empty"} | {state: "matched"; item: ItemKey} | {state: "unresolved"; gap: GearReason}> };
  current_score: GearScore;
  baseline_gaps: string[];
  theoretical: GearRanking;
  equip_eligible: GearRanking;
  obtainable: GearRanking;
  candidates: { item: ItemKey; eligibility: GearEligibility; score: GearScore; acquisition: GearAcquisition | null; practical_reasons: string[] }[];
  practical: { item: ItemKey; slot: string; proposed: GearConfiguration; gain_milli: number; probability_millionths: number; conditional_expected_gain_milli: number; reserved_effort_ms: number; conditional: boolean }[];
  practical_model_version: string;
  assumptions: string[];
}
