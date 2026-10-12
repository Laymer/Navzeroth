import { createClient } from "./client";
import { demoAdapter } from "./demoAdapter";
import { isTauri, tauriAdapter } from "./tauriAdapter";

export type {
  AppError,
  AppStatus,
  AtlasView,
  CommandAdapter,
  DataMode,
  ProfileSummary,
  Result,
  SnapshotMarker,
} from "./contract";
export { createClient } from "./client";
export type { CommandClient } from "./client";

/** Native shell uses the Tauri adapter; a plain browser always uses the labelled demo. */
export const defaultClient = () => createClient(isTauri() ? tauriAdapter : demoAdapter);
