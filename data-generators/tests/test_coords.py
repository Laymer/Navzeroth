import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
from navzeroth_datagen.coords import Locator  # noqa: E402


def row(ui, map_id, lo, hi, order=0, wmo=0):
    r = {"UiMapID": str(ui), "MapID": str(map_id), "OrderIndex": str(order), "WMOGroupID": str(wmo),
         "WMODoodadPlacementID": "0", "UiMin_0": "0", "UiMin_1": "0", "UiMax_0": "1", "UiMax_1": "1"}
    for i, v in enumerate(lo + hi):
        r[f"Region_{i}"] = str(v)
    return r


# wowf 1.60.1.70245 rows for Eastern Kingdoms (1415) and Elwynn Forest (1429)
ROWS = [row(1415, 0, (-16000, -19199.900390625, -1e6), (7466.6000976562, 16000, 1e6)),
        row(1429, 0, (-10254.166015625, -1935.4166259766, -1e6), (-7939.5830078125, 1535.4166259766, 1e6)),
        row(9999, 0, (-9500, 0, -1e6), (-9400, 100, 1e6), wmo=5)]
MAPS = [{"ID": "1415", "Type": "2"}, {"ID": "1429", "Type": "3"}, {"ID": "9999", "Type": "5"}]


class LocatorTest(unittest.TestCase):
    def test_known_npcs(self):
        loc = Locator(ROWS, MAPS)
        dughan = loc.locate(0, -9465.52, 74.0069, 56.7785)
        self.assertEqual(dughan["ui_map_id"], 1429)
        self.assertAlmostEqual(dughan["x"], 0.421, places=3)
        self.assertAlmostEqual(dughan["y"], 0.659, places=3)
        hogger = loc.locate(0, -9947.88, 594.773)
        self.assertAlmostEqual(hogger["x"], 0.271, places=3)
        self.assertAlmostEqual(hogger["y"], 0.868, places=3)

    def test_continent_fallback_and_unsupported(self):
        loc = Locator(ROWS, MAPS)
        self.assertEqual(loc.locate(0, 0, 0)["ui_map_id"], 1415)
        self.assertIsNone(loc.locate(36, 0, 0))
        self.assertEqual(loc.observation(36, 0, 0), {"status": "unsupported"})
        self.assertEqual(loc.observation(0, 0, 0)["status"], "known")


if __name__ == "__main__":
    unittest.main()
