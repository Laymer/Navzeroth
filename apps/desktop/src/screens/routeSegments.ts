import type { RouteInspection } from "../commands/routes";
import type { MapKey } from "../../../../schemas/v1/exchange";
import type { Bounds, Point } from "../atlas/viewport";
export const ROUTE_PAGE_SIZE = 50;
export type RouteFrame = RouteInspection["frames"][number];
export type InspectedHop = RouteInspection["hops"][number];
function sameMap(a: MapKey, b: MapKey): boolean {
  return a.map_id === b.map_id && a.floor === b.floor && (a.instance ?? null) === (b.instance ?? null) && a.context.namespace === b.context.namespace
    && a.context.build.edition === b.context.build.edition && a.context.build.build === b.context.build.build && a.context.build.interface === b.context.build.interface;
}
export function endpoints(value: RouteInspection, frame: RouteFrame, hop: InspectedHop): [Point, Point] | null {
  if (!(frame.width_px > 0) || !(frame.height_px > 0) || !Number.isFinite(frame.width_px) || !Number.isFinite(frame.height_px) || hop.schematic_connection !== true || hop.from.outcome !== "placed" || hop.to.outcome !== "placed"
    || hop.from.image !== frame.id || hop.to.image !== frame.id || !sameMap(hop.hop.from.map, frame.map) || !sameMap(hop.hop.to.map, frame.map)
    || frame.map.context.namespace !== value.context.namespace || frame.map.context.build.edition !== value.context.build.edition
    || frame.map.context.build.build !== value.context.build.build || frame.map.context.build.interface !== value.context.build.interface) return null;
  const points: [Point, Point] = [hop.from.pixel, hop.to.pixel];
  return points.every((p) => Number.isFinite(p.x) && Number.isFinite(p.y) && p.x >= 0 && p.y >= 0 && p.x <= frame.width_px && p.y <= frame.height_px) ? points : null;
}
export function routeBounds(value: RouteInspection, frame: RouteFrame, indices?: readonly number[]): { bounds: Bounds | null; count: number } {
  let bounds: Bounds | null = null; let count = 0;
  const visit = (hop: InspectedHop) => {
    const points = endpoints(value, frame, hop); if (!points) return;
    count++;
    for (const p of points) bounds = bounds ? { minX: Math.min(bounds.minX, p.x), minY: Math.min(bounds.minY, p.y), maxX: Math.max(bounds.maxX, p.x), maxY: Math.max(bounds.maxY, p.y) } : { minX: p.x, minY: p.y, maxX: p.x, maxY: p.y };
  };
  if (indices) for (const i of indices) { const hop = value.hops[i]; if (hop) visit(hop); }
  else for (const hop of value.hops) visit(hop);
  return { bounds, count };
}
