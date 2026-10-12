// The closed set of states a snapshot marker can be in. Every state has its
// own label, so no state is distinguished by colour or shape alone.
import type { Freshness, ProjectionGap, SnapshotMarker } from "../commands/contract";

type PlacedKey = `placed_${Freshness["status"]}_${"approximate" | "calibrated"}`;
type GapKey = `gap_${ProjectionGap["gap"]}`;
type UnavailableKey = "location_unknown" | "location_unsupported" | "location_restricted" | "location_not_applicable";

export type MarkerStateKey = "no_snapshot" | "projection_missing" | UnavailableKey | PlacedKey | GapKey;

/** The only place labels live; the Record type makes the union exhaustive. */
export const MARKER_STATE_LABELS: Record<MarkerStateKey, string> = {
  no_snapshot: "No imported snapshot: no position to show",
  projection_missing: "Position known but not projected",
  location_unknown: "Location unknown: not observed",
  location_unsupported: "Location unsupported by this client",
  location_restricted: "Location restricted by the game",
  location_not_applicable: "Location not applicable",
  placed_fresh_approximate: "Last imported position, recent: approximate, uncalibrated",
  placed_fresh_calibrated: "Last imported position, recent: calibrated",
  placed_stale_approximate: "Last imported position, stale: approximate, uncalibrated",
  placed_stale_calibrated: "Last imported position, stale: calibrated",
  placed_uncertain_approximate: "Last imported position, age uncertain: approximate, uncalibrated",
  placed_uncertain_calibrated: "Last imported position, age uncertain: calibrated",
  gap_namespace_mismatch: "Not drawn: different dataset namespace",
  gap_no_image_for_map: "Not drawn: no imagery for this map",
  gap_in_instance: "Not drawn: position is inside an instance",
  gap_different_floor: "Not drawn: position is on another floor",
  gap_build_mismatch: "Not drawn: imagery not declared for this build",
  gap_embedding_chain_too_deep: "Not drawn: map nesting too deep",
  gap_uncalibrated_image: "Not drawn: imagery is uncalibrated",
  gap_out_of_bounds: "Not drawn: position outside the image frame",
};

export const MARKER_STATE_KEYS = Object.keys(MARKER_STATE_LABELS) as MarkerStateKey[];

export function markerState(marker: SnapshotMarker | null): MarkerStateKey {
  if (marker === null) return "no_snapshot";
  const { projection } = marker;
  if (projection === null) {
    switch (marker.location_status) {
      case "unknown":
        return "location_unknown";
      case "unsupported":
        return "location_unsupported";
      case "restricted":
        return "location_restricted";
      case "not_applicable":
        return "location_not_applicable";
      case "known":
        return "projection_missing";
    }
  }
  if (projection.outcome === "gap") return `gap_${projection.gap}`;
  // A missing flag is treated as approximate: calibration is never assumed.
  const calibration = marker.approximate === false ? "calibrated" : "approximate";
  return `placed_${marker.freshness.status}_${calibration}`;
}

export interface Placement {
  image: string;
  pixel: { x: number; y: number };
  approximate: boolean;
}

/** The marker's drawable position, only when the backend placed it. */
export function placementOf(marker: SnapshotMarker | null): Placement | null {
  if (marker === null || marker.projection === null || marker.projection.outcome !== "placed") return null;
  return {
    image: marker.projection.image,
    pixel: { x: marker.projection.pixel.x, y: marker.projection.pixel.y },
    approximate: marker.approximate !== false,
  };
}
