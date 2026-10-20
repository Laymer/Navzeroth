"""Independent source grouping cases, not a second call to the production mapper."""
import importlib.util
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('dungeon_oracle', ROOT/'scripts/validate-dungeon-associations.py')
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)


class DungeonAssociationOracle(unittest.TestCase):
    def test_source_membership_one_hop_and_undefined_items_keep_precise_identity_sets(self):
        dungeon = dict(map_id=1, name='Synthetic Dungeon', area_id=10, bosses=[20])
        rows, excluded = oracle.relations([dungeon], [dict(id=1, zone_or_sort=10), dict(id=2, zone_or_sort=-10)],
            [dict(id=30, inventory_type=0), dict(id=31, inventory_type=1), dict(id=32, inventory_type=99)],
            [dict(entry=20, item=30), dict(entry=21, item=31), dict(entry=20, reference=40), dict(entry=20, reference=40)],
            [dict(entry=40, item=31), dict(entry=40, item=32), dict(entry=40, item=999), dict(entry=40, reference=41), dict(entry=41, item=30)])
        self.assertFalse(excluded)
        self.assertEqual(rows[0]['quest_count'], 1)
        self.assertEqual(rows[0]['reward_count'], 2)
        self.assertEqual(rows[0]['quest_ids_sha256'], oracle.identity_hash(['quest:1']))
        self.assertEqual(rows[0]['reward_ids_sha256'], oracle.identity_hash(['drop:creature:20:30', 'drop:creature:20:ref:40:31']))

    def test_empty_known_associations_exclusion_and_reviewed_display_identity(self):
        rows, excluded = oracle.relations([dict(map_id=36,name='Deadmines',area_id=10,bosses=[]),
            dict(map_id=169,name='Test',excluded_reason='Explicit synthetic source test exclusion')], [], [], [], [])
        self.assertEqual(rows[0]['id'],'dungeon:the-deadmines')
        self.assertEqual(rows[0]['source_name'],'Deadmines')
        self.assertEqual(rows[0]['quest_count'],0)
        self.assertEqual(rows[0]['reward_ids_sha256'],oracle.identity_hash([]))
        self.assertEqual(excluded[0]['map_id'],169)

    def test_ambiguous_source_membership_and_duplicate_id_hashes_refuse(self):
        for second in [dict(map_id=2,name='Other',bosses=[20]),dict(map_id=2,name='Same',bosses=[21])]:
            with self.subTest(second=second),self.assertRaises(ValueError):
                oracle.relations([dict(map_id=1,name='Same',bosses=[20]),second],[],[],[],[])
        with self.assertRaises(ValueError):
            oracle.identity_hash(['quest:1','quest:1'])
