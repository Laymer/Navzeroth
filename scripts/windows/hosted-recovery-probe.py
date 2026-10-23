#!/usr/bin/env python3
"""Disposable hosted Windows engineering only; never emits acceptance evidence."""
import argparse
import hashlib
import json
import os
import platform
import shutil
from pathlib import Path, PurePosixPath
import stat
import subprocess
import threading
import time
import zipfile
import sys
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parent))
from recovery_probe_process import OwnedTree

MAX_ZIP = 64 * 1024 * 1024
MAX_INSTALLER = 128 * 1024 * 1024
MAX_LOG = 8 * 1024 * 1024
SPEC = 'config/hosted-recovery-probe-fixture-v1.json'


def strict_json(raw):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('duplicate JSON field')
            result[key] = value
        return result
    return json.loads(raw.decode('utf-8'), object_pairs_hook=unique)


def regular(path):
    value = path.lstat()
    if not stat.S_ISREG(value.st_mode) or path.is_symlink():
        raise ValueError('linked or nonregular probe input')
    return value.st_size


def read_limited(path, maximum):
    if regular(path) > maximum:
        raise ValueError('probe input exceeds finite role limit')
    with path.open('rb') as stream:
        raw=stream.read(maximum+1)
    if len(raw) > maximum:
        raise ValueError('probe input grew beyond its role limit')
    return raw


def digest(path, maximum):
    size = regular(path)
    if not 0 < size <= maximum:
        raise ValueError('probe input exceeds finite role limit')
    result = hashlib.sha256()
    with path.open('rb') as stream:
        opened = os.fstat(stream.fileno())
        if opened.st_size != size:
            raise ValueError('probe input changed before read')
        count = 0
        for block in iter(lambda: stream.read(65536), b''):
            count += len(block)
            if count > size:
                raise ValueError('probe input grew during read')
            result.update(block)
        if count != size:
            raise ValueError('probe input changed during read')
    return result.hexdigest()


def run(command, output, name, seconds, env=None, maximum=MAX_LOG, cwd=None):
    """Bound whole process-tree lifetime and each retained byte stream."""
    stdout = output / (name + '.stdout')
    stderr = output / (name + '.stderr')
    start = time.monotonic()
    failures = []
    owner=OwnedTree(command,env=env,cwd=cwd)
    process=owner.process

    def capture(stream, path):
        try:
            with path.open('xb') as destination:
                count = 0
                while block := stream.read(65536):
                    remaining = maximum - count
                    destination.write(block[:max(0, remaining)])
                    count += len(block)
                    if count > maximum:
                        failures.append('process output exceeded finite bound')
                        return
        except (OSError, ValueError) as error:
            failures.append(type(error).__name__)
        finally:
            stream.close()

    threads = [threading.Thread(target=capture, args=(stream, path), daemon=True)
               for stream, path in [(process.stdout, stdout), (process.stderr, stderr)]]
    for thread in threads:
        thread.start()
    timed_out = False
    try:
        while True:
            parent_done=process.poll() is not None
            capture_done=all(not thread.is_alive() for thread in threads)
            if parent_done and capture_done and not owner.active():
                break
            if failures or time.monotonic() - start >= seconds:
                timed_out = not failures
                owner.kill()
                break
            time.sleep(0.05)
        process.wait(timeout=2)
    finally:
        try:
            if process.poll() is None or owner.active():
                owner.kill()
        finally:
            owner.close()
        cleanup_deadline=time.monotonic()+2
        for thread in threads:
            thread.join(timeout=max(0,cleanup_deadline-time.monotonic()))
        if any(thread.is_alive() for thread in threads):
            failures.append('process output capture did not finish')
    result = dict(command=command, exit_code=process.returncode, timed_out=timed_out,
                  elapsed_seconds=time.monotonic()-start, capture_errors=failures)
    (output / (name + '.json')).write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(probe_phase=name, **result)), flush=True)
    if timed_out or failures or process.returncode != 0:
        raise ValueError('bounded probe process failed: ' + name)
    return stdout


