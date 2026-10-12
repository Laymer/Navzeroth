import type { AtlasImage, AtlasLayer } from "../commands/contract";

export interface LayerToggles {
  roads: boolean;
  flight: boolean;
}

function usable(layer: AtlasLayer | undefined, failed: ReadonlySet<string>): layer is AtlasLayer {
  return layer !== undefined && layer.availability === "available" && layer.url !== null && !failed.has(layer.id);
}

/**
 * The one raster layer to draw: the flight reference layer if toggled on, else
 * the traced roads layer if toggled on, else the base (overview or terrain).
 * Layers are full images, so the topmost enabled one replaces the others.
 * Returns null when nothing can be drawn, which is the schematic fallback.
 */
export function chooseLayer(image: AtlasImage, toggles: LayerToggles, failed: ReadonlySet<string>): AtlasLayer | null {
  const byKind = (kind: AtlasLayer["kind"]) => image.layers.find((l) => l.kind === kind);
  const candidates = [
    toggles.flight ? byKind("flight_reference") : undefined,
    toggles.roads ? byKind("roads_traced") : undefined,
    byKind("overview"),
    byKind("terrain"),
    byKind("other"),
  ];
  return candidates.find((c) => usable(c, failed)) ?? null;
}
