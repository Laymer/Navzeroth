# Wave5 hardening batch

Owner: Codex /root coordinator. Frozen source baseline:
2d5de646e439885c9eea27f5bfb4e2869a2ada60, v0.4.0.
Published2026-10-14T07:17:48Z; released-source audit confirms the remaining
engineering gaps listed below. Wave5 implementation starts from this baseline.

## Frozen ownership

| Task | Sole writer | Worktree / branch | Owned scope |
| --- | --- | --- | --- |
| M6.6 dense Atlas | /root/atlas_dense | /tmp/navzeroth-wave5-atlas; feature/wave5-M6.6-atlas | AtlasScreen, MapView, feature-local CSS, synthetic fixtures/unit/browser checks |
| M9.2 recovery | /root/release_review | /tmp/navzeroth-wave5-recovery; feature/wave5-M9.2-recovery | storage recovery module, app recovery host, recovery panel, native startup/wrapper and shared command/App wiring |
| M9.3 pure search control / measurements | /root/backlog_audit | /tmp/navzeroth-wave5-performance; feature/wave5-M9.3-search-control | core search control/export, travel/solo/joint loops, focused tests and declared synthetic performance harness |
| CLI doctor | /root | /tmp/navzeroth-wave5-cli-doctor; feature/wave5-cli-doctor | CLI doctor/options/tests/docs |
| Coordination | /root | /tmp/navzeroth-wave5-coordination; docs/wave5-coordination | current STATUS/task ledger, coordination/publication handoff |

Workers record their own claim before implementation. Each task has its own PR,
small coherent commits and pushes. Root owns subsequent separate CLI pack/replay
and application planning-job tasks; those begin after needed interfaces exist.
Root does not concurrently edit recovery-owned native/shared command/App files.
Recovery changes require a source-locked peer review before merge.

## Interfaces / acceptance

Core search control: SearchControl::checkpoint(SearchPhase) -> Result<()> with
no-op wrappers preserving existing APIs/results; controlled shortest-route,
solo-plan and joint-plan operations check graph, edge, participant, expansion,
candidate and selection loops. Cancellation returns OperationCanceled, never
partial ordinary success. Core reads no clock; app later owns atomic cancellation,
bounded jobs/progress/deadlines, immutable input capture outside store lock and
snapshot/revision/membership publication guards. Core control PR alone does not
claim user-visible cancellation complete.

Recovery: maintain ready/recovery-required startup states. Inspect explicitly
selected bounded SQLite files read-only and validate canonical historical schema
before application-table queries; no imported SQL execution, extensions, extra
views/triggers or future/inconsistent migrations. Rebuild trusted tables into an
app-controlled staged candidate, validate typed documents/hashes/context/pointers,
retain exact inspected candidate identity, and preserve original DB/WAL/SHM on
replacement or failure. Exclusive operation lease coordinates outstanding calls,
watcher shutdown and reopen. Reuse transaction-consistent SQLite backup. Recovery
controls remain available after startup corruption. No personal database is used
for tests or restored by the implementation agent.

Atlas: measure declared 1k/5k/10k synthetic pins first; introduce bounded rendering
only based on findings, keeping searchable/selectable data and visible counts.
Cover keyboard/focus, narrow layout,200% text, forced colors, map/floor separation
and schematic fallback. Linux mocked browser evidence is distinct from interactive
native Windows and live gameplay.

CLI doctor: report software/build requirements and explicitly selected paths;
no recursive account scanning, system modification or arbitrary input echo.
Later build/verify pack uses a frozen bounded versioned manifest and actual hashes,
coverage/context/source metadata; replay uses declared bounded synthetic inputs.
No pack work claims permitted real playable content exists.

## Gates / unresolved acceptance

After checks and focused review, update task PR with main, rerun all nine applicable
exact-head hosted jobs, merge through protection, then verify combined main and
release assets for v0.5.0. Any new dependency-ready task may use an available slot
once its sole-writer boundary is released. Reaudit after publication.
Recorded-build live capabilities/stage mappings, real permitted content/models,
real comparable sessions, native focus/DPI/display and official API prerequisites
remain blocked/unverified as in the current ledger. M12 remains dependency-gated;
local-first moderation and no unnecessary hosting remain the user preference.

## v0.4.0 publication evidence

PR65 checked head a63546a351940cd31e85fd91ad9c6b046da4fc86 passed all nine
applicable hosted jobs (CI38525129433, Windows38525129277). Downloaded PR package
source b3dba00c67b9dde37fe6f2110917448f1274a601 was verified against PR source;
all five assets, four checksums, notes/version,19 exact addon files and license
passed. PR65 merged into2d5de646e439885c9eea27f5bfb4e2869a2ada60.

All nine applicable merged-main jobs passed: CI38525894006 and Windows38525893977.
Downloaded main package passed the same exact-source/byte checks. Tag v0.4.0
points to that checked main. Tag run38526659343 passed package and draft-release
jobs. Downloaded tag assets passed sha256sum --check --strict and exact-source,
version, notes,19 addon source files and root-license byte verification.

| Published asset | SHA256 |
| --- | --- |
| AzerothNavigator-0.4.0-x64-setup.exe | 5b77dcd313a5c72196408b220322abd6ea08eb6f58b71a962a06a240ba24ac8a |
| AzerothNavigator-addon-0.4.0.zip | 80660e87be766d3cba906e2cfaac752d97446b6ec55c94dfea16f89de8625c6e |
| CANDIDATE_NOTES.md | 450ac4da20130a5410b3ec4114b99cc74d4dba13786925cd31ccf54501abaf21 |
| SOURCE_COMMIT.txt | 48e8cc313179c9c9765998895f147cc3e12fd05c25e71b5fceb1254ddec38b4f |

GitHub release https://github.com/Laymer/Navzeroth/releases/tag/v0.4.0 is public,
prerelease=true,draft=false. Prior releases/tags unchanged. Original user main
worktree was clean and fast-forwarded to released2d5de64; preserved worktrees and
published history were not removed/rewritten. Native/live acceptance remains
unverified; hosted installer evidence is scoped to this source and syntheticgamefolder.
