// Test support: exact runtime shapes for the hand-written atlas DTOs, so the
// golden JSON written by the Rust tests (fixtures/desktop/atlas) is checked
// against what the UI types claim. Object shapes are keyed by the TS type, so a
// field added to or removed from a type without updating its shape fails to compile.
import type {
  AtlasImage,
  AtlasLayer,
  AtlasPin,
  AtlasView,
  Freshness,
  Projection,
  ProjectionGap,
  ProjectionNotice,
  SnapshotMarker,
} from "../commands/contract";
import type { BuildKey, DataContext, MapKey, PixelPoint, Position } from "../../../../schemas/v1/exchange";

export type Shape =
  | { t: "string" | "number" | "boolean" }
  | { t: "enum"; values: readonly string[] }
  | { t: "nullable"; of: Shape }
  | { t: "array"; of: Shape }
  | { t: "object"; fields: Record<string, Shape> }
  | { t: "tagged"; tag: string; variants: Record<string, Record<string, Shape>> }
  | { t: "custom"; check: (value: unknown, path: string) => string[] };

type Fields<T> = { [K in keyof T]-?: Shape };

const str: Shape = { t: "string" };
const num: Shape = { t: "number" };
const bool: Shape = { t: "boolean" };
const nullable = (of: Shape): Shape => ({ t: "nullable", of });
const array = (of: Shape): Shape => ({ t: "array", of });
const oneOf = (...values: string[]): Shape => ({ t: "enum", values });
const object = <T>(fields: Fields<T>): Shape => ({ t: "object", fields: fields as Record<string, Shape> });

const buildKey = object<BuildKey>({ build: str, edition: str, interface: str });
const dataContext = object<DataContext>({ build: buildKey, namespace: oneOf("demo", "reference") });
const mapKey = object<MapKey>({ context: dataContext, floor: str, instance: nullable(str), map_id: str });
const pixel = object<PixelPoint>({ x: num, y: num });
const observationStatus = oneOf("known", "unknown", "unsupported", "restricted", "not_applicable");
const confidence = nullable(
  oneOf("verified_on_build", "publisher_documented", "limited_sample", "cross_edition_unverified", "inferred", "disputed_or_stale"),
);

export const positionShape = object<Position>({ map: mapKey, x: num, y: num });

const layerShape = object<AtlasLayer>({
  id: str,
  kind: oneOf("terrain", "other", "roads_traced", "flight_reference", "overview"),
  url: nullable(str),
  availability: oneOf("available", "not_embedded", "integrity_mismatch", "unservable_id"),
  sha256: str,
  width_px: num,
  height_px: num,
  frame_divisor: num,
});

const pinShape = object<AtlasPin>({
  id: str,
  label: str,
  kind: {
    t: "tagged",
    tag: "kind",
    variants: {
      flight_master: { access: array(oneOf("alliance", "horde", "tower_route")) },
      settlement: {},
      landmark: {},
    },
  },
  pixel,
  measured_on: str,
  evidence: array(str),
  reference_only: bool,
});

const imageShape = object<AtlasImage>({
  id: str,
  map: mapKey,
  width_px: num,
  height_px: num,
  layers: array(layerShape),
  pins: array(pinShape),
  calibration_status: observationStatus,
  calibration_confidence: confidence,
  approximate: bool,
  gaps: str,
  attribution: str,
});

export const atlasViewShape = object<Omit<AtlasView, "page">>({
  data_mode: oneOf("demo", "stored"),
  fixture: bool,
  revision: str,
  namespace: oneOf("demo", "reference"),
  images: array(imageShape),
});

