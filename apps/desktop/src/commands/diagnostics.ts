import type { Result } from "./contract";
export interface DiagnosticCapability { availability: "known" | "unknown" | "unsupported" | "restricted" | "not_applicable"; check: "verified_available" | "verified_unavailable" | "restricted" | "failed" | "untested" | null; verified_available: boolean | null; freshness: { status: "fresh" | "stale"; age_ms: number } | { status: "uncertain"; reason: string } | null }
export interface DiagnosticReport {
  format: "azeroth-local-diagnostics"; schema_version: 1;
  versions: { app: string; bundled_addon: string; installed_addon: null };
  os: "windows" | "linux" | "macos" | "other";
  game: { edition: "forever" | "other"; build: number | null; interface: number | null; fixture: boolean } | null;
  capabilities: Record<string, DiagnosticCapability>;
  historical_user_report: { build: 70338; interface: 16001; source: "limited_user_report"; fields: ["level", "clock"]; installed_revision_match: null };
  snapshot_availability: Record<string, string>;
  snapshot_freshness: DiagnosticCapability["freshness"];
  dataset: { revision_fingerprint_sha256: string; coverage_class: "Demo" | "ReferenceFixture" | "Reference"; collections: Record<string, unknown>; errors: number; gaps: number } | null;
  dataset_context_mismatch: boolean;
  telemetry: { scope: "current_process_collector"; event_limit: 64; truncated: boolean | null; import_attempts: number | null; import_succeeded: number | null; import_failed: number | null; events: { operation: string; error: string | null; duration_ms: number | null }[] };
}
export interface DiagnosticReceipt { schema_version: 1; bytes: number }
/** Root wires these only to the explicit native capture/create-new file operations. */
export interface DiagnosticsClient {
  previewDiagnostics(): Promise<Result<DiagnosticReport>>;
  exportDiagnostics(path: string): Promise<Result<DiagnosticReceipt>>;
}
