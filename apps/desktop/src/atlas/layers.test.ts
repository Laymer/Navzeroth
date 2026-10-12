import { chooseLayer } from "./layers";
import type { AtlasImage, AtlasLayer } from "../commands/contract";

const layer = (id: string, kind: AtlasLayer["kind"], extra: Partial<AtlasLayer> = {}): AtlasLayer => ({
  id,
  kind,
  url: `atlas://localhost/img/${id}`,
  availability: "available",
  sha256: "0".repeat(64),
  width_px: 1,
  height_px: 1,
  frame_divisor: 1,
  ...extra,
});
const image = (layers: AtlasLayer[]) => ({ layers }) as unknown as AtlasImage;
const none = new Set<string>();

describe("chooseLayer", () => {
  const layers = [layer("roads", "roads_traced"), layer("flight", "flight_reference"), layer("overview", "overview")];
  it("prefers flight, then roads, then the base layer", () => {
    expect(chooseLayer(image(layers), { roads: true, flight: true }, none)?.id).toBe("flight");
    expect(chooseLayer(image(layers), { roads: true, flight: false }, none)?.id).toBe("roads");
    expect(chooseLayer(image(layers), { roads: false, flight: false }, none)?.id).toBe("overview");
  });
  it("skips layers that failed to load or are not available", () => {
    expect(chooseLayer(image(layers), { roads: true, flight: true }, new Set(["flight"]))?.id).toBe("roads");
    const refused = [layer("roads", "roads_traced", { url: null, availability: "integrity_mismatch" })];
    expect(chooseLayer(image(refused), { roads: true, flight: false }, none)).toBeNull();
  });
  it("returns null when nothing can be drawn, which selects the schematic fallback", () => {
    expect(chooseLayer(image([]), { roads: true, flight: true }, none)).toBeNull();
    expect(chooseLayer(image(layers), { roads: true, flight: true }, new Set(["roads", "flight", "overview"]))).toBeNull();
  });
});