def extract_installer(archive, output, spec):
    if regular(archive) != spec['artifact_bytes'] or digest(archive, MAX_ZIP) != spec['artifact_sha256']:
        raise ValueError('historical artifact ZIP differs')
    with zipfile.ZipFile(archive) as source:
        members = source.infolist()
        if not 1 <= len(members) <= 128:
            raise ValueError('artifact member count exceeds limit')
        names = set()
        for entry in members:
            name = PurePosixPath(entry.filename)
            if (entry.orig_filename != entry.filename or name.is_absolute() or '..' in name.parts or '\\' in entry.filename
                    or ':' in entry.filename or entry.filename in names
                    or stat.S_IFMT(entry.external_attr >> 16) == stat.S_IFLNK):
                raise ValueError('unsafe or duplicate artifact member')
            names.add(entry.filename)
        installers = [entry for entry in members if entry.filename == spec['installer_name']]
        commits = [entry for entry in members if entry.filename == 'SOURCE_COMMIT.txt']
        if len(installers) != 1 or len(commits) != 1 or commits[0].file_size > 256:
            raise ValueError('artifact lacks unique installer/source identity')
        with source.open(commits[0]) as stream:
            if stream.read(257).decode('ascii').strip() != spec['installer_source_commit']:
                raise ValueError('artifact source is not the recorded PR merge checkout')
        entry = installers[0]
        if not 0 < entry.file_size <= MAX_INSTALLER or entry.file_size != spec['installer_bytes']:
            raise ValueError('historical installer exceeds role limit')
        installer = output / spec['installer_name']
        with source.open(entry) as stream, installer.open('xb') as destination:
            total = 0
            while block := stream.read(65536):
                total += len(block)
                if total > MAX_INSTALLER:
                    raise ValueError('installer decompression exceeds limit')
                destination.write(block)
        if digest(installer, MAX_INSTALLER) != spec['installer_sha256']:
            raise ValueError('historical installer bytes differ')
        return installer


def artifact_metadata(value, spec):
    run = value.get('workflow_run', {})
    if (value.get('id') != spec['artifact_id'] or value.get('name') != spec['artifact_name']
            or value.get('size_in_bytes') != spec['artifact_bytes'] or value.get('expired') is not False
            or value.get('digest') != 'sha256:' + spec['artifact_sha256']
            or run.get('id') != spec['artifact_run_id'] or run.get('head_sha') != spec['api_head_commit']):
        raise ValueError('historical artifact API identity differs')


def owns_fixture(marker, nonce, source, installer_hash, install_directory):
    if not marker.exists():
        return False
    try:
        if not 0 < regular(marker) <= 4096:
            return False
        record = strict_json(read_limited(marker,4096))
        expected = dict(schema_version=1, ownership_nonce=nonce, source_commit=source,
                        installer_sha256=installer_hash, install_directory=str(install_directory))
        return (type(record.get('schema_version')) is int
                and set(record) == set(expected)
                and isinstance(record.get('install_directory'),str)
                and all(record[k] == v for k, v in expected.items() if k != 'install_directory')
                and os.path.normcase(record['install_directory']) == os.path.normcase(str(install_directory)))
    except (OSError, ValueError, TypeError, AttributeError):
        return False


def guarded_source(root, output):
    if (os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true'
            or os.environ.get('RUNNER_ENVIRONMENT') != 'github-hosted'):
        raise ValueError('probe requires disposable GitHub-hosted Windows')
    source = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], timeout=30).decode().strip()
    if source != os.environ.get('GITHUB_SHA') or len(source) != 40:
        raise ValueError('probe checkout differs from the actual workflow source')
    if subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], timeout=30):
        raise ValueError('probe source must be clean and committed')
    output.resolve().relative_to(Path(os.environ['RUNNER_TEMP']).resolve())
    if output.exists():
        raise ValueError('probe output must be new')
    if any(os.environ.get(name) for name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'LLVM_PROFILE_FILE')):
        raise ValueError('probe cannot inherit instrumentation/compiler overrides')
    return source


