import { createClient } from "./client";
import { demoAdapter } from "./demoAdapter";
import projections from "../../../../fixtures/desktop/atlas/project-positions.json";
import type { Position } from "../../../../schemas/v1/exchange";

const client = createClient(demoAdapter);
const samples = projections.positions as unknown as Position[];

describe("atlas commands through the demo adapter", () => {
  it("query_atlas returns the labelled demo view with browser-bundled layer URLs", async () => {
    const r = await client.queryAtlas();
    if (!r.ok) throw new Error("expected ok");
    expect(r.value).toMatchObject({ data_mode: "demo", fixture: true, namespace: "demo", revision: "demo-imagery-r1" });
    const layers = r.value.images[0]?.layers ?? [];
    expect(layers.map((l) => l.id)).toEqual(["roads", "flight", "overview"]);
    expect(layers.every((l) => l.url !== null && !l.url.startsWith("atlas:"))).toBe(true);
  });

  it("query_atlas rejects an unknown image with a safe INVALID_INPUT", async () => {
    const r = await client.queryAtlas("../../etc/passwd");
    expect(r).toEqual({ ok: false, error: { code: "INVALID_INPUT", message: "No such atlas image", retryable: false } });
  });

  it("project_positions answers its sample list and refuses anything else", async () => {
    const ok = await client.projectPositions(samples);
    expect(ok.ok && ok.value.length).toBe(samples.length);
    const other = await client.projectPositions(samples.slice(0, 2));
    expect(other.ok).toBe(false);
  });

  it("project_positions bounds the request without echoing it", async () => {
    for (const bad of [[], Array<Position>(65).fill(samples[0] as Position)]) {
      const r = await client.projectPositions(bad);
      expect(r).toEqual({
        ok: false,
        error: { code: "INVALID_INPUT", message: "Provide between 1 and 64 positions", retryable: false },
      });
    }
  });

  it("get_snapshot_markers has no trusted clock in a browser, so the age is uncertain", async () => {
    const r = await client.getSnapshotMarkers();
    expect(r.ok && r.value[0]?.freshness).toEqual({ status: "uncertain", reason: "missing_or_untrusted_time" });
  });
});
