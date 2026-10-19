"""World coordinates (map, x, y, z in yards) to a uiMap position normalized 0..1.

Uses the client's UiMapAssignment rows (wago.tools CSV dicts) and UiMap types.
Region_0..2 are the minimum x, y, z and Region_3..5 the maximum. World x grows
north and y grows west, so on the map image (origin top-left):

    u = (max_y - y) / (max_y - min_y)      v = (max_x - x) / (max_x - min_x)

then scaled into the assignment's UiMin..UiMax. Checked against known positions:
Marshal Dughan (CMaNGOS creature 240, -9465.52, 74.01) is Elwynn Forest (uiMap 1429)
42.1, 65.9, and Hogger (448, -9947.88, 594.77) is 27.1, 86.8.

When several assignments contain a point, the most specific wins: zone maps,
then dungeon and micro maps, then continents; smaller regions first. Assignments
tied to a WMO group or doodad are skipped (they need indoor context).

Zone rectangles overlap widely at borders (Hogger's spawns lie in the Elwynn,
Westfall and Duskwood rectangles). Optional `hints`, points already known to lie
on a uiMap as (map_id, ui_map_id, x, y), settle that: among several containing
assignments of the most specific type, the uiMap with the nearest hint wins;
without a hint in reach the order above holds.
"""
import math

HINT_CELL = 128.0  # yards per hint grid cell
HINT_RINGS = 24  # cells searched around a point (about 3 km)
# UiMap.Type: 0 cosmic, 1 world, 2 continent, 3 zone, 4 dungeon, 5 micro, 6 orphan
TYPE_RANK = {3: 0, 4: 1, 5: 2, 6: 3, 2: 4, 1: 5, 0: 6}


def _ring(cx, cy, r):
    """Grid cells at Chebyshev distance r from (cx, cy)."""
    if r == 0:
        yield cx, cy
        return
    for i in range(cx - r, cx + r + 1):
        yield i, cy - r
        yield i, cy + r
    for j in range(cy - r + 1, cy + r):
        yield cx - r, j
        yield cx + r, j


def _f(row, key):
    return float(row[key])


class Locator:
    def __init__(self, assignments, ui_maps, hints=()):
        """assignments: UiMapAssignment rows; ui_maps: UiMap rows (for Type);
        hints: (map_id, ui_map_id, x, y) points known to lie on that uiMap."""
        types = {int(r["ID"]): int(r.get("Type") or 6) for r in ui_maps}
        self.by_map = {}
        for r in assignments:
            if int(r.get("WMOGroupID") or 0) or int(r.get("WMODoodadPlacementID") or 0):
                continue
            lo = (_f(r, "Region_0"), _f(r, "Region_1"), _f(r, "Region_2"))
            hi = (_f(r, "Region_3"), _f(r, "Region_4"), _f(r, "Region_5"))
            if hi[0] <= lo[0] or hi[1] <= lo[1]:
                continue
            ui = int(r["UiMapID"])
            area = (hi[0] - lo[0]) * (hi[1] - lo[1])
            rank = (TYPE_RANK.get(types.get(ui, 6), 9), area, int(r.get("OrderIndex") or 0), ui)
            uv = (_f(r, "UiMin_0"), _f(r, "UiMin_1"), _f(r, "UiMax_0"), _f(r, "UiMax_1"))
            self.by_map.setdefault(int(r["MapID"]), []).append((rank, ui, lo, hi, uv))
        for entries in self.by_map.values():
            entries.sort()
        self.hints = {}
        for map_id, ui, hx, hy in hints:
            cell = (math.floor(hx / HINT_CELL), math.floor(hy / HINT_CELL))
            self.hints.setdefault((int(map_id), int(ui)), {}).setdefault(cell, []).append((float(hx), float(hy)))

    def candidates(self, map_id, x, y, z=0.0):
        """UiMap IDs of the assignments holding the point that share the most specific type, best first."""
        return [e[1] for e in self._hits(map_id, x, y, z)]

    def _hits(self, map_id, x, y, z):
        hits = [e for e in self.by_map.get(map_id, ())
                if e[2][0] <= x <= e[3][0] and e[2][1] <= y <= e[3][1] and e[2][2] <= z <= e[3][2]]
        return [e for e in hits if e[0][0] == hits[0][0][0]]

    def _nearest(self, map_id, ui, x, y):
        """Squared distance to the nearest hint on (map_id, ui) within HINT_RINGS cells, or inf."""
        grid = self.hints.get((map_id, ui))
        if not grid:
            return math.inf
        cx, cy = math.floor(x / HINT_CELL), math.floor(y / HINT_CELL)
        best = math.inf
        for ring in range(HINT_RINGS + 1):
            if best < ((ring - 1) * HINT_CELL) ** 2:
                break
            for cell in _ring(cx, cy, ring):
                for hx, hy in grid.get(cell, ()):
                    best = min(best, (hx - x) ** 2 + (hy - y) ** 2)
        return best

    def locate(self, map_id, x, y, z=0.0):
        """{ui_map_id, x, y} rounded to 4 decimals, or None when no assignment holds it."""
        hits = self._hits(map_id, x, y, z)
        if not hits:
            return None
        pick = hits[0]
        if len(hits) > 1 and self.hints:
            dist, i = min((self._nearest(map_id, e[1], x, y), i) for i, e in enumerate(hits))
            if dist < math.inf:
                pick = hits[i]
        _, ui, lo, hi, (u0, v0, u1, v1) = pick
        u = (hi[1] - y) / (hi[1] - lo[1])
        v = (hi[0] - x) / (hi[0] - lo[0])
        return {"ui_map_id": ui, "x": round(u0 + (u1 - u0) * u, 4), "y": round(v0 + (v1 - v0) * v, 4)}

    def observation(self, map_id, x, y, z=0.0):
        """Envelope Observation: known position, or unsupported when no uiMap covers it."""
        p = self.locate(map_id, x, y, z)
        return {"status": "known", "value": p} if p else {"status": "unsupported"}