def main(root, output):
    source = guarded_source(root, output)
    output.mkdir()
    if regular(root / SPEC) > 16384:
        raise ValueError('fixture policy exceeds limit')
    spec = strict_json(read_limited(root / SPEC,16384))
    if spec['schema_version'] != 1 or spec['repository'] != os.environ.get('GITHUB_REPOSITORY'):
        raise ValueError('fixture policy repository/version differs')
    if len(spec['metadata_files']) != 3 or {f['git_path'] for f in spec['metadata_files']} != {'release/v1/bundled-content-build-identity.json', 'content/bundled/signers-v1.json', 'crates/app/tests/support/stable_engineering_snapshot.json'}:
        raise ValueError('historical metadata inventory differs')
    summary = dict(kind='hosted_recovery_engineering_not_acceptance', source_commit=source,
                   fixture=spec, passed=False, acceptance_observations_promoted=False,
                   platform=platform.platform(), python_version=platform.python_version(),
                   logical_processors=os.cpu_count(), disk_free_bytes=shutil.disk_usage(output).free,
                   peak_memory_bytes=None, memory_scope='Not measured by this probe; no ordinary-PC claim.')
    ownership = output/'fixture-ownership.json'
    nonce = uuid.uuid4().hex
    install_directory = Path(os.environ['LOCALAPPDATA'])/'Azeroth Navigator'
    primary = None
    cleanup = None
    try:
        metadata = run(['gh', 'api', f"repos/{spec['repository']}/actions/artifacts/{spec['artifact_id']}"],
                       output, 'artifact-api', 60, maximum=1024*1024)
        artifact_metadata(strict_json(read_limited(metadata,1024*1024)), spec)
        commit_api = run(['gh', 'api', f"repos/{spec['repository']}/git/commits/{spec['installer_source_commit']}"],
                         output, 'fixture-commit-api', 60, maximum=1024*1024)
        commit = strict_json(read_limited(commit_api,1024*1024))
        if commit['sha'] != spec['installer_source_commit'] or commit['tree']['sha'] != spec['fixture_tree']:
            raise ValueError('historical installer source tree differs')
        tree = subprocess.check_output(['git', '-C', str(root), 'rev-parse', spec['fixture_metadata_commit']+'^{tree}'], timeout=30).decode().strip()
        if tree != spec['fixture_tree']:
            raise ValueError('historical metadata tree differs from installer source')
        archive = run(['gh', 'api', f"repos/{spec['repository']}/actions/artifacts/{spec['artifact_id']}/zip"],
                      output, 'artifact-zip', 180, maximum=MAX_ZIP)
        installer = extract_installer(archive, output, spec)
        fixture = output / 'historical-source'
        for index, reference in enumerate(spec['metadata_files']):
            if reference['git_path'] not in ('release/v1/bundled-content-build-identity.json', 'content/bundled/signers-v1.json', 'crates/app/tests/support/stable_engineering_snapshot.json'):
                raise ValueError('undeclared historical metadata path')
            raw = run(['git', '-C', str(root), 'show', spec['fixture_metadata_commit'] + ':' + reference['git_path']],
                      output, f'fixture-metadata-{index}', 30, maximum=1024*1024)
            if regular(raw) != reference['bytes'] or digest(raw, 1024*1024) != reference['sha256']:
                raise ValueError('historical metadata differs from reviewed fixture pin')
            path = fixture / reference['git_path']
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(read_limited(raw,1024*1024))
        identity = strict_json(read_limited(fixture/'release/v1/bundled-content-build-identity.json',65536))
        if identity['production_sha256'] != spec['fixture_production_sha256']:
            raise ValueError('fixture production identity differs')
        environment = os.environ.copy()
        environment.update(CARGO_TARGET_DIR=str(output/'build'), CARGO_INCREMENTAL='0',
                           NAVZEROTH_DIAGNOSTIC_RUNTIME_COMMIT=source,
                           NAVZEROTH_DIAGNOSTIC_HARNESS_COMMIT=source)
        run(['rustc', '-vV'], output, 'rustc-identity', 30, environment, 1024*1024)
        build = run(['cargo', 'test', '--locked', '--release', '-p', 'azeroth-app', '--test',
                     'hosted_recovery_probe', '--no-run', '--message-format=json'], output, 'build', 1200, environment, 16*1024*1024, cwd=root)
        binaries = [value['executable'] for line in build.read_bytes().splitlines()
                    if (value := strict_json(line)).get('reason') == 'compiler-artifact'
                    and value.get('target', {}).get('name') == 'hosted_recovery_probe' and value.get('executable')]
        if len(binaries) != 1:
            raise ValueError('expected one actual compiled probe executable')
        binary = Path(binaries[0])
        binary.resolve().relative_to((output/'build').resolve())
        summary['binary_sha256'] = digest(binary, 256*1024*1024)
        run(['pwsh', '-NoProfile', '-File', str(root/'scripts/windows/Recovery-Probe-Install.ps1'),
             '-Installer', str(installer), '-ExpectedSha256', spec['installer_sha256'],
             '-OwnershipMarker', str(ownership), '-OwnershipNonce', nonce], output, 'install-fixture', 360)
        if not owns_fixture(ownership, nonce, source, spec['installer_sha256'], install_directory):
            raise ValueError('fixture installation did not issue a valid ownership marker')
        resources = Path(os.environ['LOCALAPPDATA'])/'Azeroth Navigator/bundled-content'
        environment.update(NAVZEROTH_RECOVERY_SOURCE_ROOT=str(fixture), NAVZEROTH_RECOVERY_BUNDLE=str(resources),
                           NAVZEROTH_RECOVERY_OUTPUT=str(output/'workload'))
        start = time.monotonic()
        run([str(binary), '--ignored', '--exact', 'hosted_historical_fullpack_recovery', '--nocapture', '--test-threads=1'],
            output, 'whole-recovery', 1200, environment)
        summary['workload_elapsed_seconds'] = time.monotonic()-start
        events = []
        for line in (output/'whole-recovery.stderr').read_bytes().splitlines():
            try:
                value = strict_json(line)
            except (ValueError, UnicodeError):
                continue
            if isinstance(value, dict):
                events.append(value)
                if len(events) > 128:
                    raise ValueError('probe event inventory exceeds bound')
        if not any(v.get('diagnostic_passed') is True and v.get('acceptance_observations_promoted') is False for v in events):
            raise ValueError('completed probe assertions were not recorded')
        summary.update(phase_events=events)
        if subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], timeout=30):
            raise ValueError('probe changed source files')
    except BaseException as error:
        primary = error
        summary['primary_error'] = type(error).__name__ + ': ' + str(error)[:1024]
    finally:
        try:
            if owns_fixture(ownership, nonce, source, spec['installer_sha256'], install_directory):
                run(['pwsh', '-NoProfile', '-File', str(root/'scripts/windows/Recovery-Probe-Install.ps1'), '-Cleanup', '-ExpectedSha256', spec['installer_sha256'],
                     '-OwnershipMarker', str(ownership), '-OwnershipNonce', nonce], output, 'cleanup', 360)
            else:
                summary['cleanup_not_attempted'] = 'No valid marker: do not touch an unowned pre-existing application.'
        except BaseException as error:
            cleanup = error
            summary['cleanup_error'] = type(error).__name__ + ': ' + str(error)[:1024]
        summary['passed'] = primary is None and cleanup is None
        encoded = (json.dumps(summary, indent=2) + '\n').encode()
        if len(encoded) > 1024*1024:
            summary = dict(kind=summary['kind'], source_commit=source, passed=False,
                           acceptance_observations_promoted=False, error='result exceeds finite bound')
            encoded = (json.dumps(summary) + '\n').encode()
            if primary is None:
                primary = ValueError('result exceeds finite bound')
        (output/'engineering-result.json').write_bytes(encoded)
    if primary is not None:
        raise primary
    if cleanup is not None:
        raise cleanup


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    main(args.root.resolve(), args.output.resolve())
