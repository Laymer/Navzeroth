# V21: hosted offline firewall isolation and signed closure

Owner: Claude Code release continuation
(docs/claims/2026-10-23-claude-stable-release-continuation.md).
Baseline: candidate 2c6fe762, main 807ef674. Read
2026-10-23-claude-release-continuation.md for the overall plan and authority.

## Why V21

The V20 candidate 2c6fe762 passed CI, coverage, source parity and Pages, but its
Windows package run 39931563703 failed twice: the hosted acceptance step was
cancelled 306 s (attempt 1) and 280 s (attempt 2) after HOSTED-OFFLINE disabled
every network adapter. V19 attempt 39641767585/1 shows the same pattern (278 s).
The owner later attributed attempt 1 to a manual cancel and confirmed attempt 2
was not cancelled by them. In each attempt, INSTALL, RESOURCES, FIRST-START,
PERFORMANCE and PERSISTENCE-RECOVERY passed (recover 794 s). Offline was
interrupted, and UPGRADE-RC1 and DOWNGRADE-REFUSAL never ran. The draft gate
correctly refused the cancelled step. That runners lose their job lease without
network is inferred from timing; there are no runner diagnostic logs.
Evidence: release/history/2026-10-23-v20-offline-cancellations (hash ledger).

## Repair (PR #285, ADR 0010)

HOSTED-OFFLINE now isolates with Windows Firewall:
- default-deny in both directions, with every enabled persistent allow rule disabled;
- only the runner (Listener/Worker), the hosted VM agents (hosted-compute-agent,
  provjobd*, Azure guest agents) and DNS allowed, all by long path;
- isolation refused if an effective-store allow rule could admit the app,
  WebView2, Node or PowerShell;
- an external TCP connection refused during the workload;
- restoration exact and re-read.

Evidence label: `windows_firewall_runner_and_host_agents_only`. Historical
`network_disabled` cases keep their meaning. Bounds are unchanged.

Engineering probes (release/history/.../firewall-probes):
- 600 s with runner only: passed.
- 1260 s with runner only: lost twice, with no logs.
- The firewall refuses 8.3 short paths.
- 1260 s with host agents (39982856528): passed, with 84 refused checks and exact restore.

Independent review (Claude Code subagent): three rounds, final CLEAR WITH NITS,
all nits fixed. Merged normally at 1f972145. Tests run by the author:
- helpers 53/53 native PS7, 52 PS5.1, 52 Linux PS7 (each skips one
  platform-specific test with an explicit message);
- preparation 49/49, adapter probe 11/11, actionlint clean.

The reviewer could not run pwsh in its sandbox.

## V21 identities

- Tested source 1f972145f5adba8f253a92d924195563f24329dd; production
  7e1bb463d706584b78cd8d3d1b1a5856469eb389be4b1eec75d1ef200ef2ada5 over 3160
  entries (the method reproduces V20 d2cebaec/3158 exactly).
- Cargo --no-run confirmed the cached functional executable d1745d72 and CLI
  fbbf4571 (no new release build). The functional run at 1f972145 passed: 75.77 s,
  peak 6206036 KiB on shared Linux/WSL2. The reviewer's own rerun passed: 76.72 s,
  peak 6205924 KiB. No Windows or ordinary-PC performance claim.
- Evidence: same records and unknowns as V20. It differs from the V20 prebundle
  only in the proof refs and the source/production binding. Preview: ready, 82
  files, 2 packs. The ledger is release/v1/verification-v21. The `*.log` files are
  force-added, because .gitignore had dropped them; the review caught this.
- Review V21: release/v1/public-content-review-v21.json, 142226 B, SHA256
  7542fb89ed8954c648f5df48d548cf99b25a8dbae6804a78421db3f111919257. Evidence
  baseline 29814d3e; clear for receipt preparation only. Merged at 142dc213.
- Prepared at 9e5e4473 before key use:
  - manifest 17187 B, 1ce2b62650bea680e6ff019475b544ce5be8e980c9be099883f8b2be24b0bfa4
  - identity 528 B, b6336b301aa26ed7cb727c8fce43c0dbee63ac2068b68e231e1388bae879fc22

  The reviewer reproduced both byte-for-byte with its own build and built its own
  expected payload (equal).
- Receipt at 772608f8: 1080 B,
  eccd6b63f20e67af6d1cc8ede1864d826bbf6ca38f3c84da00a04ac0817cf29c, signed with
  the existing r2 keys (no rotation, both operator-controlled). Verified against
  policy c4bbc54f and the reviewer's expected payload.
- Closure (root and independent): assembly of 86 files, default compiled trust of
  84 files and 2 packs, and prebuild build_ready with 89 evidence files, 2 packs and
  exactly nine pending observations. Publication is false. The reports are in
  release/v1/verification-v21/signed-closure. The reviewer's negative control
  (an altered signature) is refused.
- The exact V20 signed and functional bytes are preserved in
  release/history/2026-10-23-v20-signed.

## Not verified

- Hosted Windows acceptance with the new offline isolation.
- All protected checks on the V21 candidate.
- Resulting-main stability.
- Downloaded-draft UI/native acceptance.
- Live client.

All nine canonical observations remain pending. No main merge, tag or publication
has happened.

## Next

Push the V21 candidate to PR #239 and read the actual results:
1. all ten protected contexts;
2. source parity;
3. per-file coverage;
4. all eight installed cases, including the firewall-isolated HOSTED-OFFLINE.

Then follow steps 3 to 10 of the continuation handoff.
