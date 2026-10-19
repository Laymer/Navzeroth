"""Synthetic input refusal tests; no network or private corpus."""
import gzip
import hashlib
import io
import json
import os
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from navzeroth_datagen import bounded, http, sqldump, to_facts
import register_facts


class IntegrityTest(unittest.TestCase):
    def test_cache_requires_exact_identity_bytes_and_metadata(self):
        with tempfile.TemporaryDirectory() as root, mock.patch.dict(os.environ, {
                'NAVZEROTH_DATAGEN_CACHE': root, 'NAVZEROTH_DATAGEN_OFFLINE': '1',
                'NAVZEROTH_DATAGEN_REFRESH': '0'}):
            url = 'https://example.org/integrity'
            body, meta = http._paths(url, 'input')
            body.parent.mkdir(parents=True)
            good = dict(url=url, status=200, bytes=2, sha256=hashlib.sha256(b'ok').hexdigest())
            for changed in [dict(good, url=url+'wrong'), dict(good, bytes=3),
                            dict(good, sha256='0'*64), {k:v for k,v in good.items() if k != 'bytes'},
                            {k:v for k,v in good.items() if k != 'sha256'}]:
                with self.subTest(meta=changed):
                    body.write_bytes(b'ok'); meta.write_text(json.dumps(changed))
                    with self.assertRaises(http.FetchError):
                        http.fetch(url, 'input')
            meta.write_text(json.dumps(good)); body.write_bytes(b'no')
            with self.assertRaises(http.FetchError):
                http.fetch(url, 'input')
            meta.unlink()
            with self.assertRaises(http.FetchError):
                http.fetch(url, 'input')
            meta.write_text(json.dumps(good)); body.write_bytes(b'ok')
            self.assertEqual(http.fetch(url, 'input'), b'ok')

    def test_stream_and_network_reads_stop_at_limit_plus_one(self):
        stream = io.BytesIO(b'x'*100)
        with self.assertRaises(ValueError):
            bounded.read_stream(stream, 8)
        self.assertEqual(stream.tell(), 9)
        response = mock.MagicMock()
        response.status = 200
        response.read1.side_effect = io.BytesIO(b'x'*100).read
        response.__enter__.return_value = response
        with mock.patch.object(http, 'MAX_BODY_BYTES', 8), mock.patch.object(http, '_host_slot'), \
                mock.patch.object(http._opener, 'open', return_value=response):
            with self.assertRaises(ValueError):
                http._download('https://example.org/a', 1)
        self.assertEqual(response.read1.call_args.args[0], 9)

    def test_oversized_file_refused_before_open_and_links_refused(self):
        with tempfile.TemporaryDirectory() as root:
            p = pathlib.Path(root)/'a'; p.write_bytes(b'123456789')
            with mock.patch.object(bounded.os, 'open', side_effect=AssertionError('must not open')):
                with self.assertRaises(ValueError):
                    bounded.read_file(p, 8)
            link = pathlib.Path(root)/'link'
            try:
                link.symlink_to(p)
            except OSError:
                return  # symlink permission is unavailable on some Windows runners
            with self.assertRaises(ValueError):
                bounded.read_file(link, 100)

    def test_cache_and_decompression_limits(self):
        with mock.patch.object(sqldump, 'MAX_SQL_BYTES', 8):
            with self.assertRaises(ValueError):
                sqldump.decompress_sql(gzip.compress(b'x'*1000))
            self.assertEqual(sqldump.decompress_sql(gzip.compress(b'exactly8')), 'exactly8')
        with tempfile.TemporaryDirectory() as root, mock.patch.dict(os.environ, {
                'NAVZEROTH_DATAGEN_CACHE': root, 'NAVZEROTH_DATAGEN_OFFLINE': '1',
                'NAVZEROTH_DATAGEN_REFRESH': '0'}):
            url='https://example.org/large'; body,meta=http._paths(url, 'large')
            body.parent.mkdir(parents=True); body.write_bytes(b'x'*9)
            meta.write_text(json.dumps(dict(url=url,status=200,bytes=9,sha256=hashlib.sha256(b'x'*9).hexdigest())))
            with mock.patch.object(http, 'MAX_BODY_BYTES', 8):
                with self.assertRaises(ValueError):
                    http.fetch(url, 'large')

    def test_registration_and_projection_refuse_oversized_shards(self):
        with tempfile.TemporaryDirectory() as root:
            p=pathlib.Path(root)/'gen_test/x/build/part-0000.json'
            p.parent.mkdir(parents=True); p.write_bytes(b'x'*9)
            with mock.patch.object(register_facts, 'MAX_FILE_BYTES', 8):
                with self.assertRaises(register_facts.RegisterError):
                    register_facts.scan(root)
            with mock.patch.object(bounded, 'MAX_SHARD_BYTES', 8):
                with self.assertRaises(ValueError):
                    to_facts.convert(root, pathlib.Path(root)/'out')

    def test_trickled_response_has_total_deadline_and_bounded_socket_timeout(self):
        response = mock.MagicMock()
        response.url = 'https://example.org/trickle'
        response.read1.return_value = b'x'
        with mock.patch.object(http.time, 'monotonic', side_effect=[0, 1, 2, 6]):
            with self.assertRaisesRegex(http.FetchError, 'deadline'):
                http._response_bytes(response, 5)
        self.assertEqual(response.read1.call_count, 2)
        response.__enter__.return_value = response
        response.status = 200
        response.read1.return_value = b''
        with mock.patch.object(http, '_host_slot'), mock.patch.object(http._opener, 'open', return_value=response) as opened:
            http._download(response.url, 300)
        self.assertEqual(opened.call_args.kwargs['timeout'], http.MAX_SOCKET_SECONDS)

    def test_directory_and_aggregate_budgets(self):
        with tempfile.TemporaryDirectory() as root:
            p = pathlib.Path(root)/'a.json'; p.write_bytes(b'1234')
            (pathlib.Path(root)/'b.json').write_bytes(b'1234')
            with mock.patch.object(bounded, 'MAX_INPUT_FILES', 1):
                with self.assertRaisesRegex(ValueError, 'file count'):
                    bounded.input_files(root, '*.json')
            with mock.patch.object(bounded, 'MAX_INPUT_ENTRIES', 1):
                with self.assertRaisesRegex(ValueError, 'directory scan'):
                    bounded.input_files(root, '*.json')
            with mock.patch.object(bounded, 'MAX_INPUT_BYTES', 7):
                budget = bounded.InputBudget()
                self.assertEqual(budget.read(p, 8), b'1234')
                with self.assertRaises(ValueError):
                    budget.read(p, 8)
            with mock.patch.object(bounded, 'MAX_INPUT_RECORDS', 2):
                budget = bounded.InputBudget(); budget.add_records(2)
                with self.assertRaisesRegex(ValueError, 'record count'):
                    budget.add_records(1)
