"""Copy explicitly reviewed Git blobs as inert data; Rust readiness remains authority."""
import argparse
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import threading
import time

PREFIX = 'release/evidence/1.0.0/'
MAX_FILE = 64 * 1024 * 1024
MAX_TOTAL = 256 * 1024 * 1024
MAX_FILES = 1024
MAX_TREE = 2 * 1024 * 1024
DEADLINE = None
RESERVED = re.compile(r'^(con|prn|aux|nul|com[0-9]|lpt[0-9])(?:\.|$)', re.I)


def command(root, args, limit=MAX_TREE, timeout=60):
    """Bound both pipes while reading; no shell, Git filters or checkout."""
    if DEADLINE is not None:
        timeout = min(timeout, DEADLINE - time.monotonic())
        if timeout <= 0:
            raise ValueError('Evidence transport exceeded its overall deadline')
    process = subprocess.Popen(['git', '-C', str(root), *args], stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, stdin=subprocess.DEVNULL)
    outputs = [bytearray(), bytearray()]
    errors = []

    def drain(pipe, slot, ceiling):
        try:
            while True:
                block = pipe.read(8192)
                if not block:
                    break
                if len(outputs[slot]) + len(block) > ceiling:
                    errors.append('Git output exceeds transport bounds')
                    process.kill()
                    break
                outputs[slot].extend(block)
        except OSError:
            errors.append('Git output read failed')
        finally:
            pipe.close()

    threads = [threading.Thread(target=drain, args=(process.stdout, 0, limit), daemon=True),
               threading.Thread(target=drain, args=(process.stderr, 1, 65536), daemon=True)]
    for thread in threads:
        thread.start()
    try:
        process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()
        errors.append('Git command exceeded deadline')
    finally:
        for thread in threads:
            thread.join(timeout=1)
            if thread.is_alive():
                errors.append('Git pipe remained open after command deadline')
    if errors or process.returncode:
        raise ValueError(errors[0] if errors else 'Git transport command failed')
    return bytes(outputs[0])


def identity(value):
    if not re.fullmatch('[0-9a-f]{40}', value):
        raise ValueError('Explicit lowercase full commit required')
    return value


def validate_inputs(commit, version, phase, evidence_run):
    if commit:
        identity(commit)
        if version != '1.0.0' or phase != 'public' or evidence_run:
            raise ValueError('Reviewed commit is public stable1.0.0 only and excludes evidence_run_id')


def safe_relative(raw):
    try:
        value = raw.decode('ascii')
    except UnicodeDecodeError as error:
        raise ValueError('Evidence paths must be portable ASCII') from error
    parts = value.split('/')
    if (len(value) > 220 or len(parts) > 8 or parts[0] not in ('acceptance-evidence', 'draft-evidence')
            or len(parts) < 2 or any(not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,127}', part)
                                     or part.endswith('.') or RESERVED.match(part) for part in parts)):
        raise ValueError('Unsafe evidence path')
    return value


def assert_clean(root, expected):
    if command(root, ['rev-parse', 'HEAD'], 128).decode().strip() != identity(expected):
        raise ValueError('Tag checkout changed')
    if command(root, ['status', '--porcelain', '--untracked-files=no']):
        raise ValueError('Tracked source must remain clean')
    return command(root, ['rev-parse', 'HEAD^{tree}'], 128)


