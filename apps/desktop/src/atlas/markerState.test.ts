import fresh from "../../../../fixtures/desktop/atlas/snapshot-markers-fresh.json";
import stale from "../../../../fixtures/desktop/atlas/snapshot-markers-stale.json";
import uncertain from "../../../../fixtures/desktop/atlas/snapshot-markers-uncertain.json";
import restricted from "../../../../fixtures/desktop/atlas/snapshot-markers-location-restricted.json";
import otherFloor from "../../../../fixtures/desktop/atlas/snapshot-markers-gap-different-floor.json";
import type { ProjectionGap, SnapshotMarker } from "../commands/contract";
import { describeAge, describeGap } from "./describe";
import { MARKER_STATE_KEYS, MARKER_STATE_LABELS, markerState, placementOf } from "./markerState";

const first = (golden: unknown) => (golden as SnapshotMarker[])[0] ?? null;
const GAPS: Record<ProjectionGap["gap"], true> = {
  namespace_mismatch: true,
  no_image_for_map: true,
  in_instance: true,
  different_floor: true,
  build_mismatch: true,
  embedding_chain_too_deep: true,
  uncalibrated_image: true,
  out_of_bounds: true,
};

describe("markerState", () => {
  it("covers every placed, gap and unavailable state exactly once", () => {
    const expected = [
      "no_snapshot",
      "projection_missing",
      "location_unknown",
      "location_unsupported",
      "location_restricted",
      "location_not_applicable",
      ...(["fresh", "stale", "uncertain"] as const).flatMap((f) =>
        (["approximate", "calibrated"] as const).map((c) => `placed_${f}_${c}`),
      ),
      ...Object.keys(GAPS).map((g) => `gap_${g}`),
    ];
    expect([...MARKER_STATE_KEYS].sort()).toEqual([...expected].sort());
  });

  it("has pairwise distinct labels that never say live", () => {
    const labels = Object.values(MARKER_STATE_LABELS);
    expect(new Set(labels).size).toBe(labels.length);
    for (const label of labels) expect(label).not.toMatch(/\blive\b/i);
  });

  it("classifies the golden markers", () => {
    expect(markerState(null)).toBe("no_snapshot");
    expect(markerState(first(fresh))).toBe("placed_fresh_approximate");
    expect(markerState(first(stale))).toBe("placed_stale_approximate");
    expect(markerState(first(uncertain))).toBe("placed_uncertain_approximate");
    expect(markerState(first(restricted))).toBe("location_restricted");
    expect(markerState(first(otherFloor))).toBe("gap_different_floor");
  });

  it("treats a missing calibration flag as approximate and a known location without projection as its own state", () => {
    const placed = structuredClone(first(fresh)) as SnapshotMarker;
    placed.approximate = null;
    expect(markerState(placed)).toBe("placed_fresh_approximate");
    placed.approximate = false;
    expect(markerState(placed)).toBe("placed_fresh_calibrated");
    placed.projection = null;
    expect(markerState(placed)).toBe("projection_missing");
  });

  it("only places a marker the backend placed", () => {
    expect(placementOf(first(fresh))).toEqual({ image: "demo-image-1", pixel: { x: 16, y: 96 }, approximate: true });
    expect(placementOf(first(restricted))).toBeNull();
    expect(placementOf(first(otherFloor))).toBeNull();
    expect(placementOf(null)).toBeNull();
  });
});

describe("describe", () => {
  it("states the capture age without claiming it is live", () => {
    expect(describeAge({ status: "fresh", age_ms: 60_000 })).toBe("captured 1 min ago");
    expect(describeAge({ status: "stale", age_ms: 31 * 60_000 })).toBe("captured 31 min ago (stale)");
    expect(describeAge({ status: "uncertain", reason: "missing_or_untrusted_time" })).toBe("age uncertain");
  });
  it("explains a gap in words", () => {
    const gap = (first(otherFloor)?.projection ?? null) as { outcome: "gap" } & ProjectionGap;
    expect(describeGap(gap)).toMatch(/^Not drawn: the position is on floor 1/);
  });
});
