# STABLE-TOOLING: consumed-byte integrity and bounded generator inputs

Owner and sole writer: Codex `/root/tooling_review`; baseline `5d5fef9`;
branch `fix/stable-generator-integrity`. Claim:
[STABLE-TOOLING](../claims/2026-10-19-stable-tooling.md).

HTTP cache admission now requires the requested URL, integer byte length and
SHA-256 of the exact consumed body before recording successful source provenance.
Missing metadata remains an offline cache miss; malformed/incomplete metadata,
wrong URL, changed bodies and oversized inputs fail closed. Existing synthetic
fixtures now contain genuine digests instead of placeholder/missing hashes.

A shared reader checks regular-file type/size before opening, refuses final
symlinks, verifies descriptor identity, and reads at most the checked size plus
one byte. Streams similarly stop at the configured limit plus one byte.
Network responses and cached bodies are capped at 64 MiB, metadata at 64 KiB;
SQL gzip expansion is capped at 256 MiB and decoded strictly as UTF-8. Fact
registration, source-shard citation reads and objective-independent fact projection
check the existing 8 MiB shard limit before reading. Registration retains its
public RegisterError failure convention. Isolated Python invocation remains valid.

Limits have headroom against the retained inputs inspected read-only in the
existing datagen worktree: largest compressed download approximately 13 MiB,
SQL uncompressed size measured by chunked reading **73,289,738 bytes**. The
8 MiB bound matches the existing shard writer and source-document contract.
This does not establish an aggregate process-memory or wall-clock bound: parsers
still materialize finite input products; the network uses its existing per-read
socket timeout and workflow timeout. Concurrent ancestor replacement is outside
the file reader's guarantee. No cache is silently repaired and no historical
provenance is reclassified as verified.

Actual Linux validation:

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s data-generators/tests`:
  **130 passed**, no failures (125 existing plus five new refusal test methods).
- `python3 -I data-generators/register_facts.py --help`: passed.
- `python3 -I data-generators/navzeroth_datagen/to_facts.py --help`: passed.
- `git diff --check`: passed.

New synthetic tests exercise wrong URL/hash/size, absent hash/size/metadata,
changed bytes, finite response consumption, oversized cached files, pre-open
file refusal, final symlinks, gzip expansion, and registration/projection bounds.
The first run caught legacy placeholder-digest fixtures and an exception-type
regression; both were corrected before the final successful run. An isolated
CLI check caught the new import path under `-I`; the explicit local package
path now matches the existing projection script convention.

No network pipeline, whole-corpus regeneration, Windows check, coverage percentage,
key operation, push, merge or release is claimed. Next: independent review and
exact-head required CI; the integrator owns subsequent release work.
