"""Finite byte reads for inert generator inputs; never follow final symlinks."""
import os
import stat

MAX_SHARD_BYTES = 8 * 1024 * 1024


def read_stream(stream, limit):
    data = bytearray()
    while len(data) <= limit:
        chunk = stream.read(min(1024 * 1024, limit + 1 - len(data)))
        if not chunk:
            return bytes(data)
        data.extend(chunk)
    raise ValueError("input exceeds byte limit")


def read_file(path, limit):
    before = os.lstat(path)
    if not stat.S_ISREG(before.st_mode) or before.st_size > limit:
        raise ValueError("input is not a bounded regular file")
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0))
    with os.fdopen(fd, "rb") as stream:
        opened = os.fstat(stream.fileno())
        if (not stat.S_ISREG(opened.st_mode)
                or (before.st_dev, before.st_ino, before.st_size) !=
                   (opened.st_dev, opened.st_ino, opened.st_size)):
            raise ValueError("input changed before read")
        data = read_stream(stream, min(limit, opened.st_size))
        if len(data) != opened.st_size:
            raise ValueError("input changed during read")
        return data

# Current corpus: 119 shards including facts, approximately 0.7 GiB facts.
MAX_INPUT_FILES = 4096
MAX_INPUT_BYTES = 2 * 1024 * 1024 * 1024
MAX_INPUT_ENTRIES = 32768
MAX_INPUT_RECORDS = 6_000_000


def input_files(root, pattern):
    """Deterministic finite scan; refuse linked directories rather than traversing them."""
    import fnmatch
    from pathlib import Path
    root = Path(root)
    files = []
    entries = 0
    pending = [(root, 0)]
    while pending:
        parent, depth = pending.pop()
        if depth > 16:
            raise ValueError("input directory scan exceeds limit")
        with os.scandir(parent) as children:
            for child in children:
                entries += 1
                if entries > MAX_INPUT_ENTRIES:
                    raise ValueError("input directory scan exceeds limit")
                if child.is_symlink():
                    raise ValueError("linked input entry refused")
                if child.is_dir(follow_symlinks=False):
                    pending.append((Path(child.path), depth + 1))
                elif fnmatch.fnmatchcase(child.name, pattern):
                    files.append(Path(child.path))
                    if len(files) > MAX_INPUT_FILES:
                        raise ValueError("input file count exceeds limit")
    return sorted(files)


class InputBudget:
    def __init__(self):
        self.bytes = 0
        self.records = 0

    def read(self, path, limit):
        data = read_file(path, min(limit, MAX_INPUT_BYTES - self.bytes))
        self.bytes += len(data)
        return data

    def add_records(self, count):
        self.records += count
        if self.records > MAX_INPUT_RECORDS:
            raise ValueError("input record count exceeds limit")


def json_document(raw, max_depth=64):
    """Parse bounded inert JSON; refuse duplicate members and excessive nesting."""
    import json
    text = raw.decode("utf-8", "strict")
    depth = 0
    quoted = False
    escaped = False
    for byte in raw:
        if quoted:
            if escaped:
                escaped = False
            elif byte == 92:
                escaped = True
            elif byte == 34:
                quoted = False
        elif byte == 34:
            quoted = True
        elif byte in (91, 123):
            depth += 1
            if depth > max_depth:
                raise ValueError("JSON nesting exceeds limit")
        elif byte in (93, 125):
            depth -= 1
    def number(value):
        import math
        result = float(value)
        if not math.isfinite(result):
            raise ValueError("non-finite JSON number")
        return result
    def members(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate JSON member")
            result[key] = value
        return result
    return json.loads(text, object_pairs_hook=members, parse_float=number,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError("non-finite JSON number")))


def retain_object(path, data, limit):
    """Restore only an exact canonical Git LFS pointer from freshly hashed bytes."""
    import hashlib
    import pathlib
    import tempfile
    path = pathlib.Path(path)
    digest = hashlib.sha256(data).hexdigest()
    if not data or len(data) > limit or path.name != digest:
        raise ValueError('retained object bytes differ from content address')
    for parent in path.parents:
        info = parent.lstat()
        if not stat.S_ISDIR(info.st_mode) or stat.S_ISLNK(info.st_mode):
            raise ValueError('linked retained-object parent refused')
    pointer = (f'version https://git-lfs.github.com/spec/v1\noid sha256:{digest}\nsize {len(data)}\n').encode()
    before = None
    try:
        before = path.lstat()
        existing = read_file(path, limit)
        if existing == data:
            return
        if existing != pointer:
            raise ValueError('existing immutable object differs from exact bytes/pointer')
    except FileNotFoundError:
        pass
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix='.retained-', delete=False) as output:
            temporary = pathlib.Path(output.name)
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        if before is None:
            # Hard-link creates a new name without replacing a concurrent object.
            os.link(temporary, path)
        else:
            current = path.lstat()
            stamp = lambda value: (value.st_dev,value.st_ino,value.st_size,value.st_mtime_ns,value.st_ctime_ns)
            if stamp(current) != stamp(before) or read_file(path, 1024) != pointer:
                raise ValueError('retained pointer changed before restoration')
            os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
