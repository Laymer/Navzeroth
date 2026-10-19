"""Synthetic weekly orchestration and pointer tests; never use the network."""
import contextlib
import hashlib
import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]))
import pipeline
import run_all
import register_facts
from navzeroth_datagen import bounded, db2, envelope


class WeeklyClosure(unittest.TestCase):
    def test_exact_pointer_restoration_refuses_wrong_or_linked_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            data=b'independently acquired exact original'
            sha=hashlib.sha256(data).hexdigest()
            path=pathlib.Path(tmp)/sha
            pointer=f'version https://git-lfs.github.com/spec/v1\noid sha256:{sha}\nsize {len(data)}\n'.encode()
            for wrong in [b'unrelated',pointer.replace(sha.encode(),b'0'*64),pointer.replace(f'size {len(data)}'.encode(),b'size 1'),pointer+b'\n']:
                path.write_bytes(wrong)
                with self.assertRaises(ValueError):bounded.retain_object(path,data,1000)
                self.assertEqual(path.read_bytes(),wrong)
            path.write_bytes(pointer)
            bounded.retain_object(path,data,1000)
            self.assertEqual(path.read_bytes(),data)
            bounded.retain_object(path,data,1000)
            path.write_bytes(pointer)
            register_facts.store(path.parent,sha,data)
            self.assertEqual(path.read_bytes(),data)
            path.unlink();path.symlink_to(path.parent/'outside')
            with self.assertRaises((ValueError,OSError)):bounded.retain_object(path,data,1000)
            path.unlink()
            with self.assertRaises(ValueError):bounded.retain_object(path,b'wrong',1000)

    def test_wago_policy_pin_is_explicit_and_cannot_conflict(self):
        with mock.patch.dict(os.environ,{'NAVZEROTH_DATAGEN_WAGO_WOWF_BUILD':'1.60.1.70245'}), mock.patch.object(db2.http,'fetch_json',side_effect=AssertionError('no build-list request')):
            self.assertEqual(db2.Wago().build,'1.60.1.70245')
            with self.assertRaises(ValueError):db2.Wago(build='1.60.1.70450')
        with mock.patch.dict(os.environ,{'NAVZEROTH_DATAGEN_WAGO_WOWF_BUILD':'bad'}):
            with self.assertRaises(ValueError):db2.Wago()
        args=pipeline.parse_args(['--weekly-verified','--no-network-sources','--skip','detect'])
        self.assertLess(args.stages.index('content'),args.stages.index('register'))

    def test_weekly_mode_refuses_required_stage_skips_and_subset_runs(self):
        base=['--weekly-verified','--no-network-sources']
        for skipped in ['generate','verify','content','register','gate','generate,verify,content,register,gate']:
            with self.subTest(skipped=skipped),self.assertRaises(SystemExit),contextlib.redirect_stderr(io.StringIO()):
                pipeline.parse_args(base+['--skip',skipped])
        for options in [['--only','publish'],['--only','generate,verify,content,register,gate'],['--offline'],['--refresh'],['--pages-dir','synthetic']]:
            with self.subTest(options=options),self.assertRaises(SystemExit),contextlib.redirect_stderr(io.StringIO()):
                pipeline.parse_args(base+options)
        for options in [['--skip','detect'],['--skip','publish'],['--no-publish'],['--skip','detect,publish']]:
            args=pipeline.parse_args(base+options)
            self.assertTrue({'generate','verify','content','register','gate'}.issubset(args.stages))

    def test_generation_overrides_inherited_build_with_exact_reviewed_policy(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp)
            policy=root/'release/v1/source-normalization-policy.json';policy.parent.mkdir(parents=True)
            policy.write_text(json.dumps({'sources':[{'id':'wago-tools-wowf','origin':{'edition':'wow_forever','build':'1.60.1.70245'}}]}))
            calls=[]
            def runner(command,**kwargs):
                calls.append((command,kwargs['env']))
                return subprocess.CompletedProcess(command,0,'','')
            args=pipeline.parse_args(['--weekly-verified','--no-network-sources','--skip','detect'])
            ctx=pipeline.Context(args,root=root,runner=runner,probes=[])
            with mock.patch.dict(os.environ,{'NAVZEROTH_DATAGEN_WAGO_WOWF_BUILD':'1.60.1.99999'}),contextlib.redirect_stdout(io.StringIO()):pipeline.stage_generate(ctx)
            self.assertEqual(calls[0][1]['NAVZEROTH_DATAGEN_WAGO_WOWF_BUILD'],'1.60.1.70245')
            self.assertIn('--clean-declared',calls[0][0])

    def test_declared_cleanup_preserves_unrelated_and_refuses_unsafe_or_excluded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp)
            relative='world/areas/wow_forever/old/part-0000.json'
            old=root/relative;old.parent.mkdir(parents=True);old.write_bytes(b'pointer')
            unrelated=old.with_name('part-9999.json');unrelated.write_bytes(b'unlisted')
            previous=({'selected':{'files':[{'path':relative,'bytes':7,'sha256':hashlib.sha256(b'pointer').hexdigest()}]}},{})
            with contextlib.redirect_stdout(io.StringIO()):run_all.clean_declared(root,['selected'],previous)
            self.assertFalse(old.exists());self.assertEqual(unrelated.read_bytes(),b'unlisted')
            previous[0]['selected']['files'][0]['path']='../outside/part-0000.json'
            with self.assertRaises(ValueError):run_all.clean_declared(root,['selected'],previous)
            with self.assertRaises(ValueError):run_all.clean_declared(root,[],({'excluded':{'files':[{'path':relative}]}},{}))
            with self.assertRaises(ValueError):run_all.clean_declared(root,['selected'],None)

    def test_weekly_envelope_never_overwrites_unlisted_parts(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(os.environ,{'NAVZEROTH_DATAGEN_DECLARED_CLEAN':'1'}):
            options=dict(origin=envelope.origin(envelope.FOREVER,'1.60.1.70245'),sources=[],generator=envelope.generator('synthetic'),out=pathlib.Path(tmp))
            files=envelope.write('world.areas','area',[{'id':1}],**options)
            original=files[0].read_bytes()
            with self.assertRaises(FileExistsError):envelope.write('world.areas','area',[{'id':2}],**options)
            self.assertEqual(files[0].read_bytes(),original)

    def test_manifest_hydration_selects_only_exact_bounded_metadata(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp)
            path=root/'data-generators/output/run-manifest.json';path.parent.mkdir(parents=True)
            raw=json.dumps({'format':'navzeroth-world-data-run','generators':[]}).encode()
            sha=hashlib.sha256(raw).hexdigest()
            pointer=f'version https://git-lfs.github.com/spec/v1\noid sha256:{sha}\nsize {len(raw)}\n'
            commands=[]
            def runner(command,**kwargs):
                commands.append(command)
                output=str(len(pointer)) if command[1]=='cat-file' else pointer if command[1]=='show' else ''
                if command[1:3]==['lfs','pull']:path.write_bytes(raw)
                return subprocess.CompletedProcess(command,0,output,'')
            ctx=pipeline.Context(pipeline.parse_args([]),runner=runner,root=root,probes=[])
            with contextlib.redirect_stdout(io.StringIO()):pipeline.hydrate_weekly_manifest(ctx)
            self.assertEqual(commands[-1],['git','lfs','pull','--include=data-generators/output/run-manifest.json','--exclude='])
            self.assertEqual(path.read_bytes(),raw)
            pointer=pointer.replace(sha,'0'*64)
            with self.assertRaises(pipeline.StageError),contextlib.redirect_stdout(io.StringIO()):pipeline.hydrate_weekly_manifest(ctx)

    def test_exact_offline_replay_is_required_before_one_publication(self):
        for alter in [False,True,"product","outside-report","historical-report"]:
            with self.subTest(alter=alter),tempfile.TemporaryDirectory() as tmp:
                root=pathlib.Path(tmp)
                args=pipeline.parse_args(['--weekly-verified','--no-network-sources','--skip','detect'])
                ctx=pipeline.Context(args,root=root,probes=[])
                ctx.logs=root/'logs'
                target=root/'release/v1/world-data-target.json';target.parent.mkdir(parents=True)
                target.write_text(json.dumps({'build':'1.2.3.4'}))
                calls=[]
                def stage(name):
                    def execute(ctx):
                        calls.append((name,ctx.args.offline))
                        if name=='generate':
                            file=root/'content/world/1.2.3.4/catalog.json';file.parent.mkdir(parents=True,exist_ok=True)
                            file.write_bytes(b'changed' if alter is True and ctx.args.offline else b'exact')
                            product=root/'dist/source-knowledge-world/catalog.json';product.parent.mkdir(parents=True,exist_ok=True)
                            product.write_bytes(b'changed' if alter=='product' and ctx.args.offline else b'exact catalog')
                            file.with_name('release-readiness.json').write_text(str(len(calls)))
                            if alter in ('outside-report','historical-report'):
                                other=root/('content/source-materials/release-readiness.json' if alter=='outside-report' else 'content/world/1.2.3.5/release-readiness.json')
                                other.parent.mkdir(parents=True,exist_ok=True);other.write_text(str(len(calls)))
                        return 'ok','synthetic'
                    return execute
                with mock.patch.dict(pipeline.STAGE_FUNCS,{name:stage(name) for name in pipeline.STAGES}),contextlib.redirect_stdout(io.StringIO()):
                    results=pipeline.verified_weekly(ctx)
                self.assertEqual(sum(name=='publish' for name,_ in calls),0 if alter else 1)
                self.assertEqual(any(row[1]=='failed' for row in results),bool(alter))
                if not alter:
                    receipt=json.loads((root/'logs/weekly-reproducibility.json').read_text())
                    self.assertTrue(receipt['verified'])
                    self.assertEqual(len(receipt['files']),2)
                    self.assertIn('content/world/1.2.3.4/release-readiness.json',receipt['excluded_nonmaterial'])
                    self.assertEqual(len(receipt['unpublished_build_products']),1)
                self.assertEqual([name for name,offline in calls if not offline][:4],['generate','verify','content','register'])


if __name__=='__main__':unittest.main()
