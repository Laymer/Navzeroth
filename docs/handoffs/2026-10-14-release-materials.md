# Verified offline content and source material delivery

Owner / sole writer: Codex `/root/onboarding`.
Claimed 2026-10-14 before implementation.
Baseline `03ef364`; branch `feat/release-materials`;
isolated worktree `/tmp/navzeroth-v1-materials`.

Scope: bounded deterministic release material planning, staging and artifact
verification from existing verified pack closures and SourceEvidence raw,
license and attribution file references; Windows packaging and pre-draft
verification. No accepted upstream sources, third-party raw datasets, core or
exchange schema changes. Root owns general README, STATUS and task ledger.
CLI registration depends on public_sources' frozen `e637a9` history; coordinate
the writer handoff and normally merge before editing main/lib.

Read AGENTS, STATUS, the previous enforcement handoff and implementation plan
sections 19–20. Preserve legacy empty-content 0.x artifact layout. Stable remains
blocked by actual mandatory content, rights, model and acceptance requirements.
The source CLI history was normally merged (`4ded378`, `7adcabf`) after its
writer released registration ownership. Existing published history is preserved.

## Implemented

Production commits `63a07f5` and `2d31b2f` add `release-materials`, a deterministic
bounded selection and new-directory staging command, and Windows artifact
delivery/verification. Existing core, exchange, source and pack schemas remain
unchanged. The new public release-only material and asset manifests use version 1.
See [material contract and commands](../releases/RELEASE_MATERIALS.md).

Selection validates the authoritative pack closure, explicit offline-use and
redistribution eligibility, exact accepted source IDs/revisions, source permission
and target proof hashes, and every selected regular tracked production file.
It validates before mutation, then streams exact bytes into a new temporary tree
and atomically renames it. Missing/changed files, invalid revisions or permission,
untracked/ignored files, symlinks/reparse paths, traversal and oversized input
refuse. Existing output is never overwritten. Empty preview selection creates no
material output and preserves the original five-file full package layout.

The material ZIP contains separate `content-packs/` and inert `source-materials/`
trees. It retains original bytes and terms; no upstream Lua or SQL is executed
or copied into the addon. Public source metadata includes only declared source,
license and attribution fields. Proof-only paths and private evidence metadata
are never selected automatically. A public LICENSE or attribution notice may
also be permission evidence, including an identical proof copy: its explicitly
public bytes take precedence over the overlap guard without adding that proof
copy's path. Other private-proof/raw-material overlap remains refused.

With a nonempty verified selection, Windows packaging adds the material ZIP,
`RELEASE_MATERIALS.json` and `RELEASE_ASSETS.json`. It validates before output
cleanup, after build, and again against downloaded assets before tag drafting.
Verification checks every bounded decoded ZIP entry against a freshly generated
selection; exact manifests, ordinary artifact set, commit metadata, release notes
and checksums must agree. Rebuilding outer checksums cannot hide changed inner
material bytes. Compact ordered asset JSON avoids different indentation between
Windows PowerShell 5.1 packaging and PowerShell 7 verification.

The temporary unconditional unsupported-delivery blocker is removed only after
this concrete delivery and verification path exists. Actual stable readiness
still requires complete mandatory inventory, accepted source permissions,
supported record bindings and real client/interactive Windows evidence.

## Actual checks and limitations

- The complete CLI suite passed after the initial material implementation:
  98 passed, one existing ignored generator. After adding the explicit optional
  PowerShell integration test and public-notice regression, focused checks pass:
  eight material tests, 13 release-readiness tests, and the explicitly invoked
  real CLI-to-PowerShell artifact integration test. The latter stages a synthetic
  pack and exact source/license bytes, verifies all seven addon-only artifacts,
  then changes a license and confirms refusal preserves prior output.
- Strict CLI Clippy (`--all-targets -- -D warnings`), formatting and whitespace
  checks pass after the Rust role-precedence change.
- PowerShell 7 on Linux passes 18 material transport checks, including fixed
  canonical manifest bytes, deterministic ZIPs, binary/CRLF byte identity,
  missing/changed/extra/traversal/duplicate/link entries, staged mutation refusal,
  source-commit mismatch and inner tampering despite rebuilt outer metadata.
  The existing 27 release-gate checks also passed after helper integration.
- Initial local tests correctly rejected synthetic pack declarations that still
  carried test-fixture provenance; the temporary fixture now explicitly declares
  its ordinary verification path without adding any real game/source assertion.
  A PowerShell object-array sort failure was fixed using typed string arrays.
- All records used by these tests are synthetic and confined to temporary
  directories. No third-party raw data, personal exports or accepted upstream
  source is committed or shipped. These tests do not establish real source rights,
  complete content, live gameplay, native Windows interaction or performance.
- The earlier enforcement PR 120 at `03ef364` has all nine hosted checks green.
  Fresh hosted Windows PowerShell 5.1/7, installer and repository checks remain
  required for this material-delivery branch and the coordinator's merge candidate.

## Independent review and next task

Reviewer `/root/acceptance` is reviewing the material implementation read-only.
The public-notice role-precedence and compact JSON follow-up are being sent as a
new frozen revision; actual findings and final review will be appended here.
At `5ae7df3`, the reviewer independently passed all 99 CLI tests (two ignored),
18 PowerShell 7 material checks and 27 existing release-gate checks. The reviewer
identified that both release jobs still checked out LFS pointers,
which would prevent a future verified LFS asset closure from passing its hashes.
Both release checkouts now hydrate LFS objects, while the selector still excludes
all unaccepted assets. A temporary local LFS test is explicitly invoked by the
Windows package job alongside the real CLI/PowerShell integration. Both explicit
integration tests now pass locally: a pointer-only clean Git checkout refuses
before staging, and local LFS object hydration delivers exact source bytes. The
first attempt wrote pointer text directly and exposed Git's stale index/stat
state, so the regression now uses a real skip-smudge checkout before hydration.
The reviewer also narrowed documentation wording to non-target pack contexts;
source-origin labels remain bounded provenance text, not authenticated editions.
No stable tag or release was created. Next: resolve independent findings, finish
all nine hosted checks, and let the coordinator normally integrate the reviewed
task. Real source acceptance, content population, unsupported class/role/party/
dungeon models and recorded client/Windows evidence remain separate prerequisites.
