# Wave7 local state and governance batch

Owner: Codex /root coordinator. Released source baseline
739a82229a5b7369cab2a6d83c4b2a935bd0ba2e, v0.6.0.
Published2026-10-14T12:49:07Z, draft=false, prerelease=true.
User authorizes parallel task PRs, protected-main merges and recurring verified
releases. No further publication permission is needed.

## Publication evidence

Release PR91 checked head393471694246ff9fa97735672a310e7becc90535 passed all nine
applicable jobs: CI38563531839 and package38563531717. Downloaded PR assets used
synthetic merge ac420d016f827dcf3aaa7a48c6fd1854245c6f9b, whose tree exactly equaled
the checked head. Initial verifier invocation contained a repository-path typo;
corrected absolute-path rerun passed. No failed invocation is counted as success.
Main739a822 passed all nine jobs: CI38564691454 and package38564691259.
Annotated tagv0.6.0 peels to exact main739a822. Tag package38565804520 passed both
Windows installer/addon and Draft GitHub release jobs.

PR, main and tag downloads each passed the exact five-file asset set, all four
checksums, MZ header, ZIP integrity, all19 addon source/root-license bytes, package
version, release notes and exact40-character source metadata. Tag assets:

| Asset | SHA256 |
| --- | --- |
| AzerothNavigator-0.6.0-x64-setup.exe | f0e2221bfa9a0acc58e95cff4c1df9a36be07ac5aa917ff94967849282351991 |
| AzerothNavigator-addon-0.6.0.zip | 909b8bfba22ed5cfde56f8bfab4de39d070a801a4455a12e068ac25c82b98e4d |
| CANDIDATE_NOTES.md | cade82a1ef8ed32e3943a010ef4ed7e480d27806d62a981b86478536e76aae93 |
| SOURCE_COMMIT.txt | f9ea3b89cbc5bd81cdb1ea545f235fee31801c2ea1000a315242bed2d7462440 |

https://github.com/Laymer/Navzeroth/releases/tag/v0.6.0 is published with those
four assets plus SHA256SUMS.txt. Original clean main fast-forwarded fromf525911
to739a822 only after publication. Other worktrees and published history remain.
Versioned source71c436c passed746 portable Rust tests (one existing ignored asset
generator), strict Clippy/fmt,152 desktop tests and all30 mocked Chromium flows.
Independent content/schema checks passed73contract/33transport fixtures, no drift,
0content errors/3gaps and four unchanged PNG/four exact Git LFS pointer checks.
No native interactive or recorded-client acceptance follows from these results.

## Released-source audit and frozen ownership

Read published STATUS/ledger, task handoffs, module/tests and plan8/10/12/17.
Clustering, route-derived alternatives, agreed recipient export, preparation
infrastructure, compatibility and ordinary reference-window engineering are merged.
The following four finite scopes remain dependency-ready. M12.4 full acceptance
remains blocked; local groundwork does not satisfy focused-release prerequisites.

| Task | Sole writer | Worktree / branch | Scope |
| --- | --- | --- | --- |
| Today/Settings/current-plan reference | /root/backlog_audit | /tmp/navzeroth-wave7-today; feat/today-settings | Bounded settings/plan modules, guarded server-owned job pin descriptor, Today/Settings and read-only reference projection |
| Durable content restriction/fallback | /root/release_review | /tmp/navzeroth-wave7-quarantine; feat/content-dispositions | Pure policy, disposition storage/services, activation/rollback eligibility and governance controls |
| Local submission/moderation/withdrawal | /root/atlas_dense | /tmp/navzeroth-wave7-submissions; feat/local-submissions | Bounded opted-in queue, CAS lifecycle/tombstones and permission-gated local export |
| Typed gear/acquisition explanations | /root after shared foundation | Separate claim/branch/PR | Bounded ordinary contribution/unmodeled-effect/reason views using existing facts |
| Shared schema/recovery foundation | /root | /tmp/navzeroth-wave7-storage; feat/local-state-schema | Migration9, four tables, exact recovery registration/seed handling and shared eligibility read boundary |

Each worker claims baseline before code, owns its task exclusively and pushes
small coherent commits in a separate draft PR. Additive native/SDK/app registration
belongs to its feature; root normally reconciles shared entry files. Preserve the
actual reference-webview read-only caller gate and explicitly typed generated
handler. Root freezes foundation source before releasing overlapping content.rs
ownership to the content worker. Feature recovery validators join the shared gate
before publication; no populated feature document is silently trusted or dropped.

Settings persist explicit local choices without scanning, watching, importing,
installing, enabling providers or opening windows. Current-plan pinning uses a
bounded backend-owned successful descriptor, preserving one-shot report retrieval.
Validate original full dependencies and typed freshness at current UTC, including
rollback and newly usable fields; stale advice stays explicitly historical.
Empty plan clear retains its monotonic CAS tombstone. No snapshot/export duplication.

Content restriction retains immutable evidence and blocks reactivation; active
restriction atomically selects a validated previously activated eligible same-scope
fallback or clears active state. Missing/corrupt fallback cannot leave poisoned
content active. Eligibility does not mean verified evidence or redistribution rights.
Local review never activates content or upgrades confidence. Withdrawal erases its
payload and preserves bounded dedup/CAS tombstones; revocation remains terminal.
Explicit accepted-record export requires currently valid declared source permissions
and field/transformation policy. No uploads, accounts, hidden collection or hosting.

## Remaining external prerequisites

Real playable quest/acquisition catalogs, four documented role models, verified
Deadmines content, consolidated recorded70338 field/event/restriction/plan/session
checks, representative real sessions, native Windows interaction/performance and
DPI/monitor/restore acceptance remain unavailable or unverified. Party messaging
and combat-log format depend on verified permitted capabilities. Official Forever
API support remains unverified, not proven absent. World expansion remains gated;
hosting remains deferred under the approved local-first scope.

Next: implement shared schema while three feature workers proceed independently,
freeze APIs, review exact source and run meaningful checks. Require all nine fresh
jobs per updated task head, protected-main reconciliation and combined candidate
checks before the next minor developer preview; verify PR/main/tag assets again.
After publication re-audit every task against the actual released source.
