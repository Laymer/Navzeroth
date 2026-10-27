# Deferred weekly draft #243: exact downstream workflow outcomes

Owner /root/desktop_review, docs-only audit; baseline generated candidate
c23dd2b5e1e8566df519ac58f921395c2f2c768e (github-actions[bot], data/world-weekly).
Isolated branch docs/weekly-draft-243-audit, /tmp/navzeroth-weekly-draft-243-audit.
PR243 is OPEN/DRAFT targeting main and remains deferred under the release scope.

Actual explicit runs all completed successfully at that exact head:

| Workflow | Run | Actual result |
| --- | --- | --- |
| CI | 39398609139 | Ten successful jobs: repeat stability, Rust format/Linux/Windows, pnpm Linux/Windows, Python, desktop shell, mocked Chromium and Lua checks |
| Coverage | 39398612016 | Linux Rust, Windows Rust, TypeScript and aggregate coverage gate succeeded |
| Independent source knowledge | 39398615432 | Both source platforms and source catalog parity succeeded |
| Windows package | 39398618648 | Windows installer/addon package succeeded; draft GitHub release job skipped |

Exact workflow/job identities, timestamps, URLs and results are retained in
[the status receipt](../data/weekly-draft-243-check-runs-v1.json). No job failed,
so there is no failure to classify as deferred data/binding or shared product
correctness. This audit made no repair or workflow retry.

Scope matters: the generated branch uses release inventory/evidence schema3 and
nine inventory units. Its source does not contain the later Stable-Acceptance.ps1
producer or deliverydf62631f. Its Cargo.lock and notice manifest consistently bind
8859ab2f..., unlike the later delivery's independently repaired3656470... lock.
Consequently its passing package does not contradict the actual later delivery
helper/notice failures and does not establish stable1.0.0 schema4 bundle, hosted
performance, exact-draft owner smoke or final readiness. The green runs do not
review/admit new gameplay facts, permit merging this generated update or replace
the reviewed final mapper4/profile2/r4 corpus.

Actions were read-only GitHub PR/run queries and Git metadata fetch. Worktree used
GIT_LFS_SKIP_SMUDGE=1; no upstream world acquisition, LFS hydration, data regeneration,
pipeline dispatch/retry, private key access, signing, release or merge occurred.
Only this claim, receipt and handoff were added; no product/data byte changed.
Prior weekly implementation/original-v3 handoffs were read as context, not treated
as fresh network or final-content evidence.

Next: retain PR243 as a deferred draft. Root continues exact reviewed stable
source/material/receipt and Windows acceptance work independently. If a later
weekly head is produced it needs its own source review and exact-head checks;
this receipt is permanently scoped to c23dd2b5.
