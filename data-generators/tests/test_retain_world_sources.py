"""Exact-source closure tests using synthetic grants/cache/repositories; no network."""
import gzip
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import retain_world_sources as retain

BUILD = '1.60.1.70245'


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], stderr=subprocess.DEVNULL).decode().strip()


class Fixture:
    def __init__(self, directory):
        self.root = pathlib.Path(directory) / 'repo'
        self.root.mkdir()
        self.cache = pathlib.Path(directory) / 'cache'
        self.world = self.root / 'world'
        self.world.mkdir()
        self.registry = self.root / 'registry.json'
        self.pack = self.root / 'pack.json'
        self.output = self.root / 'closure.json'
        files = {
            'LICENSE': 'Synthetic original license for tests only.',
            'data-generators/world_to_content.py': 'VERSION = "7"\n',
            'data-generators/navzeroth_datagen/bounded.py': '# Synthetic original helper\n',
            'data-generators/navzeroth_datagen/sqldump.py': '# Synthetic parser, derived under separate permission\n',
            'data-generators/Navzeroth-data-generators/gen_world_db2.py': "TABLES = {'world.areas': ('AreaTable',)}\n",
            'data-generators/TALOD-data-generators/tools/gen_fishing.py': '# Synthetic separately-permitted source\n',
            'data-generators/TALOD-data-generators/PERMISSION.md': 'Synthetic permission fixture, not a real grant.',
            'data-generators/NOTICE.md': 'Synthetic notice preserving distinct source terms.',
        }
        for relative, text in files.items():
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        git(self.root, 'init', '-q')
        git(self.root, 'config', 'user.name', 'Synthetic test')
        git(self.root, 'config', 'user.email', 'test@example.invalid')
        git(self.root, 'add', '.')
        git(self.root, 'commit', '-qm', 'Synthetic code inputs')
        self.commit = git(self.root, 'rev-parse', 'HEAD')
        proof = retain.retain(self.root, b'Synthetic pre-admitted proof; not a grant')
        sources = []
        for sid in sorted(retain.UPSTREAMS):
            sources.append({'id': sid, 'revision': 'a'*40, 'publisher':'Synthetic publisher', 'rights': {'status': 'admitted', 'license_identifier':'LicenseRef-Synthetic-Test', 'attribution':'Synthetic original author', 'evidence': [proof]},
                'retrieval': {'access_evidence': proof, 'max_total_bytes': 100000, 'max_requests': 128}, 'files': []})
        self.registry.write_text(json.dumps({'schema_version': 1, 'sources': sources}))
        self.policy = self.root / 'policy.json'
        rules = [dict(id=s['id'], admission_revision=s['revision'], grant_sha256=retain.grant_identity(s), origin={'edition':'wow_forever' if s['id']=='wago-tools-wowf' else 'vanilla_1_12','build':BUILD if s['id']=='wago-tools-wowf' else '1.12.1'},allowed_fields=['quests.title']) for s in sources]
        rules.append(dict(id=retain.DERIVED,allowed_fields=['quests.title'],publisher='Synthetic publisher',attribution='Synthetic original author'))
        self.policy.write_text(json.dumps(dict(schema_version=1,transformation='world_to_content_v7',sources=rules)))
        self.pack.write_text(json.dumps({'sources': [dict(id=sid, publisher='Synthetic publisher',
            allowed_fields={'status': 'known', 'value': ['quests.title']},
            attribution={'status': 'known', 'value': 'Synthetic original author'})
            for sid in sorted(retain.UPSTREAMS | {retain.DERIVED})]}))
        self.urls = {}
        self.metas = {}
        for table in retain.CMANGOS_TABLES:
            url = f'https://wago.tools/db2/{table}/csv?product=wowf&build={BUILD}'
            self.cache_entry(table, url, f'ID,{table}\n1,Synthetic\n'.encode())
            self.urls[table] = url
        raw = gzip.compress(b'-- Synthetic SQL, parsed as inert data\n', mtime=0)
        name = 'ClassicDB_synthetic.sql.gz'
        blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        cm_url = f'https://raw.githubusercontent.com/cmangos/classic-db/master/Full_DB/{name}'
        self.cache_entry('cmangos', cm_url, raw)
        cm = dict(id='cmangos-classic-db', revision=f'{name} (git blob {blob})', snapshot_sha256=retain.digest(raw), url='https://github.com/cmangos/classic-db')
        self.shard('cmangos', 'gen_world_cmangos', 'world.npcs', [cm, self.wago(retain.CMANGOS_TABLES)], 'vanilla_1_12', '1.12.1')
        self.shard('db2', 'gen_world_db2', 'world.areas', [self.wago(('AreaTable',))], 'wow_forever', BUILD)
        self.shard('fishing', 'gen_fishing', 'fishing.areas', [cm, self.wago(('AreaTable',))], 'vanilla_1_12', '1.12.1')

    def cache_entry(self, name, url, raw):
        path = self.cache / (name + '.csv')
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        meta = path.with_name(path.name + '.meta.json')
        meta.write_text(json.dumps(dict(url=url, status=200, bytes=len(raw), sha256=retain.digest(raw))))
        self.metas[url] = meta

    def wago(self, tables):
        pairs = sorted((self.urls[t], json.loads(self.metas[self.urls[t]].read_bytes())['sha256']) for t in tables)
        return dict(id='wago-tools-wowf', revision=BUILD, snapshot_sha256=retain.digest(json.dumps(pairs).encode()), url='https://wago.tools/db2?product=wowf')

    def shard(self, name, generator, dataset, sources, edition, build):
        folder = self.world / name
        folder.mkdir()
        (folder / 'part-0000.json').write_text(json.dumps(dict(generator={'name': generator}, dataset=dataset,
            records=[{'id': 'synthetic'}], sources=sources, origin={'edition': edition, 'build': {'status': 'known', 'value': build}})))

    def run(self, commit=None):
        return retain.prepare(self.root, self.world, self.cache, self.pack, self.registry, self.output, commit, self.policy)


