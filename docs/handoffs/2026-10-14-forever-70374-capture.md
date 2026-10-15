# Explicit experimental Forever 70374 adapter

Owner/sole writer: Codex /root. Baseline `bb3782c`, claim `683eed2` precedes code.
`369af26` centralizes exact experimental target admission, `823fd98` uses it in
all existing manual workflows, `1767eeb` moves the unsupported-copy regression
to genuinely unreported 70375. Copy-export production `f1a2af5` is a normal merge
dependency, not rewritten history. The copy writer froze Diagnostics before root
edited its build guards; Version.lua and existing diagnostics formats are intact.

User evidence: on 2026-10-14 the operator reports GetBuildInfo version1.60.1,
build70374, interface16001, Windows11/out of combat and error-free probe/status
with addon0.8.1. Installed bytes and new adapter behavior remain unverified.
This change enables guarded **experimental** reads on the explicitly identified
tuple; it does not mark any capability or release acceptance row verified.

Both exact70338 and70374 tuples are admitted. Other versions/builds/interfaces,
wrong types and unreadable build metadata remain blocked. Manual capture, clock,
position diagnostics, plan controls, recorder and HUD share the same predicate.
Existing readers preserve unknown/restricted states; actual build strings still
form snapshot/session/plan contexts. A cached70374 snapshot cannot restore into
70338. Clock trust remains explicit and resets on reload. Position interpretation,
objective completion mapping, plan dependencies and real content/model coverage
remain unverified or unavailable; no completion mapping is fabricated.

Actual local checks on production1767eeb: all119 Lua-harness tests pass across23
groups, strict affected all-target Clippy passes, formatting/whitespace pass.
Four new tests exercise exact tuples, unchanged capability status, combat/unknown
HUD gating and actual synthetic70374 capture→Rust import/session/clock/plan paths
with cached-context and unsupported-target isolation. These are synthetic tests.
An initial test compile used the wrong Rust module path and was corrected. The
first combined run failed because the copy task used70374 as an unrecorded build;
that negative fixture now uses70375. Neither failed run is counted as a pass.
Logs: `/tmp/navzeroth-v1-build-tests.log`, `/tmp/navzeroth-v1-build-clippy.log`.

Next: independent peer review and all nine current hosted checks, then verify the
candidate artifact's source and file hashes before guided installation. In-game:
`/anavdiag probe`, `/anavdiag copy`, then explicit snapshot/copy and desktop import;
record actual errors and field comparisons before expanding live evidence. No
release tag or stable readiness claim is part of this task.
