# V1 bounded staging, enforcement and capture integration

Owner / sole integration writer: Codex `/root`.
Claim `6dd53e7` precedes changes. Initial candidate baseline `bb99c51`, foundation
main `3cf4f0aa59f206fc04c34d14c8d9d6bd8710e7ce` subsequently merged normally.
Branch/worktree: `integration/v1-enforcement-capture` / `/tmp/navzeroth-v1-integration`.
Original main worktree and published history are preserved.

## Implementation and ownership

Normal merges preserve separate task histories for PR116 (Party availability),
117 (copyable stored exports), 118 (guarded 70374 admission), 120 (release I/O and
Windows/workflow enforcement), 121 (bounded raw staging) and 122 (staging CLI).
No source is accepted or gameplay catalog populated. The nine-domain inventory
and 25 live/Windows evidence rows still block full-content publication.

The coordinator reconciles current addon/runbook/README/release-process wording,
records narrow user reports and the verified candidate artifact, and updates
shared STATUS/ledger. Published 0.8.1 is distinguished from development source
with the same displayed version. Historical handoffs remain intact. Task writers
freeze shared files before ownership transfers; CLI registration passed from
enforcement to staging CLI and then to the separate materials-delivery worker.

Independent reviews and actual worker checks are in the
[review record](../reviews/2026-10-14-v1-adapters-enforcement.md) and each task's
handoff. Core/exchange schemas, imported game-data identities and published tags
retain their scope. No raw upstream datasets or personal exports are committed.

## Actual checks and recorded failures

Foundation PR115 head `bb99c51` passed all nine checks before reviewed merge
`3cf4f0a`; main CI `38630482291` and packaging `38630482244` also passed all nine.
Task PRs116/117/118/120/121 passed their current nine hosted checks.
Fresh integration and corrected PR122 checks must pass before this batch reaches
protected main. Exact run/revision results will be appended when available.

The ledger's 79 unique M0–M12 rows exactly match the implementation-plan task IDs:
55 IMPLEMENTED_UNVERIFIED, 16 BLOCKED and 8 DEFERRED_WITH_REASON. No whole task
is promoted by synthetic or artifact checks. Source/evidence re-audit identifies
mandatory missing playable records, corrected target enumeration, rights, real
class/role models, objective/dependency mappings and native/live checks. Stable
asset delivery is a concrete independent next implementation, not waived.

Root verified 83 relative file links in the current README/status/ledger, addon,
release and staging documents, all present, and `git diff --check` passes. The
combined affected check `cargo test --locked -p azeroth-cli -p azeroth-ingest
-p azeroth-lua-harness` passes: 272 tests, zero failures, one existing ignored
generator. The root log is `/tmp/navzeroth-v1-integration-tests.log`. This checks
the composition of release/staging imports and addon capture/copy behavior; it
does not establish live or interactive Windows acceptance.

For the guided 70374 check, root downloaded artifact `11925586142` from package
run `38627880967`, PR118 head `8ec4edc`. Its source `004058f` has the identical
tree `240f6670a30d00f87e23e2230b2ab20b49269ff9`. Exact five-file set, all four
checksum entries, SOURCE_COMMIT bytes, notes bytes, installer MZ header, ZIP
integrity and all 21 addon files match source. Independent acceptance reviewer
repeated these checks. The first notes assertion incorrectly expected new
candidate/source text and failed; exact source-byte comparison was then used.
The notes are existing 0.8.1 notes, so separate candidate instructions state its
additional guarded build and copy features. See the
[artifact record](../releases/candidates/70374-004058f.json).

The Windows release test initially passed all 27 assertions but left native
LASTEXITCODE=1 for the GitHub wrapper. Author and peer reproduced it, then verified
the explicit-success exit fix `03ef364` returns zero after success while an
injected failure still returns nonzero. PR120 reran all nine successfully. PR122
inherits that reviewed fix through normal merge; its earlier failure is retained.

## Acceptance and release limits

User reports client `1.60.1 / 70374 / 16001`, Windows 11, out of combat, addon
`0.8.1` and error-free probe/status. Exact installed files and serialized output
remain uninspected. The verified new candidate was linked with installation
instructions and a pending `/anavdiag probe` then `/anavdiag copy` request. No new
candidate observation is recorded. No aggregate live/Windows row passes.

QuestieDB redistribution rights and exact-target completeness remain unknown;
CMaNGOS is historical with separate copyright restrictions. All 18 correction
providers and derived passes remain unapplied. Real quest stages, equipment stats,
travel geometry/connections, acquisition facts and class/role models are absent.
Three mandatory pack-binding contracts and real advancement/session scenarios
also remain incomplete. Stable v1.0.0 and entry to the final RC batch are blocked.

Next: finish fresh combined/PR122 checks and protected-main verification; complete
the independently claimed release-materials task using verified contracts and
synthetic tests; continue the guided client/Windows checks when observations arrive.
Populate only accepted, fully validated source coverage units. Optional API,
messaging, combat logs and hosting deferrals do not waive mandatory local workflows.
