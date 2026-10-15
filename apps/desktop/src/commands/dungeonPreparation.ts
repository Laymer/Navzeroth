import type { DataContext, ItemKey } from "../../../../schemas/v1/exchange";
import type { Freshness, PartyQuestReport } from "./contract";
export type Availability = "known" | "unknown" | "unsupported" | "restricted" | "not_applicable";
export type Observed<T> = { status: "known"; value: T } | { status: Exclude<Availability, "known">; value?: never };
export interface Declared<T> { observation: Observed<T>; evidence: string[] }
export interface ConditionAssessment { availability: Availability; evaluation: { truth: "true" | "false" | "unknown"; reasons: unknown[]; warnings: unknown[]; steps: number } | null }
export interface LocalPreparationPackage {
  schema_version: 1; id: string; title: string; context: DataContext; fixture: boolean; revision: string;
  entrance: Declared<{ id: string; map: unknown }>; access: Declared<unknown>; quests: Declared<string[]>;
  rewards: Declared<{ item: ItemKey; source: string; declared_role: Observed<string> }[]>;
  checks: Declared<{ id: string; label: string; category: "role" | "consumables" | "equipment" | "before_departure" | "before_entry"; requirement: Declared<unknown> }[]>;
  formation_ms: Declared<{ min_ms: number; max_ms: number }>; run_ms: Declared<{ min_ms: number; max_ms: number }>;
  before_departure: Declared<string[]>; before_entry: Declared<string[]>; encounter_notes: Declared<string[]>;
}
export interface PreparationReport {
  authored?: import("./authoredContent").AuthoredSelection | null;
  schema_version: 1; context: DataContext; fixture: boolean; revision: string | null; package: LocalPreparationPackage | null;
  members: { profile_id: string; character: string; snapshot_id: string | null; freshness: Freshness | null;
    access: ConditionAssessment | null;
    quests: { quest: string; assessment: PartyQuestReport["participants"][number]["assessment"] }[];
    checks: { check: string; condition: ConditionAssessment }[];
    rewards: { item: ItemKey; source: string; item_requirement: ConditionAssessment; acquisition_requirement: ConditionAssessment; probability: Observed<number>; cost: Observed<string> }[];
  }[];
  evidence: { id: string; source: string; confidence: string; license: Observed<string>; redistribution: Observed<string>; offline_usable: Observed<boolean> }[];
  gaps: { code: "package_missing" | "local_package_unverified" | "active_content_missing" | "section_unavailable" | "evidence_missing" | "snapshot_missing" | "snapshot_not_fresh" | "origin_missing" | "entrance_unavailable" | "route_needs_fresh_members" | "no_supported_route" | "consumable_inventory_unavailable"; section: string; availability: Availability | null }[];
  route: { outcome: { status: "complete"; route: { total_ms: number; hops: { edge: string; mode: string; cost_ms: number }[] } } | { status: string; route?: never } } | null;
  route_exclusions: { edge: string; character: string | null; condition: ConditionAssessment | null; reason: string }[];
}
