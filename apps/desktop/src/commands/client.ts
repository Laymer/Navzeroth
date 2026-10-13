import type {
  AppError,
  AppStatus,
  FileInputFormat,
  HistoryEntry,
  ImportReceipt,
  StoredSnapshot,
  StoredParty,
  WatchState,
  AtlasView,
  CommandAdapter,
  ProfileSummary,
  ProjectedPosition,
  Result,
  SnapshotMarker,
} from "./contract";
import type { CharacterSnapshotEnvelope, Position } from "../../../../schemas/v1/exchange";

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
    getGearCatalog: (profileId: string) => run(() => adapter.invoke("get_gear_catalog", { profile_id: profileId })),
    compareGear: (text: string) => run(() => adapter.invoke("compare_gear", { text })),
    getHistoricalSnapshot: (profileId: string, snapshotId: string) => run(() => adapter.invoke("get_historical_snapshot", { profile_id: profileId, snapshot_id: snapshotId })),
    promoteSnapshot: (profileId: string, snapshotId: string, expectedCurrent: string, reason: string) => run(() => adapter.invoke("promote_snapshot", { profile_id: profileId, snapshot_id: snapshotId, expected_current: expectedCurrent, reason })),
    getPlanningCatalog: (profileId: string) => run(() => adapter.invoke("get_planning_catalog", { profile_id: profileId })),
    exportPlan: (text: string) => run(() => adapter.invoke("export_plan", { text })),
    planSession: (text: string) => run(() => adapter.invoke("plan_session", { text })),
    getAppStatus: (): Promise<Result<AppStatus>> =>
      run(() => adapter.invoke("get_app_status", undefined)),
    listProfiles: (): Promise<Result<ProfileSummary[]>> =>
      run(() => adapter.invoke("list_profiles", undefined)),
    stageContent: (text: string) => run(() => adapter.invoke("stage_content", { text })),
    getActiveContent: (profileId: string) => run(() => adapter.invoke("get_active_content", { profile_id: profileId })),
    activateContent: (profileId: string, revision: string) => run(() => adapter.invoke("activate_content", { profile_id: profileId, revision })),
    rollbackContent: (profileId: string) => run(() => adapter.invoke("rollback_content", { profile_id: profileId })),
    listParties: (): Promise<Result<string[]>> => run(() => adapter.invoke("list_parties", undefined)),
    getPartyQuests: (partyId: string) => run(() => adapter.invoke("get_party_quests", { party_id: partyId })),
    getPartyEligibility: (partyId: string, revision: string, questId: string) => run(() => adapter.invoke("get_party_eligibility", { party_id: partyId, revision, quest_id: questId })),
    getParty: (partyId: string): Promise<Result<StoredParty | null>> => run(() => adapter.invoke("get_party", { party_id: partyId })),
    saveParty: (partyId: string, profileIds: string[]): Promise<Result<null>> => run(() => adapter.invoke("save_party", { party_id: partyId, profile_ids: profileIds })),
    deleteParty: (partyId: string): Promise<Result<boolean>> => run(() => adapter.invoke("delete_party", { party_id: partyId })),
    getCharacterSnapshot: (profileId: string): Promise<Result<CharacterSnapshotEnvelope>> =>
      run(() => adapter.invoke("get_character_snapshot", { profile_id: profileId })),
    importText: (text: string): Promise<Result<ImportReceipt>> =>
      run(() => adapter.invoke("import_snapshot_text", { text })),
    importFile: (path: string, format: FileInputFormat): Promise<Result<ImportReceipt>> =>
      run(() => adapter.invoke("import_snapshot_file", { path, format })),
    startWatch: (path: string, format: FileInputFormat): Promise<Result<WatchState>> =>
      run(() => adapter.invoke("start_snapshot_watch", { path, format })),
    stopWatch: (): Promise<Result<WatchState>> =>
      run(() => adapter.invoke("stop_snapshot_watch", undefined)),
    getWatch: (): Promise<Result<WatchState>> =>
      run(() => adapter.invoke("get_snapshot_watch", undefined)),
    selectProfile: (profileId: string): Promise<Result<null>> =>
      run(() => adapter.invoke("select_profile", { profile_id: profileId })),
    getStoredSnapshot: (profileId: string): Promise<Result<StoredSnapshot | null>> =>
      run(() => adapter.invoke("get_stored_snapshot", { profile_id: profileId })),
    getSnapshotHistory: (profileId: string, limit = 32): Promise<Result<HistoryEntry[]>> =>
      run(() => adapter.invoke("get_snapshot_history", { profile_id: profileId, limit })),
    queryAtlas: (image: string | null = null): Promise<Result<AtlasView>> =>
      run(() => adapter.invoke("query_atlas", { image })),
    projectPositions: (positions: Position[]): Promise<Result<ProjectedPosition[]>> =>
      run(() => adapter.invoke("project_positions", { positions })),
    getSnapshotMarkers: (): Promise<Result<SnapshotMarker[]>> =>
      run(() => adapter.invoke("get_snapshot_markers", undefined)),
  };
}

export type CommandClient = ReturnType<typeof createClient>;
