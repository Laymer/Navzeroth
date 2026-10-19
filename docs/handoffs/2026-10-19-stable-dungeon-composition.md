# STABLE-COMPOSITION

Owner and sole writer: Codex `/root/world_review`; [claim](../claims/2026-10-19-stable-dungeon-composition.md).
Baseline `88ce1ecbb087ff4bbec1e0db4c134f238a610c9d` plus dungeon commits
cherry-picked as `45b3b30` and `961455d`. No authored catalog edits or push.

## Decision and behavior

The same dungeon identity in authored and historical packs is now enriched
explicitly rather than rejected or renamed. Only an unambiguous authored source
and accepted historical 1.12/Classic source qualify. Missing, disputed, stale,
limited, contradictory or target-client categories refuse this special merge.
Inferred evidence remains neutral and does not verify target compatibility.
Other collection conflicts retain their existing refusal behavior.

Authored known values, including false, zero and empty collections, remain.
Restricted, unsupported and not-applicable observations remain. Only unknown
observations inherit a whole lower observation, retaining its uncertainty and
metadata. Checklists union by check identity; matching authored labels and known
conditions win. Both evidence lists remain. Disagreements become inspectable
SourceConflict records, including check identities.

The only geometry exception is the admitted Deadmines sentinel: exact dungeon
`dungeon:the-deadmines`, map `authored:the-deadmines`, floor `authored:pending`,
no instance. Its generated map replaces the placeholder and records a conflict.
Other authored geometry remains authoritative; arbitrary authored:* identifiers
are not placeholders. Historical planar entrance warnings remain visible.

Internal persisted v2 source-conflict keys add the explicit Dungeon field enum.
Storage validates dungeon and individual checklist references. Historical v2
keys remain readable; v1 explicitly refuses dungeon keys. Public addon exchange
schemas are unchanged. Existing paged JSON conflict inspection displays these
keys without an untyped promotion into game facts.

Composition identity policy 2 hashes the actual canonical composed tree, source
registrations, sorted conflicts and member identities with a policy marker.
Changing content under reused external revision labels cannot reuse a set ID.
Member order does not affect identity. Streaming hashing avoids another large
serialized hash buffer. Large header and extension clones were removed; the
composition Value is dropped before strict decoding its owned serialized bytes.

## Validation and limitations

Targeted tests: core dungeon enrichment 5, app content sets 9, storage catalog v2
5, and app source-category guards 1 passed. Tests cover explicit unavailable and
false states, exact sentinel scope, idempotence, deterministic member order,
conflicting categories, persisted activation and unchanged active state on
failure, legacy evidence fallback, changed bytes under reused labels, and
invalid dungeon/check conflict references.

The ignored actual-data test uses the full temporary world catalog described in
[the dungeon handoff](2026-10-19-stable-dungeons.md) and the committed authored
slice. It passed before the final allocation optimization: 26 dungeons, four
sources, one recorded map conflict, source preparation quest 166 and visible
CMaNGOS ghost-coordinate uncertainty. Identity:
`set-f585526da8a69a28ddac70c8d422bb19`. Linux debug compose 55.46 seconds;
maximum child RSS 7,867,068 KiB. This is a substantial measured limitation,
not an ordinary-PC or Windows claim. Final optimized run passed with the same identity: load 30.45 seconds, compose
62.08 seconds, maximum child RSS 5,678,904 KiB (about 28% lower). Portable
checks ran concurrently; these timings do not establish a speed improvement.

The test loads catalog/source metadata directly and does not establish installer,
receipt, redistribution or Windows readiness. Production conflict-file admission
is still required: the existing pack v2 reader has no conflicts channel and
stage_verified_content_bundle constructs an empty conflict list. Root owns the
separate native bundled-manifest adapter, which must validate and pass the
corresponding standalone conflicts file before storage staging. Composition
preserves supplied member conflicts and adds its own disagreements.

The strict v2 decoder still holds the strict JSON Value, a cloned Value used for
typed decoding, the typed model and a canonical comparison Value. It remains a
major full-data allocation source. Redesign must preserve duplicate-key, unknown
field, canonical-field and resource bounds; no checks were weakened here.
Portable workspace/all-target Clippy, Rust format and whitespace checks passed.
The initial Clippy run identified a test-module placement issue; the module was
moved to the end before the final passing check.
Coverage measurement, hosted Windows, native UI, performance acceptance, source
materials and signed production receipts remain independent release checks.

## Next task

Independently review this branch, integrate deliberately with default activation
and native bundle changes, retain actual generated conflict metadata, and run the
per-file 90% coverage and exact resulting-head checks before merge. Regenerated
production content must bind the same policy, source identity and uncertainty.
