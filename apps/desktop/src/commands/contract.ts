import type { CompatibilityReport } from "./compatibility";
import type { DatabaseStatus, BackupPreview, RestoreReceipt } from "./recovery";
import type { PartyProposalReport } from "../party/proposals";
import type { JointPlanningCatalog, JointPlanningReport } from "../party/contract";
import type { CalibrationRequest, CalibrationReport, SessionCalibrationAnnotation } from "./calibration";
import type { ReminderView } from "./reminders";
// Typed application command boundary (plan section 13). Mirrors the Rust
// DTOs in crates/app; wire names are snake_case, exactly as serialized.
import type {
  BuildKey,
  CharacterSnapshotEnvelope,
  Confidence,
  DatasetNamespace,
  DataContext,
  ImageLayerKind,
  ImagePin,
  MapKey,
  PixelPoint,
  Position,
  TravelNode,
  Objective,
} from "../../../../schemas/v1/exchange";

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
  context?: DataContext;
  fixture?: boolean;
  snapshot_count?: number;
}

export type FileInputFormat = { kind: "anav1_text" } | { kind: "saved_variables"; global: string; key: string };
export type ImportDisposition =
  | { outcome: "stored"; previous_current: string | null }
  | { outcome: "duplicate" }
  | { outcome: "history_only"; reason: "older" | "same_position" | "incomparable" };
export interface ImportReceipt {
  profile_id: string;
  snapshot_id: string;
  disposition: ImportDisposition;
  fixture: boolean;
  namespace: DatasetNamespace;
  freshness: Freshness;
  data_mode: DataMode;
}
export interface StoredSnapshot {
  envelope: CharacterSnapshotEnvelope;
  profile_id: string;
  freshness: Freshness;
  data_mode: DataMode;
}
export interface StoredParty {
  id: string;
  data_mode: DataMode;
  participants: {
    profile_id: string;
    character: string;
    snapshot: StoredSnapshot | null;
  }[];
}
export interface PartyQuestCatalog {
  pages?: import("./catalogPages").CatalogPages;
  context: DataContext;
  fixture: boolean;
  revision: string | null;
  quests: string[];
}
type Policy<T> = { status: "known"; value: T } | { status: "unknown" | "unsupported" | "restricted" | "not_applicable" };
export interface ContentInspection {
  pages?: {content_sha256:string;sources:number;conflicts:number;gaps:number;errors:number};
  key: { scope: { context: DataContext; fixture: boolean }; revision: string };
  report: {
    passes: boolean;
    report_version: number;
    coverage: {
      collections: Record<string, { total: number; fields: Record<string, Record<ObservationStatus, number>> }>;
      evidence_records: { by_confidence: Record<string, number> };
    } | null;
    errors: { code: string }[];
    gaps: { code: string }[];
  };
  sources: { id: string; publisher: string; license: Policy<string>; attribution: Policy<string>; redistribution: Policy<string>; offline_usable: Policy<boolean> }[];
  conflicts: unknown[];
}
export type CatchUp =
  | { kind: "single"; requirement: { kind: "quest"; quest: string; state: "accepted" | "completed" | "turned_in" } | { kind: "level"; level: number } | { kind: "other" } }
  | { kind: "all_of"; items: CatchUp[] }
  | { kind: "one_of"; alternatives: CatchUp[] };
export interface PartyQuestReport {
  context: DataContext;
  fixture: boolean;
  revision: string;
  quest: string;
  joint_status: "jointly_eligible" | "eligible_for_some" | "all_behind" | "blocked" | "unknown" | "missing_or_old_snapshots";
  participants: {
    character: string;
    snapshot_id: string | null;
    freshness: Freshness | null;
    assessment: {
      acceptance: "eligible" | "behind" | "blocked" | "unknown";
      progress: "not_started" | "in_progress" | "all_stages_done" | "unknown";
      completion: "complete" | "incomplete" | "unknown";
      turn_in: "ready" | "not_ready" | "already_turned_in" | "unknown";
      catch_up: CatchUp | null;
      reasons: { phase: "acceptance" | "progress" | "completion" | "turn_in"; message: string }[];
    } | null;
  }[];
}
export interface HistoryEntry {
  snapshot_id: string;
  is_current: boolean;
  freshness: Freshness;
  observed_at_utc_ms: number | null;
  received_at_utc_ms: number | null;
}
export type WatchStatus = "waiting_for_stable_file" | "imported" | "missing" | "unreadable" | "invalid_file" | "retry_pending" | "retry_exhausted" | "cancelled";
export interface WatchState {
  active: boolean;
  status: WatchStatus | null;
  last_import: ImportReceipt | null;
  error: AppError | null;
}

