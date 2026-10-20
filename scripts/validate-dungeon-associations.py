#!/usr/bin/env python3
"""Independent source-relation oracle; never imports the production mapper."""
import argparse
import collections
import hashlib
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'data-generators'))
from navzeroth_datagen import bounded

OUTPUT = 'docs/data/dungeon-associations-v1.json'
DATASETS = ('dungeons/dungeons', 'quests/quests', 'items/items', 'loot/creature', 'loot/reference')
ORIGINS = {'vanilla_1_12': '1.12.1', 'wow_forever': '1.60.1.70245'}


def encoded(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')) + '\n').encode()


def identity_hash(values):
    """Hash sorted unique string IDs as compact UTF8 JSON, with no final newline."""
    if len(values) != len(set(values)):
        raise ValueError('duplicate source association')
    return hashlib.sha256(json.dumps(sorted(values), ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()


def relations(dungeons, quests, items, creature_loot, reference_loot):
    """Source boss membership and one-hop loot references, without probability math."""
    bosses = {}
    records = {}
    excluded = []
    for dungeon in dungeons:
        if dungeon.get('excluded_reason'):
            excluded.append({'map_id': dungeon['map_id'], 'reason': dungeon['excluded_reason']})
            continue
        identity = ('dungeon:the-deadmines' if dungeon['map_id'] == 36 else
                    'dungeon:' + re.sub('[^a-z0-9]+', '-', dungeon['name'].lower()).strip('-'))
        if identity in records:
            raise ValueError('ambiguous source dungeon identity')
        records[identity] = {'quests': set(), 'rewards': set(), 'area': dungeon.get('area_id'), 'map_id': dungeon['map_id'], 'source_name': dungeon['name']}
        for boss in dungeon.get('bosses', []):
            if boss in bosses:
                raise ValueError('ambiguous source boss membership')
            bosses[boss] = identity
    valid_items = {item['id'] for item in items if type(item.get('inventory_type')) is int
                   and 0 <= item['inventory_type'] <= 28}
    references = collections.defaultdict(set)
    for row in reference_loot:
        if 'item' in row and row['item'] in valid_items:
            references[row['entry']].add(row['item'])
    for row in creature_loot:
        dungeon = bosses.get(row['entry'])
        if dungeon is None:
            continue
        if 'reference' in row:
            for item in references[row['reference']]:
                records[dungeon]['rewards'].add(f"drop:creature:{row['entry']}:ref:{row['reference']}:{item}")
        elif row.get('item') in valid_items:
            records[dungeon]['rewards'].add(f"drop:creature:{row['entry']}:{row['item']}")
    for quest in quests:
        area = quest.get('zone_or_sort', 0)
        if area > 0:
            for record in records.values():
                if area == record['area']:
                    record['quests'].add(f"quest:{quest['id']}")
    return [dict(id=identity, map_id=row['map_id'], source_name=row['source_name'], quest_count=len(row['quests']), reward_count=len(row['rewards']),
                 quest_ids_sha256=identity_hash(list(row['quests'])),
                 reward_ids_sha256=identity_hash(list(row['rewards'])))
            for identity, row in sorted(records.items())], sorted(excluded, key=lambda x: x['map_id'])


def derive(root):
    budget = bounded.InputBudget()
    registry = bounded.json_document(budget.read(root / 'docs/sources/source-admission-v1.json', 8*1024*1024))
    admitted = {s['id']: s for s in registry['sources']}
    inputs = []
    source_ids = set()
    selected = {}
    for dataset in DATASETS:
        gathered = []
        for path in bounded.input_files(root / 'data-generators/output' / dataset, '*.json'):
            raw = budget.read(path, bounded.MAX_SHARD_BYTES)
            shard = bounded.json_document(raw)
            origin = shard['origin']
            if (origin['edition'] not in ORIGINS or origin['build'] != {'status': 'known', 'value': ORIGINS[origin['edition']]}
                    or (dataset != 'items/items' and origin['edition'] != 'vanilla_1_12')
                    or shard['dataset'] != dataset.replace('/', '.') or shard['format'] != 'navzeroth-world-data'):
                raise ValueError('oracle only supports the reviewed exact historical source inputs')
            for source in shard['sources']:
                source_ids.add(source['id'])
            inputs.append(dict(path=path.relative_to(root).as_posix(), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest()))
            rank = 0 if origin['edition'] == 'wow_forever' else 1
            gathered.append((rank, path.as_posix(), shard['records']))
        if not gathered:
            raise ValueError('required source relation dataset is absent')
        rows = []
        identities = set()
        for _, _, values in sorted(gathered):
            budget.add_records(len(values))
            for row in values:
                if dataset == 'items/items':
                    if row['id'] in identities:
                        continue
                    identities.add(row['id'])
                rows.append(row)
        selected[dataset] = rows
    if source_ids != {'cmangos-classic-db', 'wago-tools-wowf'}:
        raise ValueError('oracle source set differs from the reviewed source corpus')
    originals = {}
    for sid in sorted(source_ids):
        source = admitted[sid]
        if source['rights']['status'] != 'admitted':
            raise ValueError('source originals are not admitted')
        for file in source['files']:
            path=pathlib.PurePosixPath(file['path'])
            if path.is_absolute() or '..' in path.parts or str(path)!=file['path']:
                raise ValueError('invalid admitted original path')
            raw = budget.read(root / file['path'], 64*1024*1024)
            if len(raw) != file['bytes'] or hashlib.sha256(raw).hexdigest() != file['sha256']:
                raise ValueError('admitted source original changed')
            originals[file['path']] = {k: file[k] for k in ('path', 'bytes', 'sha256')}
    rows, excluded = relations(*(selected[dataset] for dataset in DATASETS))
    return dict(schema_version=1, kind='independent_historical_dungeon_associations', game_observation=False,
                basis='Map36 uses the reviewed authored display-identity alias dungeon:the-deadmines (source name retained); other IDs slug source names. Positive source quest zone_or_sort equals dungeon area; source boss membership, direct creature loot and one reference-table hop; defined admitted items after historical Forever-before-CMaNGOS item precedence. Reference repetitions merge identities; nested references are not expanded. This does not establish eligibility, access, probabilities, current drops or whole-game coverage.',
                identity_hash_format='sha256(compact UTF8 JSON of lexically sorted unique string IDs; no trailing newline)',
                input_shards=sorted(inputs, key=lambda x: x['path']),
                admitted_originals=sorted(originals.values(), key=lambda x: x['path']),
                dungeons=rows, excluded_dungeons=excluded)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=pathlib.Path, default=ROOT)
    parser.add_argument('--write', action='store_true', help='Write the reviewed compact expectation artifact; otherwise verify it')
    args = parser.parse_args()
    try:
        result = encoded(derive(args.root))
        if len(result) > 1024*1024:
            raise ValueError('oracle artifact exceeds 1 MiB')
        destination = args.root / OUTPUT
        if args.write:
            destination.write_bytes(result)
        elif bounded.read_file(destination, 1024*1024) != result:
            raise ValueError('committed source association oracle differs')
        print(f'{len(result)} bytes; independent source associations match')
    except (ValueError, KeyError, OSError) as error:
        parser.exit(1, f'dungeon association oracle: {error}\n')


if __name__ == '__main__':
    main()
