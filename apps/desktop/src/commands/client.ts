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
    listLocalSubmissions: (after: string | null = null, limit = 32) => run(() => adapter.invoke("list_local_submissions", { after, limit })),
    submitLocalObservation: (text: string, opt_in: boolean) => run(() => adapter.invoke("submit_local_observation", { text, opt_in })),
    changeLocalSubmission: (expected: import("./localSubmissions").SubmissionGuard, action: import("./localSubmissions").SubmissionAction) => run(() => adapter.invoke("change_local_submission", { text: JSON.stringify({ expected, action }) })),
    exportLocalSubmission: (expected: import("./localSubmissions").SubmissionGuard) => run(() => adapter.invoke("export_local_submission", { text: JSON.stringify(expected) })),
    getAppSettings:()=>run(()=>adapter.invoke("get_app_settings",undefined)),
    updateAppSettings:(text:string)=>run(()=>adapter.invoke("update_app_settings",{text})),
    getTodayView:()=>run(()=>adapter.invoke("get_today_view",undefined)),
    pinCurrentPlan:(jobId:string,expectedRevision:number)=>run(()=>adapter.invoke("pin_current_plan",{job_id:jobId,expected_revision:expectedRevision})),
    clearCurrentPlan:(expectedRevision:number)=>run(()=>adapter.invoke("clear_current_plan",{expected_revision:expectedRevision})),
    
    getContentDispositions: (profileId:string) => run(() => adapter.invoke("get_content_dispositions",{profile_id:profileId})),
    changeContentDisposition: (text:string) => run(() => adapter.invoke("change_content_disposition",{text})),
    getAuthoredCatalog: (text: string) => run(() => adapter.invoke("get_authored_catalog", { text })),
    getDungeonPreparation: (text: string) => run(() => adapter.invoke("get_dungeon_preparation", { text })),
    getReferenceView: () => run(() => adapter.invoke("get_reference_view",undefined)),
    getReferenceWindowState: () => run(() => adapter.invoke("get_reference_window_state",undefined)),
    toggleReferenceWindow: (alwaysOnTop:boolean,interactionMode:import("./reference").InteractionMode) => run(() => adapter.invoke("toggle_reference_window",{always_on_top:alwaysOnTop,interaction_mode:interactionMode})),
    setReferenceInteraction: (interactionMode:import("./reference").InteractionMode) => run(() => adapter.invoke("set_reference_interaction",{interaction_mode:interactionMode})),
    setReferenceWindow: (visible:boolean,alwaysOnTop:boolean) => run(() => adapter.invoke("set_reference_window",{visible,always_on_top:alwaysOnTop})),
    previewAcquisitionRoute: (text: string) => run(() => adapter.invoke("preview_acquisition_route", { text })),
    assessRouteAcquisition: (text: string) => run(() => adapter.invoke("assess_route_acquisition", { text })),
    previewDiagnostics: () => run(() => adapter.invoke("preview_diagnostics", undefined)),
    exportDiagnostics: (path: string) => run(() => adapter.invoke("export_diagnostics", { path })),
    startPlanningJob: (kind: import("./planningJobs").PlanningKind, text: string) => run(() => adapter.invoke("start_planning_job", { kind, text })),
    getPlanningJob: (jobId: string) => run(() => adapter.invoke("get_planning_job", { job_id: jobId })),
    cancelPlanningJob: (jobId: string) => run(() => adapter.invoke("cancel_planning_job", { job_id: jobId })),
    takePlanningJobResult: (jobId: string) => run(() => adapter.invoke("take_planning_job_result", { job_id: jobId })),
    getDatabaseStatus: () => run(() => adapter.invoke("get_database_status", undefined)),
    checkDatabase: () => run(() => adapter.invoke("check_database", undefined)),
    createDatabaseBackup: (path: string) => run(() => adapter.invoke("create_database_backup", { path })),
    inspectDatabaseBackup: (path: string) => run(() => adapter.invoke("inspect_database_backup", { path })),
    restoreDatabaseBackup: (candidateId: string) => run(() => adapter.invoke("restore_database_backup", { candidate_id: candidateId })),

    getPartyPlanProposal: (partyId: string) => run(() => adapter.invoke("get_party_plan_proposal", { party_id: partyId })),
    proposePartyPlan: (text: string) => run(() => adapter.invoke("propose_party_plan", { text })),
    acceptPartyPlan: (text: string) => run(() => adapter.invoke("accept_party_plan", { text })),
    getJointPlanningCatalog: (partyId: string) => run(() => adapter.invoke("get_joint_planning_catalog", { party_id: partyId })),
    planJointSession: (text: string) => run(() => adapter.invoke("plan_joint_session", { text })),
    setSessionCalibration: (profileId: string, sessionId: string, annotation: import("./calibration").SessionCalibrationAnnotation) => run(() => adapter.invoke("set_session_calibration", { profile_id: profileId, session_id: sessionId, annotation })),
    getCalibration: (profileId: string, request: import("./calibration").CalibrationRequest) => run(() => adapter.invoke("get_calibration", { profile_id: profileId, request })),
    reminderView: (profileId: string) => run(() => adapter.invoke("reminder_view", { profile_id: profileId })),
    mutateReminders: (request: import("./reminders").ReminderRequest) => run(() => adapter.invoke("mutate_reminders", { text: JSON.stringify(request) })),
    importSessionText: (profileId: string, text: string) => run(() => adapter.invoke("import_session_text", { profile_id: profileId, text })),
    importSessionFile: (profileId: string, path: string, format: FileInputFormat) => run(() => adapter.invoke("import_session_file", { profile_id: profileId, path, format })),
    listSessions: (profileId: string) => run(() => adapter.invoke("list_sessions", { profile_id: profileId })),
    getSessionDetail: (profileId: string, sessionId: string) => run(() => adapter.invoke("get_session_detail", { profile_id: profileId, session_id: sessionId })),
    deleteSession: (profileId: string, sessionId: string) => run(() => adapter.invoke("delete_session", { profile_id: profileId, session_id: sessionId })),
    getGearCatalog: (profileId: string) => run(() => adapter.invoke("get_gear_catalog", { profile_id: profileId })),
    compareGear: (text: string) => run(() => adapter.invoke("compare_gear", { text })),
    getHistoricalSnapshot: (profileId: string, snapshotId: string) => run(() => adapter.invoke("get_historical_snapshot", { profile_id: profileId, snapshot_id: snapshotId })),
    promoteSnapshot: (profileId: string, snapshotId: string, expectedCurrent: string, reason: string) => run(() => adapter.invoke("promote_snapshot", { profile_id: profileId, snapshot_id: snapshotId, expected_current: expectedCurrent, reason })),
    getPlanningCatalog: (profileId: string) => run(() => adapter.invoke("get_planning_catalog", { profile_id: profileId })),
    exportPlan: (text: string) => run(() => adapter.invoke("export_plan", { text })),
    exportAgreedPartyPlan: (text: string) => run(() => adapter.invoke("export_agreed_party_plan", { text })),
    inspectPlanRoute: (text: string) => run(() => adapter.invoke("inspect_plan_route", { text })),
    planSession: (text: string) => run(() => adapter.invoke("plan_session", { text })),
    getCompatibilityReport: (profile_id: string | null) => run(() => adapter.invoke("get_compatibility_report", { profile_id })),
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
    queryAtlas: (image: string | null = null, page?: import("./contract").AtlasToken, offset?: number): Promise<Result<AtlasView>> =>
      run(() => adapter.invoke("query_atlas", { image, ...(page ? {page,offset:offset??0}: {}) })),
    projectPositions: (positions: Position[], page?: import("./contract").AtlasToken): Promise<Result<ProjectedPosition[]>> =>
      run(() => adapter.invoke("project_positions", { positions, ...(page ? {page}: {}) })),
    getSnapshotMarkers: (page?: import("./contract").AtlasToken): Promise<Result<SnapshotMarker[]>> =>
      run(() => adapter.invoke("get_snapshot_markers", page?{page}:undefined)),
  };
}

export type CommandClient = ReturnType<typeof createClient>;