// --- Atlas DTOs ------------------------------------------------------------
// Hand-written mirrors of crates/app/src/atlas.rs (these are not part of the
// generated exchange schema). fixtures/desktop/atlas/*.json are written by a
// Rust test and parsed against these types in atlas/golden.test.ts, so a
// drifting field fails a test instead of silently breaking the screen.

export type ObservationStatus = "known" | "unknown" | "unsupported" | "restricted" | "not_applicable";

export type LayerAvailability = "available" | "not_embedded" | "integrity_mismatch" | "unservable_id";

export interface AtlasLayer {
  id: string;
  kind: ImageLayerKind;
  /** Present only when the layer is available; images are fetched through the atlas scheme. */
  url: string | null;
  availability: LayerAvailability;
  sha256: string;
  width_px: number;
  height_px: number;
  frame_divisor: number;
}

/** Image pins are reference geography; `reference_only` is always true. */
export type AtlasPin = ImagePin & { reference_only: true };

export interface AtlasImage {
  id: string;
  map: MapKey;
  width_px: number;
  height_px: number;
  layers: AtlasLayer[];
  pins: AtlasPin[];
  calibration_status: ObservationStatus;
  calibration_confidence: Confidence | null;
  /** True unless calibration is verified or sampled from enough control points. */
  approximate: boolean;
  gaps: string;
  attribution: string;
}

export interface AtlasToken { profile_id: string; revision: string; catalog_sha256: string }
export interface AtlasPage { token: AtlasToken; offset: number; total: number; next_offset: number | null }
export interface AtlasView {
  page?: AtlasPage;
  data_mode: DataMode;
  fixture: boolean;
  revision: string;
  namespace: DatasetNamespace;
  images: AtlasImage[];
}

export type ProjectionNotice =
  | { notice: "cross_build_reuse"; image_build: BuildKey }
  | { notice: "embedding_fallback"; via: MapKey[] };

export interface Projection {
  image: string;
  /** Full-frame pixels. */
  pixel: PixelPoint;
  notices: ProjectionNotice[];
  calibration_evidence: string[];
  calibration_confidence: Confidence | null;
}

export type ProjectionGap =
  | { gap: "namespace_mismatch" }
  | { gap: "no_image_for_map"; map: MapKey }
  | { gap: "in_instance"; map: MapKey }
  | { gap: "different_floor"; map: MapKey; available_floors: string[] }
  | { gap: "build_mismatch"; position_build: BuildKey; image: string | null; applicable_builds: BuildKey[] }
  | { gap: "embedding_chain_too_deep"; via: MapKey[] }
  | { gap: "uncalibrated_image"; image: string; status: ObservationStatus }
  | { gap: "out_of_bounds"; image: string; pixel: PixelPoint };

export type ProjectedPosition = ({ outcome: "placed" } & Projection) | ({ outcome: "gap" } & ProjectionGap);

export type Freshness =
  | { status: "fresh"; age_ms: number }
  | { status: "stale"; age_ms: number }
  | { status: "uncertain"; reason: "missing_or_untrusted_time" | "clock_reversed" | "age_out_of_range" };

export interface SnapshotMarker {
  profile_id: string;
  snapshot_id: string;
  location_status: ObservationStatus;
  observed_at_utc_ms: number | null;
  received_at_utc_ms: number | null;
  freshness: Freshness;
  /** Null unless the location is known. */
  projection: ProjectedPosition | null;
  /** Null unless the marker is placed. */
  approximate: boolean | null;
  fixture: boolean;
  data_mode: DataMode;
}

