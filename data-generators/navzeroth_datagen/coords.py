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
"""
# UiMap.Type: 0 cosmic, 1 world, 2 continent, 3 zone, 4 dungeon, 5 micro, 6 orphan
TYPE_RANK = {3: 0, 4: 1, 5: 2, 6: 3, 2: 4, 1: 5, 0: 6}


def _f(row, key):
    return float(row[key])


class Locator:
    def __init__(self, assignments, ui_maps):
        """assignments: UiMapAssignment rows; ui_maps: UiMap rows (for Type)."""
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

    def locate(self, map_id, x, y, z=0.0):
        """{ui_map_id, x, y} rounded to 4 decimals, or None when no assignment holds it."""
        for _, ui, lo, hi, (u0, v0, u1, v1) in self.by_map.get(map_id, ()):
            if lo[0] <= x <= hi[0] and lo[1] <= y <= hi[1] and lo[2] <= z <= hi[2]:
                u = (hi[1] - y) / (hi[1] - lo[1])
                v = (hi[0] - x) / (hi[0] - lo[0])
                return {"ui_map_id": ui, "x": round(u0 + (u1 - u0) * u, 4), "y": round(v0 + (v1 - v0) * v, 4)}
        return None

    def observation(self, map_id, x, y, z=0.0):
        """Envelope Observation: known position, or unsupported when no uiMap covers it."""
        p = self.locate(map_id, x, y, z)
        return {"status": "known", "value": p} if p else {"status": "unsupported"}
