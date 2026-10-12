import type { AppError, AppStatus, CommandAdapter, ProfileSummary, Result } from "./contract";
import type { CharacterSnapshotEnvelope } from "../../../../schemas/v1/exchange";

const CODES: ReadonlySet<string> = new Set([
  "INVALID_INPUT", "UNSUPPORTED_SCHEMA", "BUILD_MISMATCH", "SNAPSHOT_OUTDATED",
  "MISSING_REQUIRED_OBSERVATION", "DATASET_INCOMPLETE", "PROVIDER_UNAVAILABLE",
  "IMPORT_TOO_LARGE", "OPERATION_CANCELED", "STORAGE_ERROR", "INTERNAL",
]);

/** Normalizes anything thrown across the boundary into a safe AppError. */
export function toAppError(thrown: unknown): AppError {
  if (typeof thrown === "object" && thrown !== null) {
    const e = thrown as Record<string, unknown>;
    if (typeof e["code"] === "string" && CODES.has(e["code"]) && typeof e["message"] === "string") {
      return {
        code: e["code"] as AppError["code"],
        message: e["message"],
        retryable: e["retryable"] === true,
      };
    }
  }
  return { code: "INTERNAL", message: "The command failed unexpectedly.", retryable: false };
}

async function run<T>(call: () => Promise<T>): Promise<Result<T>> {
  try {
    return { ok: true, value: await call() };
  } catch (thrown) {
    return { ok: false, error: toAppError(thrown) };
  }
}

/** Typed command wrappers: the only way UI code reaches the backend. */
export function createClient(adapter: CommandAdapter) {
  return {
    getAppStatus: (): Promise<Result<AppStatus>> =>
      run(() => adapter.invoke("get_app_status", undefined)),
    listProfiles: (): Promise<Result<ProfileSummary[]>> =>
      run(() => adapter.invoke("list_profiles", undefined)),
    getCharacterSnapshot: (profileId: string): Promise<Result<CharacterSnapshotEnvelope>> =>
      run(() => adapter.invoke("get_character_snapshot", { profile_id: profileId })),
  };
}

export type CommandClient = ReturnType<typeof createClient>;