export interface PlanningCatalog {
  pages?: import("./catalogPages").CatalogPages;
  snapshot_id: string | null; revision: string | null; fixture: boolean;
  nodes: TravelNode[]; objectives: Objective[];
}
export interface PlanningRequest {
  profile_id: string; snapshot_id: string; revision: string; goal_profile: string;
  origin: string; stopping_node: string; budget_ms: number; stopping_service_ms: number;
  candidates: { objective: string; node: string; utility: number; completion_ms: number; service_ms: number }[];
  excluded: string[];
}
export interface PlanningReport {
  clustering?: import("./clustering").ClusteringMetadata;
  request: PlanningRequest; context: DataContext; fixture: boolean; assumptions: string[];
  filtered: { objective: string; reason: string }[];
  result: {
    selected: { candidate: string; utility: number; incremental_ms: number; completion_ms: number; service_ms: number; travel: { total_ms: number }; expected_session_ms: number }[];
    stopping: { total_ms: number }; expected_total_ms: number; estimated: boolean;
    alternatives: unknown[]; trace: unknown[]; termination: string;
  };
}

/** Every command the UI may call. Each maps to one narrowly scoped Rust command. */
export interface CommandMap {
  prepare_research_delivery:{input:{catalog:string;locale:string;bindings?:import("./localData").LocalDeliveryChoices};output:string};
  get_research_delivery_page:{input:{revision:string;domain:string;offset:number;limit:number};output:import("./localData").ResearchDeliveryPage};
  get_research_delivery_activation:{input:{revision:string};output:import("./localData").ResearchDeliveryActivation};
  activate_research_delivery:{input:{revision:string;expected:number;rollback:boolean};output:import("./localData").ResearchDeliveryActivation};
  diff_research_delivery:{input:{before:string;after:string};output:import("./localData").ResearchDeliveryDiff};
  list_local_installations:{input:undefined;output:import("./localData").SavedLocalInstallation[]};
  restore_local_installation:{input:{selection_id:string};output:import("./localData").LocalInstallation};
  list_local_data_jobs:{input:undefined;output:import("./localData").LocalDataJob[]};
  forget_local_data_job:{input:{job_id:string};output:null};
  resume_local_data:{input:{job_id:string};output:import("./localData").LocalDataJob};
  get_local_data_activation:{input:{revision:string};output:import("./localData").LocalDataActivation};
  activate_local_data:{input:{revision:string;expected:number;rollback:boolean};output:import("./localData").LocalDataActivation};
  select_local_installation:{input:undefined;output:import("./localData").LocalInstallation|null};
  preview_local_data:{input:{selection_id:string;preset:import("./localData").LocalDataPreset};output:import("./localData").LocalDataPlan};
  get_research_delivery_manifest:{input:{revision:string};output:import("./localData").LocalDeliveryManifest};
  build_local_inventory:{input:{delivery:string;authored:string|null;preset:import("./localData").LocalDataPreset};output:string};
  get_local_inventory_summary:{input:{revision:string;authored:string|null};output:import("./localData").LocalInventorySummary};
  get_local_inventory_page:{input:{revision:string;domain:string;offset:number;limit:number;authored:string|null};output:import("./localData").LocalInventoryPage};
  review_selected_local_guide:{input:{request:import("./localData").LocalGuideRequest};output:import("./localData").LocalGuidePage};
  get_local_tooltip:{input:{projection:string;spell:number};output:import("./localData").LocalTooltip};
  get_local_artifact_page:{input:{namespace:string;offset:number;limit:number};output:import("./localData").LocalArtifactPage};
  discover_local_media:{input:{catalog:string;locale:string;offset:number;limit:number};output:import("./localData").LocalMediaDiscovery};
  extract_local_media:{input:{selection_id:string;catalog:string;locale:string;ids:number[];preset:import("./localData").LocalDataPreset};output:string};
  get_local_media_page:{input:{revision:string;offset:number;limit:number};output:import("./localData").LocalMediaPage};
  get_local_media_manifest:{input:{revision:string};output:import("./localData").LocalMediaManifest};
  select_local_material:{input:{kind:import("./localData").LocalMaterialKind};output:import("./localData").LocalMaterial|null};
  list_local_materials:{input:undefined;output:import("./localData").LocalMaterial[]};
  forget_local_material:{input:{id:string};output:null};
  stage_selected_local_input:{input:{catalog:string;id:string;locale:string;preset:import("./localData").LocalDataPreset};output:import("./localData").LocalStagedInput};
  get_local_input_page:{input:{revision:string;offset:number;limit:number};output:import("./localData").LocalInputPage};
  get_local_workflow:{input:undefined;output:import("./localData").LocalWorkflowRecord};
  finish_local_startup:{input:{expected:number;mode:import("./localData").LocalStartupMode;completed:boolean};output:import("./localData").LocalWorkflowRecord};
  save_local_workflow_choices:{input:{selection_id:string;expected:number;locales:string[];preset:import("./localData").LocalDataPreset;detect:boolean};output:import("./localData").LocalWorkflowRecord};
  get_local_change_detectors:{input:undefined;output:import("./localData").LocalChangeDetector[]};
  acknowledge_local_build_change:{input:{selection_id:string;expected:number;candidate:string};output:import("./localData").LocalWorkflowRecord};
  start_local_data:{input:{selection_id:string;preset:import("./localData").LocalDataPreset;locales?:string[]};output:import("./localData").LocalDataJob};
  get_local_data_job:{input:{job_id:string};output:import("./localData").LocalDataJob};
  cancel_local_data_job:{input:{job_id:string};output:import("./localData").LocalDataJob};
  get_local_data_inventory:{input:{revision:string;offset:number;limit:number};output:import("./localData").LocalDataInventoryPage};
  toggle_reference_window: {input:{always_on_top:boolean;interaction_mode:import("./reference").InteractionMode};output:import("./reference").ReferenceControlReceipt};
  set_reference_interaction: {input:{interaction_mode:import("./reference").InteractionMode};output:import("./reference").ReferenceControlReceipt};
  list_local_submissions: { input: { after: string | null; limit: number }; output: import("./localSubmissions").SubmissionPage };
  submit_local_observation: { input: { text: string; opt_in: boolean }; output: import("./localSubmissions").SubmitReceipt };
  change_local_submission: { input: { text: string }; output: import("./localSubmissions").SubmissionRecord };
  export_local_submission: { input: { text: string }; output: import("./localSubmissions").LocalSubmissionExport };
  get_app_settings:{input:undefined;output:import("./today").SettingsRecord};
  update_app_settings:{input:{text:string};output:import("./today").SettingsRecord};
  get_today_view:{input:undefined;output:import("./today").TodayView};
  pin_current_plan:{input:{job_id:string;expected_revision:number};output:number};
  clear_current_plan:{input:{expected_revision:number};output:number};
  get_content_dispositions: { input: {profile_id:string}; output: import("./contentDispositions").ContentGovernance };
  change_content_disposition: { input: {text:string}; output: import("./contentDispositions").ContentDecision };
  get_catalog_page: { input: { text: string }; output: import("./catalogPages").CatalogRecordPage };
  get_authored_catalog: { input: { text: string }; output: import("./authoredContent").AuthoredCatalog };
  get_dungeon_preparation: { input: { text: string }; output: import("./dungeonPreparation").PreparationReport };
  get_reference_view: {input:undefined;output:import("./reference").ReferenceViewData};
  get_reference_window_state: {input:undefined;output:import("./reference").ReferenceWindowState};
  set_reference_window: {input:{visible:boolean;always_on_top:boolean};output:import("./reference").ReferenceWindowState};
  preview_acquisition_route: { input: { text: string }; output: import("./routeAcquisition").RouteAcquisitionBaseline };
  assess_route_acquisition: { input: { text: string }; output: import("./routeAcquisition").RouteAcquisitionReport };
  preview_diagnostics: { input: undefined; output: import("./diagnostics").DiagnosticReport };
  export_diagnostics: { input: { path: string }; output: import("./diagnostics").DiagnosticReceipt };
  start_planning_job: { input: { kind: import("./planningJobs").PlanningKind; text: string }; output: { job_schema_version: 1; job_id: string } };
  get_planning_job: { input: { job_id: string }; output: import("./planningJobs").PlanningJobStatus & { job_schema_version: 1 } };
  cancel_planning_job: { input: { job_id: string }; output: null };
  take_planning_job_result: { input: { job_id: string }; output: import("./planningJobs").PlanningJobResult & { job_schema_version: 1 } };
  get_database_status: { input: undefined; output: DatabaseStatus };
  check_database: { input: undefined; output: null };
  create_database_backup: { input: { path: string }; output: null };
  inspect_database_backup: { input: { path: string }; output: BackupPreview };
  restore_database_backup: { input: { candidate_id: string }; output: RestoreReceipt };
  get_party_plan_proposal: { input: { party_id: string }; output: PartyProposalReport | null };
  propose_party_plan: { input: { text: string }; output: PartyProposalReport };
  accept_party_plan: { input: { text: string }; output: PartyProposalReport };
  get_joint_planning_catalog: { input: { party_id: string }; output: JointPlanningCatalog };
  plan_joint_session: { input: { text: string }; output: JointPlanningReport };
  set_session_calibration: { input: { profile_id: string; session_id: string; annotation: SessionCalibrationAnnotation }; output: null };
  get_calibration: { input: { profile_id: string; request: CalibrationRequest }; output: CalibrationReport };
  reminder_view: { input: { profile_id: string }; output: ReminderView };
  mutate_reminders: { input: { text: string }; output: ReminderView };

