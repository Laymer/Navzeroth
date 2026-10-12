// The golden JSON is written by crates/app/tests/atlas_commands.rs. These tests
// parse it against the hand-written DTO types, so Rust and TypeScript cannot
// drift apart without a failing test.
import atlas from "../../../../fixtures/desktop/atlas/query-atlas.json";
import projections from "../../../../fixtures/desktop/atlas/project-positions.json";
import fresh from "../../../../fixtures/desktop/atlas/snapshot-markers-fresh.json";
import stale from "../../../../fixtures/desktop/atlas/snapshot-markers-stale.json";
import uncertain from "../../../../fixtures/desktop/atlas/snapshot-markers-uncertain.json";
import restricted from "../../../../fixtures/desktop/atlas/snapshot-markers-location-restricted.json";
import otherFloor from "../../../../fixtures/desktop/atlas/snapshot-markers-gap-different-floor.json";
import type { AtlasView, ProjectedPosition, SnapshotMarker } from "../commands/contract";
import {
  atlasViewShape,
  positionShape,
  problems,
  projectedPositionsShape,
  snapshotMarkersShape,
  type Shape,
} from "./golden.shapes";

describe("golden atlas DTO JSON", () => {
  it("query_atlas parses as AtlasView", () => {
    expect(problems(atlas, atlasViewShape)).toEqual([]);
    const view = atlas as unknown as AtlasView;
    expect(view.data_mode).toBe("demo");
    expect(view.images[0]?.layers.map((l) => l.id)).toEqual(["roads", "flight", "overview"]);
    expect(view.images[0]?.pins.every((pin) => pin.reference_only)).toBe(true);
  });

  it("project_positions parses as one ProjectedPosition per input position", () => {
    expect(problems(projections.results, projectedPositionsShape)).toEqual([]);
    expect(problems(projections.positions, { t: "array", of: positionShape } as Shape)).toEqual([]);
    const results = projections.results as unknown as ProjectedPosition[];
    expect(results.length).toBe(projections.cases.length);
    const byCase = new Map(projections.cases.map((name, i) => [name, results[i]]));
    expect(byCase.get("placed")?.outcome).toBe("placed");
    expect(byCase.get("embedding_fallback")).toMatchObject({ outcome: "placed", notices: [{ notice: "embedding_fallback" }] });
    for (const gap of ["namespace_mismatch", "different_floor", "build_mismatch"]) {
      expect(byCase.get(gap)).toMatchObject({ outcome: "gap", gap });
    }
  });

  it.each([
    ["fresh", fresh, "fresh"],
    ["stale", stale, "stale"],
    ["uncertain", uncertain, "uncertain"],
    ["location restricted", restricted, "fresh"],
    ["gap on another floor", otherFloor, "fresh"],
  ])("get_snapshot_markers (%s) parses as SnapshotMarker[]", (_name, golden, freshness) => {
    expect(problems(golden, snapshotMarkersShape)).toEqual([]);
    const [marker] = golden as unknown as SnapshotMarker[];
    expect(marker?.freshness.status).toBe(freshness);
    expect(marker?.data_mode).toBe("demo");
    expect(marker?.fixture).toBe(true);
  });

  it("the validator itself rejects drift", () => {
    const withExtra = structuredClone(atlas) as unknown as Record<string, unknown>;
    withExtra["live"] = true;
    expect(problems(withExtra, atlasViewShape)).toContain("$.live: unexpected");
    const missing = structuredClone(atlas) as unknown as Record<string, unknown>;
    delete missing["revision"];
    expect(problems(missing, atlasViewShape)).toContain("$.revision: missing");
    const wrong = structuredClone(fresh);
    (wrong[0] as unknown as Record<string, unknown>)["fixture"] = "yes";
    expect(problems(wrong, snapshotMarkersShape)).toContain("$[0].fixture: expected boolean");
    const unknownGap = [{ outcome: "gap", gap: "teleported" }];
    expect(problems(unknownGap, projectedPositionsShape)).toContain("$[0].gap: unknown gap");
  });
});
