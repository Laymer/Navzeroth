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