  inspect_plan_route: { input: { text: string }; output: import("./routes").RouteInspection };
  import_session_text: { input: { profile_id: string; text: string }; output: import("./sessions").SessionImportReceipt };
  import_session_file: { input: { profile_id: string; path: string; format: FileInputFormat }; output: import("./sessions").SessionImportReceipt };
  list_sessions: { input: { profile_id: string }; output: import("./sessions").SessionListEntry[] };
  get_session_detail: { input: { profile_id: string; session_id: string }; output: import("./sessions").SessionDetail };
  delete_session: { input: { profile_id: string; session_id: string }; output: boolean };
  get_gear_catalog: { input: { profile_id: string }; output: import("./gear").GearCatalog };
  compare_gear: { input: { text: string }; output: import("./gear").GearReport };
  export_plan: { input: { text: string }; output: { text: string; byte_count: number; plan_id: string; expires_at_utc_ms: number } };
  export_agreed_party_plan: { input: { text: string }; output: { text: string; byte_count: number; plan_id: string; expires_at_utc_ms: number } };
  get_planning_catalog: { input: { profile_id: string }; output: PlanningCatalog };
  plan_session: { input: { text: string }; output: PlanningReport };
  get_historical_snapshot: { input: { profile_id: string; snapshot_id: string }; output: StoredSnapshot | null };
  promote_snapshot: { input: { profile_id: string; snapshot_id: string; expected_current: string; reason: string }; output: null };