def inventory(root, commit):
    # -t includes tree nodes, so links/submodules cannot hide behind a directory.
    raw = command(root, ['ls-tree', '-r', '-t', '-z', '--full-tree', identity(commit), '--', PREFIX])
    files, names, total = [], set(), 0
    for row in raw.split(b'\0'):
        if not row:
            continue
        metadata, path = row.split(b'\t', 1)
        mode, kind, oid = metadata.decode('ascii').split(' ')
        if path in (b'release', b'release/evidence', PREFIX.rstrip('/').encode()):
            if kind != 'tree' or mode != '040000':
                raise ValueError('Evidence ancestor is not a tree')
            continue
        if not path.startswith(PREFIX.encode()):
            raise ValueError('Unexpected evidence tree path')
        relative = path[len(PREFIX):]
        if not relative:
            if kind != 'tree' or mode != '040000':
                raise ValueError('Evidence root is not a tree')
            continue
        name = safe_relative(relative) if b'/' in relative else relative.decode('ascii')
        if b'/' not in relative and name not in ('acceptance-evidence', 'draft-evidence'):
            raise ValueError('Unexpected evidence subtree')
        if name.casefold() in names:
            raise ValueError('Case-colliding evidence paths')
        names.add(name.casefold())
        if kind == 'tree' and mode == '040000':
            continue
        if kind != 'blob' or mode != '100644' or not re.fullmatch('[0-9a-f]{40}', oid):
            raise ValueError('Evidence must contain regular non-executable blobs only')
        size = int(command(root, ['cat-file', '-s', oid], 64))
        if size < 1 or size > MAX_FILE or len(files) >= MAX_FILES or total + size > MAX_TOTAL:
            raise ValueError('Evidence file/count/aggregate exceeds transport bounds')
        total += size
        files.append((name, oid, size))
    names = {name for name, _, _ in files}
    if 'acceptance-evidence/acceptance-evidence.json' not in names or not any(n.startswith('draft-evidence/') for n in names):
        raise ValueError('Both exact evidence subtrees and overlay are required')
    # File-vs-directory case collisions are already caught by -t; refuse prefixes too.
    return files


def import_blobs(root, commit, expected):
    root = Path(root).resolve()
    before = assert_clean(root, expected)
    files = inventory(root, commit)
    destination = root / 'dist'
    if destination.is_symlink() or (destination.exists() and not destination.is_dir()):
        raise ValueError('Unsafe dist directory')
    for name in ('acceptance-evidence', 'draft-evidence', 'public-download-evidence'):
        if os.path.lexists(destination / name):
            raise ValueError('Evidence destinations must not exist')
    destination.mkdir(exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix='.reviewed-evidence-', dir=destination))
    moved = []
    try:
        for name, oid, size in files:
            data = command(root, ['cat-file', 'blob', oid], min(MAX_FILE, size))
            if len(data) != size or data.startswith(b'version https://git-lfs.github.com/spec/v1'):
                raise ValueError('Missing bytes or LFS pointer in evidence')
            path = stage / name
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('xb') as output:
                output.write(data)
        if assert_clean(root, expected) != before:
            raise ValueError('Source tree changed during evidence import')
        for name in ('acceptance-evidence', 'draft-evidence'):
            if os.path.lexists(destination / name):
                raise ValueError('Evidence destination appeared during import')
            (stage / name).rename(destination / name)
            moved.append(destination / name)
        if assert_clean(root, expected) != before:
            raise ValueError('Source changed after evidence import')
    except Exception:
        for path in moved:
            shutil.rmtree(path)
        raise
    finally:
        shutil.rmtree(stage)
    return len(files)


def main():
    global DEADLINE
    DEADLINE = time.monotonic() + 600
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', default='.')
    parser.add_argument('--commit', default='')
    parser.add_argument('--expected-source', required=True)
    parser.add_argument('--repository', required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--phase', required=True)
    parser.add_argument('--evidence-run', default='')
    args = parser.parse_args()
    validate_inputs(args.commit, args.version, args.phase, args.evidence_run)
    identity(args.expected_source)
    if not args.commit:
        return
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', args.repository):
        raise ValueError('Invalid repository identity')
    root = Path(args.root).resolve()
    assert_clean(root, args.expected_source)
    origin = command(root, ['remote', 'get-url', 'origin'], 2048).decode().strip()
    if origin not in (f'https://github.com/{args.repository}', f'https://github.com/{args.repository}.git'):
        raise ValueError('Evidence must come from the same GitHub repository origin')
    command(root, ['-c', 'protocol.allow=never', '-c', 'protocol.https.allow=always',
                   'fetch', '--no-tags', '--no-recurse-submodules', 'origin', args.commit], timeout=180)
    if command(root, ['rev-parse', '--verify', args.commit + '^{commit}'], 128).decode().strip() != args.commit:
        raise ValueError('Evidence object is not the explicit commit')
    count = import_blobs(root, args.commit, args.expected_source)
    print(f'Imported {count} inert reviewed-evidence files; acceptance remains subject to full Rust readiness.')


if __name__ == '__main__':
    main()
