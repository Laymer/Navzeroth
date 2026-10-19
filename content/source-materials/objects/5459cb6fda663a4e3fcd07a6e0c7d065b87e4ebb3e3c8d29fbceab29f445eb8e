"""Reads tables from a MySQL dump (the CMaNGOS classic-db Full_DB file) as data.

The SQL is never executed: CREATE TABLE gives the column names and each
INSERT ... VALUES body is tokenized here. Based on TALOD's gen_fishing.py
parse_values / table (used with permission), extended to keep NULL distinct from
the string "NULL" and to decode backslash escapes.

    dump = load_classic_db()          # Dump, cached download of the newest Full_DB dump
    for row in dump.rows("creature_template"): ...
"""
import gzip
import io

from .bounded import read_stream
import re

from . import http

CMANGOS_LIST = "https://api.github.com/repos/cmangos/classic-db/contents/Full_DB"
CMANGOS_REPO = "https://github.com/cmangos/classic-db"
ESCAPES = {"n": "\n", "r": "\r", "t": "\t", "0": "\0", "Z": "\x1a", "b": "\b"}


def parse_values(body):
    """Rows of an INSERT ... VALUES body: lists of str (quoted), int/float/None (bare)."""
    rows, i, n = [], 0, len(body)
    while i < n:
        if body[i] != "(":
            i += 1
            continue
        row, cur, quoted, i = [], "", None, i + 1
        while i < n:
            c = body[i]
            if c == "'":
                i += 1
                s = []
                while body[i] != "'" or (i + 1 < n and body[i + 1] == "'"):
                    if body[i] == "\\":
                        i += 1
                        s.append(ESCAPES.get(body[i], body[i]))
                    elif body[i] == "'":  # '' inside a string
                        s.append("'")
                        i += 1
                    else:
                        s.append(body[i])
                    i += 1
                quoted = "".join(s)
            elif c in ",)":
                row.append(quoted if quoted is not None else _bare(cur.strip()))
                cur, quoted = "", None
                if c == ")":
                    rows.append(row)
                    i += 1
                    break
            else:
                cur += c
            i += 1
    return rows


def _bare(token):
    if token.upper() == "NULL" or token == "":
        return None
    try:
        return int(token)
    except ValueError:
        try:
            return float(token)
        except ValueError:
            return token


class Dump:
    def __init__(self, sql):
        self.sql = sql
        self._columns = {}
        for m in re.finditer(r"CREATE TABLE `(\w+)` \((.*?)\n\)", sql, re.S):
            self._columns[m.group(1)] = re.findall(r"^\s*`(\w+)`", m.group(2), re.M)
        self._bodies = {}
        for m in re.finditer(r"INSERT INTO `(\w+)` (?:\(([^)]*)\) )?VALUES (.*?);\n", sql, re.S):
            # An explicit column list (subset or reordered insert) wins over CREATE TABLE order.
            cols = re.findall(r"`(\w+)`", m.group(2)) if m.group(2) else None
            self._bodies.setdefault(m.group(1), []).append((cols, m.group(3)))

    def tables(self):
        return sorted(self._columns)

    def columns(self, table):
        if table not in self._columns:
            raise KeyError(f"table {table} not in the dump")
        return self._columns[table]

    def rows(self, table):
        """Rows as dicts keyed by column name; an empty table gives []."""
        cols = self.columns(table)
        out = []
        for insert_cols, body in self._bodies.get(table, ()):
            names = insert_cols or cols
            out.extend(dict(zip(names, r)) for r in parse_values(body))
        return out


# The retained SQL expands to 73,289,738 bytes; finite growth allowance.
MAX_SQL_BYTES = 256 * 1024 * 1024


def decompress_sql(raw):
    with gzip.GzipFile(fileobj=io.BytesIO(raw)) as stream:
        return read_stream(stream, MAX_SQL_BYTES).decode("utf-8", "strict")


def load_classic_db():
    """(Dump, dump file name, source ref dict) for the newest CMaNGOS classic-db dump."""
    from . import envelope
    listing = http.fetch_json(CMANGOS_LIST, "cmangos_full_db.json")
    entry = next(f for f in listing if f["name"].endswith(".sql.gz"))
    name, url = entry["name"], entry["download_url"]  # raw.githubusercontent.com, no cross-host redirect
    raw = http.fetch(url, name, timeout=300)
    sql = decompress_sql(raw)
    ref = envelope.source("cmangos-classic-db", CMANGOS_REPO, revision=f"{name} (git blob {entry['sha']})",
                          snapshot_sha256=http.used()[url]["sha256"])
    return Dump(sql), name, ref
