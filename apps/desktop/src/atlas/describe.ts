// Text for every atlas outcome. State is always carried by words, never by
// colour alone, and the wording never calls an imported snapshot "live".
import type { AtlasLayer, AtlasPin, Freshness, ProjectionGap, ProjectionNotice } from "../commands/contract";

export const DEMO_LABEL = "DEMO — synthetic data, not from your game";
export const APPROXIMATE_LABEL = "approximate, uncalibrated";
export const FLIGHT_PIN_LABEL = "Reference flight master. Not your unlocks";
export const ROADS_TOGGLE_LABEL = "Roads (traced, unverified)";
export const FLIGHT_TOGGLE_LABEL = "Flight network (reference, not your unlocks)";

/** Human duration such as "45 s", "31 min" or "2 h 5 min". */
export function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds} s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (hours < 24) return rest === 0 ? `${hours} h` : `${hours} h ${rest} min`;
  const days = Math.floor(hours / 24);
  return `${days} d ${hours % 24} h`;
}

/** The capture age as shown next to "Last imported". */
export function describeAge(freshness: Freshness): string {
  switch (freshness.status) {
    case "fresh":
      return `captured ${formatDuration(freshness.age_ms)} ago`;
    case "stale":
      return `captured ${formatDuration(freshness.age_ms)} ago (stale)`;
    case "uncertain":
      return "age uncertain";
  }
}

export function describeUncertainty(reason: Extract<Freshness, { status: "uncertain" }>["reason"]): string {
  switch (reason) {
    case "missing_or_untrusted_time":
      return "No trusted clock or capture time is available, so the age cannot be stated.";
    case "clock_reversed":
      return "The capture time is later than the current time, so the age cannot be stated.";
    case "age_out_of_range":
      return "The capture age is outside the supported range.";
  }
}

/** Why a position is not drawn. Gaps are explanations, never markers. */
export function describeGap(gap: ProjectionGap): string {
  switch (gap.gap) {
    case "namespace_mismatch":
      return "Not drawn: the position belongs to a different dataset namespace (demo and reference data are never mixed).";
    case "no_image_for_map":
      return `Not drawn: this atlas has no imagery for map ${gap.map.map_id}.`;
    case "in_instance":
      return `Not drawn: map ${gap.map.map_id} is an instance, and instances have no imagery here.`;
    case "different_floor":
      return `Not drawn: the position is on floor ${gap.map.floor} of map ${gap.map.map_id}; imagery exists only for floor ${gap.available_floors.join(", ")}.`;
    case "build_mismatch":
      return `Not drawn: the position was recorded on build ${gap.position_build.build}, which this imagery is not declared to cover.`;
    case "embedding_chain_too_deep":
      return "Not drawn: the map nesting is deeper than this atlas supports.";
    case "uncalibrated_image":
      return `Not drawn: image ${gap.image} has no known calibration (${gap.status}).`;
    case "out_of_bounds":
      return `Not drawn: the position falls outside the frame of image ${gap.image}.`;
  }
}

export function describeNotice(notice: ProjectionNotice): string {
  switch (notice.notice) {
    case "cross_build_reuse":
      return `Imagery from build ${notice.image_build.build} is reused for this position's build.`;
    case "embedding_fallback":
      return `Placed through the enclosing map (via ${notice.via.map((m) => m.map_id).join(", ")}), so it is coarser than a position recorded on that map.`;
  }
}

export function describePin(pin: AtlasPin): string {
  switch (pin.kind.kind) {
    case "flight_master":
      return FLIGHT_PIN_LABEL;
    case "settlement":
      return "Reference settlement";
    case "landmark":
      return "Reference landmark";
  }
}

export function describePinAccess(pin: AtlasPin): string | null {
  if (pin.kind.kind !== "flight_master") return null;
  const names = { alliance: "Alliance", horde: "Horde", tower_route: "tower route" } as const;
  return pin.kind.access.length === 0 ? null : `Access: ${pin.kind.access.map((a) => names[a]).join(", ")}`;
}

export function describeLayerAvailability(layer: AtlasLayer): string {
  switch (layer.availability) {
    case "available":
      return "available";
    case "not_embedded":
      return "declared, but no file is installed";
    case "integrity_mismatch":
      return "refused, the file does not match its declared digest";
    case "unservable_id":
      return "cannot be requested";
  }
}
