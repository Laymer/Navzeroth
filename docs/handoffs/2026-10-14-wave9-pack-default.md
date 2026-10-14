# Strict pack typed-empty-default correction

Owner: Codex /root/backlog_audit; sole writer. Published baseline v0.8.0,
db8530f8d5b8633292a038632e76494168310ccd (2026-10-14T16:48:12Z).
Worktree /tmp/navzeroth-wave9-pack-default; branch fix/content-pack-empty-default.
Coordination1034f27 merged before claim2add57d and implementation.
Production614e8ba; frozen complete source/testsf1e8d909e14fd92949b463700846916a649a2cdd.
Root owns protected-main integration/release. Atlas independently reviews this freeze.

## Change

Published real reference data has valid explicit `payload.map_embeddings: []`.
Validated Rust serialization omits this empty typed default, so the strict pack
comparison rejected it before reading the still-missing source registry.
`dataset()` now restores only that exact validated schema1 empty array to the
comparison-only serialized shape. The raw input and returned/canonical envelope
are untouched. Generic `typed()`/`known_fields`, duplicate parsing, unknown/future
schema and source/manifest behavior are unchanged. No ignored unknown field is
accepted because it is empty or named map_embeddings elsewhere.

Root extended sole ownership narrowly to `docs/data/CONTENT_PACK.md` to document
this exception after the separate registry writer froze/released its files.
No SOURCES.md/registry/dataset/PNG edit is part of this task.
The original raw reference digest remains
531860e5189146b17bfee5b2654dfbef58c4acb922d692f0f16c73a9726789db.
Edition/build/interface/namespace/fixture/revision stay
wow_forever/1.60.1.70205/16001/reference/false/forever-atlas-r1.

## Actual checks and regressions

At frozenf1e8d90:

- `cargo test --locked -p azeroth-cli`:62 passed,0 failures and one existing ignored
  generator. Log /tmp/navzeroth-pack-default-tests.log; private target
  /tmp/navzeroth-wave9-pack-default-target.
- `cargo clippy --locked -p azeroth-cli --all-targets -- -D warnings`:PASS;
  /tmp/navzeroth-pack-default-clippy.log.
- `cargo fmt --all --check` and `git diff --check`:PASS.
- New actual-reference unit parses the unchanged committed file, checks canonical
  identity against the normal contract decoder, original qualified identity and
  zero errors/three coverage gaps. It does not claim imagery or full pack validation.
- New synthetic integration case compares omission/explicit[] pack bytes exactly,
  verifies the pack and confirms input directory bytes are preserved.
- New12-case rejection regression checks null/object/false/zero/invalid nonempty
  embeddings, arbitrary unknown empty arrays, matching keys at wrong depths,
  future schema, duplicate keys and generic source parsing. Other empty defaults
  remain strict. Each failure has safe nonzero/empty stdout, no published invalid
  output, unchanged input and a byte-identical still-verifiable prior valid pack.
- Existing manifest/exact-tree/policy tests also pass in the full CLI suite.

Both positive regressions were added before the fix and actually failed at the
published parser with the expected noncanonical-field error (exit101):
/tmp/navzeroth-pack-default-baseline.log and
/tmp/navzeroth-pack-default-pack-baseline.log. These are successful reproductions,
not passing checks. Production/test commits are separate granular pushes.

An initial shell workdir mistake made the coordination merge run in the original
repository after worktree creation (creation does not switch cwd). Its local main
fast-forwardeddb8530f→1034f27 with docs only; no remote push or removal occurred.
Root checked exact tip/clean state and restored only that local pointer via
reset--keep to publisheddb8530f, preserving the coordination branch and all user,
remote and task worktree changes. Task coordination/claim were then performed in
the actual isolated worktree before code. Subsequent mutations used its explicit
exec workdir. This correction is not counted as validation evidence.

## Limits and next task

No full real-directory build/verify is claimed: the separate typed source-sidecar
repair is still required. This change neither supplies source permissions nor
promotes historical70205 content to70338 or verified playable content. Native/live
acceptance remains unverified; unknown/conditional policies remain distinct.
No source assets, public schema or game dataset revision changed.

Next: source-locked independent peer review, latest exact-head nine hosted gates,
root reconciliation with the separate registry PR, then actual combined local
real-directory build/verify. Distribution eligibility must remain false. Prepare
0.8.1 only under root's authorized release/asset gates and audit its published tag
again before claiming engineering-ready exhaustion.
