import type { AppError } from "./contract";
export interface DatabaseStatus { startup_measurements?: { load_ms: number; activation_ms: number }; preserved_recovery_directory?: string | null; state: "ready" | "recovery_required" | "checking"; error: AppError | null; schema_version: number; restore_limit_bytes: number }
export interface BackupPreview { candidate_id: string; inspection: { source_schema_version: number; restored_schema_version: number; rows: Record<string, number> } }
export interface RestoreReceipt { pending_marker_cleanup_required: boolean; original_files_retained: boolean; recovery_directory: string }