class RetainedSources(unittest.TestCase):
    def test_complete_closure_reproduces_aggregates_and_preserves_distinct_code_terms(self):
        with tempfile.TemporaryDirectory() as directory:
            f = Fixture(directory)
            closure = f.run()
            manifest = json.loads(f.output.read_bytes())
            self.assertEqual(manifest['schema_version'], 2)
            self.assertEqual(manifest['transformation'], 'world_to_content_v7')
            self.assertEqual(manifest['source_commit'], f.commit)
            wago = next(s for s in closure if s['id'] == 'wago-tools-wowf')
            self.assertEqual(len(wago['files']), 6)
            self.assertEqual(len(wago['snapshots']), 2)
            for snapshot in wago['snapshots']:
                pairs = [(i['url'], i['sha256']) for i in snapshot['inputs']]
                self.assertEqual(retain.digest(json.dumps(sorted(pairs)).encode()), snapshot['snapshot_sha256'])
            registry = json.loads(f.registry.read_bytes())
            derived = next(s for s in registry['sources'] if s['id'] == retain.DERIVED)
            self.assertEqual({s['url'].rsplit('/', 1)[-1] for s in derived['files']}, {'world_to_content.py', 'bounded.py'})
            self.assertEqual(derived['retrieval']['max_requests'], 3)
            self.assertEqual(derived['retrieval']['max_total_bytes'], sum(v['bytes'] for v in derived['files']) + (f.root/'LICENSE').stat().st_size)
            corresponding = manifest['corresponding_source']
            self.assertIn('not relicensed as BSD', corresponding['classification'])
            paths = {item['source_path'] for item in corresponding['files']}
            self.assertIn('data-generators/navzeroth_datagen/sqldump.py', paths)
            self.assertIn(corresponding['permission_path'], paths)
            self.assertIn(corresponding['notice_path'], paths)
            original = (f.registry.read_bytes(), f.output.read_bytes())
            # An output-only commit cannot move the default source pin or any generated bytes.
            git(f.root, 'add', '.'); git(f.root, 'commit', '-qm', 'Synthetic generated outputs')
            f.run()
            self.assertEqual(original, (f.registry.read_bytes(), f.output.read_bytes()))

    def test_cache_tampering_missing_raw_and_aggregate_mismatch_fail_closed(self):
        for mutation in ('body', 'url', 'size', 'status', 'missing', 'aggregate'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                f = Fixture(directory)
                path = f.metas[f.urls['AreaTable']]
                meta = json.loads(path.read_bytes())
                if mutation == 'body': path.with_name(path.name[:-10]).write_bytes(b'changed')
                elif mutation == 'url': meta['url'] += '&extra=1'
                elif mutation == 'size': meta['bytes'] += 1
                elif mutation == 'status': meta['status'] = 404
                elif mutation == 'missing': path.with_name(path.name[:-10]).unlink()
                else:
                    shard = f.world / 'db2/part-0000.json'; doc = json.loads(shard.read_bytes())
                    doc['sources'][0]['snapshot_sha256'] = '0'*64; shard.write_text(json.dumps(doc))
                path.write_text(json.dumps(meta))
                before = f.registry.read_bytes()
                with self.assertRaises((ValueError, OSError)):
                    f.run()
                self.assertEqual(f.registry.read_bytes(), before)

    def test_selected_cache_has_aggregate_budget(self):
        with tempfile.TemporaryDirectory() as directory:
            f = Fixture(directory)
            with mock.patch.object(retain, 'MAX_RAW_TOTAL', 8), self.assertRaises(ValueError):
                retain.cache_inputs(f.cache, retain.snapshots(f.world, retain.table_mapping(f.root)))

    def test_immutable_objects_reject_corruption_and_links(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            reference = retain.retain(root, b'original')
            path = root / reference['path']
            path.write_bytes(b'corrupt')
            with self.assertRaises(ValueError): retain.retain(root, b'original')
            self.assertEqual(path.read_bytes(), b'corrupt')
            path.unlink()
            target = root / 'must-not-create'
            try: path.symlink_to(target)
            except OSError: return
            with self.assertRaises(ValueError): retain.retain(root, b'original')
            self.assertFalse(target.exists())

    def test_dirty_code_and_unknown_source_pins_cannot_claim_immutable_urls(self):
        with tempfile.TemporaryDirectory() as directory:
            f = Fixture(directory)
            mapper = f.root / retain.DERIVED_CODE[0]
            mapper.write_text('VERSION = "8"\n')
            with self.assertRaises(ValueError): f.run(f.commit)
            self.assertFalse(f.output.exists())
            with self.assertRaises(ValueError): f.run('HEAD')
            with self.assertRaises(subprocess.CalledProcessError): f.run('0'*40)

    def test_duplicate_sources_and_invalid_mapper_version_refuse(self):
        with self.assertRaises(ValueError): retain.sources_by_id([{'id':'x'}, {'id':'x'}])
        with self.assertRaises(ValueError): retain.transformation_version(b'VERSION = "unknown"')

    def test_normalization_policy_and_budgets_cannot_be_promoted_by_pack(self):
        for mutation in ('fields', 'grant', 'origin', 'revision', 'budget', 'policy-version'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                f = Fixture(directory)
                registry = json.loads(f.registry.read_bytes())
                if mutation == 'fields':
                    pack = json.loads(f.pack.read_bytes())
                    pack['sources'][0]['allowed_fields']['value'].append('unreviewed.field')
                    f.pack.write_text(json.dumps(pack))
                elif mutation == 'origin':
                    policy = json.loads(f.policy.read_bytes())
                    policy['sources'][0]['origin']['build'] = 'unreviewed'
                    f.policy.write_text(json.dumps(policy))
                elif mutation == 'policy-version':
                    policy = json.loads(f.policy.read_bytes());policy['transformation'] = 'world_to_content_v8'
                    f.policy.write_text(json.dumps(policy))
                else:
                    if mutation == 'grant': registry['sources'][0]['rights']['status'] = 'unreviewed'
                    elif mutation == 'revision': registry['sources'][0]['revision'] = 'b'*40
                    else:
                        registry['sources'][0]['retrieval']['max_total_bytes'] = 1
                        # Explicitly reviewed small budget is still a hard ceiling.
                        policy = json.loads(f.policy.read_bytes())
                        policy['sources'][0]['grant_sha256'] = retain.grant_identity(registry['sources'][0])
                        f.policy.write_text(json.dumps(policy))
                    f.registry.write_text(json.dumps(registry))
                before = f.registry.read_bytes()
                with self.assertRaises(ValueError): f.run()
                self.assertEqual(f.registry.read_bytes(), before)
