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
