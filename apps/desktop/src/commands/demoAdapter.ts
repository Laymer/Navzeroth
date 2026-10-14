// Browser-only adapter serving the synthetic fixture. Used for `vite dev`,
// unit tests and the explicit demo mode; it can never report stored data.
import type { CharacterSnapshotEnvelope } from "../../../../schemas/v1/exchange";
import fixture from "../../../../fixtures/contracts/v1/character-snapshot.json";
import atlasGolden from "../../../../fixtures/desktop/atlas/query-atlas.json";
import projectionGolden from "../../../../fixtures/desktop/atlas/project-positions.json";
import markersGolden from "../../../../fixtures/desktop/atlas/snapshot-markers-uncertain.json";
import overviewPng from "../../../../content/demo/atlas/synthetic-continent/layers/overview.png";
import roadsPng from "../../../../content/demo/atlas/synthetic-continent/layers/roads.png";
import flightPng from "../../../../content/demo/atlas/synthetic-continent/layers/flight.png";
import type {
  AppError,
  AtlasView,
  CommandAdapter,
  CommandMap,
  CommandName,
  ProjectedPosition,
  SnapshotMarker,
} from "./contract";

const snapshot = fixture as unknown as CharacterSnapshotEnvelope;
export const DEMO_PROFILE_ID = snapshot.payload.character;

/** Browser demo only: there is no atlas scheme, so layers come from bundled demo PNGs. */
const DEMO_LAYER_URLS: Record<string, string> = {
  overview: overviewPng,
  roads: roadsPng,
  flight: flightPng,
};
const demoAtlas = atlasGolden as unknown as AtlasView;
const sampleResults = projectionGolden.results as unknown as ProjectedPosition[];
/** A plain browser has no trusted clock, so the demo reports the age as uncertain. */
const demoMarkers = markersGolden as unknown as SnapshotMarker[];
const MAX_POSITIONS = 64;

function invalid(message: string): AppError {
  return { code: "INVALID_INPUT", message, retryable: false };
}

function queryAtlas(image: string | null): AtlasView {
  const images = demoAtlas.images
    .filter((candidate) => image === null || candidate.id === image)
    .map((candidate) => ({
      ...candidate,
      layers: candidate.layers.map((layer) => ({ ...layer, url: DEMO_LAYER_URLS[layer.id] ?? null })),
    }));
  if (image !== null && images.length === 0) throw invalid("No such atlas image");
  return { ...demoAtlas, images };
}

function projectPositions(positions: unknown[]): ProjectedPosition[] {
  if (positions.length < 1 || positions.length > MAX_POSITIONS) {
    throw invalid("Provide between 1 and 64 positions");
  }
  // The demo adapter has no projection engine; it answers only the golden sample list.
  if (JSON.stringify(positions) !== JSON.stringify(projectionGolden.positions)) {
    throw invalid("The demo adapter only answers its sample position list.");
  }
  return sampleResults;
}

export const demoAdapter: CommandAdapter = {
  invoke<C extends CommandName>(command: C, input: CommandMap[C]["input"]) {
    const answer = ((): CommandMap[CommandName]["output"] => {
      switch (command) {
        case "get_database_status": throw { code: "PROVIDER_UNAVAILABLE", message: "Database recovery is available in the native desktop application", retryable: false };
        case "check_database":
        case "create_database_backup":
        case "inspect_database_backup":
        case "restore_database_backup": throw { code: "PROVIDER_UNAVAILABLE", message: "Database recovery is available in the native desktop application", retryable: false };
        case "get_app_status":
          return {
            version: "0.5.0",
            data_mode: "demo",
            active_profile: DEMO_PROFILE_ID,
            capabilities: { addon_transport: "unverified", live_position: "unverified" },
          };
        case "list_profiles":
          return [{ id: DEMO_PROFILE_ID, display_name: "Demo paladin", data_mode: "demo" }];
        case "get_character_snapshot": {
          const { profile_id } = input as CommandMap["get_character_snapshot"]["input"];
          if (profile_id !== DEMO_PROFILE_ID) {
            const err: AppError = { code: "INVALID_INPUT", message: "No such profile.", retryable: false };
            throw err;
          }
          return snapshot;
        }
        case "get_snapshot_watch":
        case "stop_snapshot_watch":
          return { active: false, status: null, last_import: null, error: null };
        case "select_profile": {
          const { profile_id } = input as CommandMap["select_profile"]["input"];
          if (profile_id !== DEMO_PROFILE_ID) throw invalid("No such profile.");
          return null;
        }
        case "get_stored_snapshot": {
          const { profile_id } = input as CommandMap["get_stored_snapshot"]["input"];
          if (profile_id !== DEMO_PROFILE_ID) throw invalid("No such profile.");
          return { envelope: snapshot, profile_id, data_mode: "demo", freshness: { status: "uncertain", reason: "missing_or_untrusted_time" } };
        }
        case "get_snapshot_history":
        case "list_parties":
          return [];
        case "get_active_content":
          return null;
        case "preview_diagnostics":
        case "export_diagnostics":
        case "start_planning_job":
        case "get_planning_job":
        case "cancel_planning_job":
        case "take_planning_job_result":
        case "get_joint_planning_catalog":
        case "plan_joint_session":
        case "get_party_plan_proposal":
        case "propose_party_plan":
        case "accept_party_plan":
        case "set_session_calibration":
        case "get_calibration":
        case "reminder_view":
        case "mutate_reminders":
        case "get_gear_catalog":
        case "compare_gear":
        case "inspect_plan_route":
        case "import_session_text":
        case "import_session_file":
        case "list_sessions":
        case "get_session_detail":
        case "delete_session":
          throw { code: "PROVIDER_UNAVAILABLE", message: "This workflow requires imported data in the desktop application.", retryable: false } satisfies AppError;
        case "stage_content":
        case "activate_content":
        case "rollback_content":
          throw { code: "PROVIDER_UNAVAILABLE", message: "Content changes require the desktop application.", retryable: false } satisfies AppError;
        case "get_party":
          return null;
        case "save_party":
        case "get_party_quests":
        case "get_party_eligibility":
        case "delete_party":
          throw { code: "PROVIDER_UNAVAILABLE", message: "Local parties require the desktop application.", retryable: false } satisfies AppError;
        case "import_snapshot_text":
        case "import_snapshot_file":
        case "start_snapshot_watch":
          throw { code: "PROVIDER_UNAVAILABLE", message: "Imports and file watching require the desktop application.", retryable: false } satisfies AppError;
        case "query_atlas":
          return queryAtlas((input as CommandMap["query_atlas"]["input"]).image);
        case "project_positions":
          return projectPositions((input as CommandMap["project_positions"]["input"]).positions);
        case "get_snapshot_markers":
          return demoMarkers;
        default:
          throw { code: "INVALID_INPUT", message: "Unknown command.", retryable: false } satisfies AppError;
      }
    })();
    return Promise.resolve(answer as CommandMap[C]["output"]);
  },
};
