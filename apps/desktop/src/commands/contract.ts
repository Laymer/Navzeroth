// Typed application command boundary (plan section 13). Mirrors the Rust
// DTOs in crates/app; wire names are snake_case, exactly as serialized.
import type { CharacterSnapshotEnvelope } from "../../../../schemas/v1/exchange";

/** Where displayed data comes from. `demo` data is synthetic and always labelled. */
export type DataMode = "demo" | "stored";

export type AppErrorCode =
  | "INVALID_INPUT"
  | "UNSUPPORTED_SCHEMA"
  | "BUILD_MISMATCH"
  | "SNAPSHOT_OUTDATED"
  | "MISSING_REQUIRED_OBSERVATION"
  | "DATASET_INCOMPLETE"
  | "PROVIDER_UNAVAILABLE"
  | "IMPORT_TOO_LARGE"
  | "OPERATION_CANCELED"
  | "STORAGE_ERROR"
  | "INTERNAL"; // client-only: any failure that is not a well-formed backend error

export interface AppError {
  code: AppErrorCode;
  /** Safe to display; never contains secrets or raw file contents. */
  message: string;
  retryable: boolean;
}

export interface AppStatus {
  version: string;
  data_mode: DataMode;
  active_profile: string | null;
  /** Capability states stay explicit; nothing is verified in this milestone. */
  capabilities: Record<string, "unverified" | "unsupported">;
}

export interface ProfileSummary {
  id: string;
  display_name: string;
  data_mode: DataMode;
}

/** Every command the UI may call. Each maps to one narrowly scoped Rust command. */
export interface CommandMap {
  get_app_status: { input: undefined; output: AppStatus };
  list_profiles: { input: undefined; output: ProfileSummary[] };
  get_character_snapshot: {
    input: { profile_id: string };
    output: CharacterSnapshotEnvelope;
  };
}

export type CommandName = keyof CommandMap;

/** Transport seam: the Tauri adapter in the app, the demo or a mock in tests. */
export interface CommandAdapter {
  invoke<C extends CommandName>(
    command: C,
    input: CommandMap[C]["input"],
  ): Promise<CommandMap[C]["output"]>;
}

export type Result<T> = { ok: true; value: T } | { ok: false; error: AppError };
