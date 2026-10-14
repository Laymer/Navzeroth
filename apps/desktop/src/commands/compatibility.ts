import type { DataContext } from "../../../../schemas/v1/exchange";
export interface StatusCounts { known: number; unknown: number; unsupported: number; restricted: number; not_applicable: number }
export type EvidenceState = "reported_flow" | "function_present_only" | "reported_missing" | "unverified" | "assumed_reference";
export interface CompatibilityReport {
  report_version: number; app_version: string;
  selected: { context: DataContext; fixture: boolean; snapshot_present: boolean } | null;
  recorded_builds: { build: DataContext["build"]; version: string; basis: EvidenceState; reported_on: string; addon_version: string | null; matches_selected_build: boolean | null; limitations: string; capabilities: { capability: string; state: EvidenceState; statement: string; source: string }[] }[];
  active: {
    context: DataContext; fixture: boolean; revision: string; structurally_valid: boolean; finite_scope: string;
    counts: { collections: Record<string, { total: number; evidence: { records_with_evidence: number; records_without_evidence: number; links_by_confidence: Record<string, number> } | null; fields: Record<string, StatusCounts> }>; evidence_records: { by_confidence: Record<string, number>; records_with_conflicts: number; conflict_links: number }; edge_status: Record<string, number>; quest_graph: Record<string, number>; atlas: { positions_checked: number; projected: number; gaps: Record<string, number> } };
    errors_by_code: Record<string, number>; gaps_by_code: Record<string, number>;
    sources: { registered: number; referenced_distinct: number; unregistered_distinct: number; evidence_references: number; image_references: number; policy_statuses: Record<string, StatusCounts>; redistribution_values: { permitted: number; not_permitted: number; conditional: number }; offline_values: { known_true: number; known_false: number }; editions_include_selected: number; editions_exclude_selected: number; verified_revision_matches: number; verified_revision_differs: number; declared_conflicts: number };
  } | null;
  unverified_checks: { check: string; required_evidence: string }[];
  official_api: string; limitations: string;
}