  get_compatibility_report: { input: { profile_id: string | null }; output: CompatibilityReport };
  get_app_status: { input: undefined; output: AppStatus };
  list_profiles: { input: undefined; output: ProfileSummary[] };
  stage_verified_content_bundle: {input:{path:string};output:ContentInspection};
  get_content_inspection_page: {input:{text:string};output:ContentInspectionPage};
  stage_content: { input: { text: string }; output: ContentInspection };
  get_active_content: { input: { profile_id: string }; output: ContentInspection | null };
  activate_content: { input: { profile_id: string; revision: string }; output: ContentInspection };
  rollback_content: { input: { profile_id: string }; output: { changed: boolean; active: ContentInspection | null } };
  list_parties: { input: undefined; output: string[] };
  get_party_quests: { input: { party_id: string }; output: PartyQuestCatalog };
  get_party_eligibility: { input: { party_id: string; revision: string; quest_id: string }; output: PartyQuestReport };
  get_party: { input: { party_id: string }; output: StoredParty | null };
  save_party: { input: { party_id: string; profile_ids: string[] }; output: null };
  delete_party: { input: { party_id: string }; output: boolean };
  get_character_snapshot: {
    input: { profile_id: string };
    output: CharacterSnapshotEnvelope;
  };
  import_snapshot_text: { input: { text: string }; output: ImportReceipt };
  import_snapshot_file: { input: { path: string; format: FileInputFormat }; output: ImportReceipt };
  start_snapshot_watch: { input: { path: string; format: FileInputFormat }; output: WatchState };
  stop_snapshot_watch: { input: undefined; output: WatchState };
  get_snapshot_watch: { input: undefined; output: WatchState };
  select_profile: { input: { profile_id: string }; output: null };
  get_stored_snapshot: { input: { profile_id: string }; output: StoredSnapshot | null };
  get_snapshot_history: { input: { profile_id: string; limit: number }; output: HistoryEntry[] };
  query_atlas: { input: { image: string | null; page?: AtlasToken; offset?: number }; output: AtlasView };
  /** One to 64 positions. */
  project_positions: { input: { positions: Position[]; page?: AtlasToken }; output: ProjectedPosition[] };
  get_snapshot_markers: { input: {page?: AtlasToken} | undefined; output: SnapshotMarker[] };
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

export interface ContentInspectionPage { key:ContentInspection["key"];content_sha256:string;collection:"sources"|"conflicts"|"gaps"|"errors";offset:number;total:number;next_offset:number|null;records:unknown[] }