const noticeFields: { [V in ProjectionNotice["notice"]]: Fields<Omit<Extract<ProjectionNotice, { notice: V }>, "notice">> } = {
  cross_build_reuse: { image_build: buildKey },
  embedding_fallback: { via: array(mapKey) },
};
const gapFields: { [V in ProjectionGap["gap"]]: Fields<Omit<Extract<ProjectionGap, { gap: V }>, "gap">> } = {
  namespace_mismatch: {},
  no_image_for_map: { map: mapKey },
  in_instance: { map: mapKey },
  different_floor: { map: mapKey, available_floors: array(str) },
  build_mismatch: { position_build: buildKey, image: nullable(str), applicable_builds: array(buildKey) },
  embedding_chain_too_deep: { via: array(mapKey) },
  uncalibrated_image: { image: str, status: observationStatus },
  out_of_bounds: { image: str, pixel },
};
const placedFields: Fields<Projection> = {
  image: str,
  pixel,
  notices: array({ t: "tagged", tag: "notice", variants: noticeFields }),
  calibration_evidence: array(str),
  calibration_confidence: confidence,
};

/** `placed` carries a Projection; `gap` carries the named ProjectionGap's fields. */
const projectedPositionShape: Shape = {
  t: "custom",
  check: (value, path) => {
    if (typeof value !== "object" || value === null) return [`${path}: expected object`];
    const record = value as Record<string, unknown>;
    if (record["outcome"] === "placed") return objectProblems(value, { outcome: str, ...placedFields }, path);
    if (record["outcome"] === "gap") {
      const gap = record["gap"];
      const fields = typeof gap === "string" ? (gapFields as Record<string, Record<string, Shape>>)[gap] : undefined;
      if (fields === undefined) return [`${path}.gap: unknown gap`];
      return objectProblems(value, { outcome: str, gap: str, ...fields }, path);
    }
    return [`${path}.outcome: unknown outcome`];
  },
};
export const projectedPositionsShape: Shape = array(projectedPositionShape);

const freshnessVariants: { [V in Freshness["status"]]: Record<string, Shape> } = {
  fresh: { age_ms: num },
  stale: { age_ms: num },
  uncertain: { reason: oneOf("missing_or_untrusted_time", "clock_reversed", "age_out_of_range") },
};

const snapshotMarkerShape = object<SnapshotMarker>({
  profile_id: str,
  snapshot_id: str,
  location_status: observationStatus,
  observed_at_utc_ms: nullable(num),
  received_at_utc_ms: nullable(num),
  freshness: { t: "tagged", tag: "status", variants: freshnessVariants },
  projection: nullable(projectedPositionShape),
  approximate: nullable(bool),
  fixture: bool,
  data_mode: oneOf("demo", "stored"),
});
export const snapshotMarkersShape: Shape = array(snapshotMarkerShape);

/** Returns a list of problems; empty means the value has exactly the shape. */
export function problems(value: unknown, shape: Shape, path = "$"): string[] {
  switch (shape.t) {
    case "string":
    case "number":
    case "boolean":
      return typeof value === shape.t ? [] : [`${path}: expected ${shape.t}`];
    case "enum":
      return typeof value === "string" && shape.values.includes(value) ? [] : [`${path}: not one of ${shape.values.join("|")}`];
    case "nullable":
      return value === null ? [] : problems(value, shape.of, path);
    case "array":
      return Array.isArray(value)
        ? value.flatMap((item, i) => problems(item, shape.of, `${path}[${i}]`))
        : [`${path}: expected array`];
    case "object":
      return objectProblems(value, shape.fields, path);
    case "tagged": {
      if (typeof value !== "object" || value === null) return [`${path}: expected object`];
      const tag = (value as Record<string, unknown>)[shape.tag];
      const variant = typeof tag === "string" ? shape.variants[tag] : undefined;
      if (variant === undefined) return [`${path}.${shape.tag}: unknown variant`];
      return objectProblems(value, { [shape.tag]: str, ...variant }, path);
    }
    case "custom":
      return shape.check(value, path);
  }
}

function objectProblems(value: unknown, fields: Record<string, Shape>, path: string): string[] {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return [`${path}: expected object`];
  const record = value as Record<string, unknown>;
  const out: string[] = [];
  for (const key of Object.keys(fields)) {
    if (!(key in record)) out.push(`${path}.${key}: missing`);
  }
  for (const key of Object.keys(record)) {
    const shape = fields[key];
    if (shape === undefined) out.push(`${path}.${key}: unexpected`);
    else out.push(...problems(record[key], shape, `${path}.${key}`));
  }
  return out;
}
