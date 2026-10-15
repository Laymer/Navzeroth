# Independent review: v1 foundations

Baseline: `bb3782cbac13b5fb3e6ca8196278ba7fcc427474`. Scope: source audit,
acceptance procedures, current onboarding and separate release-only contracts.
No live or interactive Windows result is inferred from review or portable tests.

| PR / reviewed revision | Independent reviewer | Result |
| --- | --- | --- |
| [113](https://github.com/Laymer/Navzeroth/pull/113), `1e923c5` | Codex `/root/acceptance` | No blocker/important findings after backup-location wording clarification; commands, source behavior and 37 relative links checked |
| [114](https://github.com/Laymer/Navzeroth/pull/114), `7d9d93c` | Codex `/root` | No blocker/important findings; the 25 mandatory IDs and live/source/native evidence boundaries match the implementation plan |
| [115](https://github.com/Laymer/Navzeroth/pull/115), core `c6a5792` | Codex `/root/onboarding` | Unsupported schema and excessive evaluation-work findings corrected; no remaining core blocker/important findings; actual Git/file/content-record binding belongs to the separate CLI boundary |
| [119](https://github.com/Laymer/Navzeroth/pull/119), `7ec7429` | Codex `/root` | No blocker/important findings; all 26 full-file byte lengths, SHA-256 and Git blob identities independently verified against the pinned sources |

Root rechecked source provenance/licensing claims against the immutable primary
pages and the local retrieved bytes. The audit distinguishes public availability
from distribution permission, historical edition from target verification, raw
ID-line counts from effective corrected records, and tree-only metadata from
complete-byte hashes. Neither upstream source is accepted for shipped assets.
No upstream Lua or SQL was executed and no raw upstream dataset is included.

The contract's twelve focused tests and strict core Clippy pass. A passing
synthetic evidence fixture verifies validator behavior only; all committed real
acceptance rows remain unverified. Unknown required inventory IDs remain unknown.
The CLI must independently bind declared record IDs to verified actual pack data,
check bytes/paths/source revisions and prove production ancestry and identity.

PR113 and PR114 each passed all nine current hosted checks. Original PR115
`7f94919` also passed all nine. PR119 and the combined foundation candidate require
their current hosted results before protected-main integration. GitHub is the
authoritative current check record; later merges do not inherit a test result
from an earlier commit. The combined candidate preserves each task's commits
through normal merges. All nine checks must run on that integration candidate.

Remaining mandatory full-content, rights, role-model and live/Windows evidence
gaps are release blockers, not defects hidden by this foundation review.

On the combined foundation tree, root reran the 12 core release tests successfully,
checked all 48 relative file links in the current v1/install documents with zero
missing targets, and passed `git diff --check`. This resolves the runbook's three
previously planned release-contract links. Hosted integration checks follow.
