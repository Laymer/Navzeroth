# V22 app-data isolation continuation

Owner and sole integration writer: Codex `/root`.
Claim: `docs/claims/2026-10-24-codex-v22-continuation.md`.
Baseline: release candidate `b81e55c576df22d73f5d6ece3396a3b556376198`.

## Completed

Independently reviewed Claude Code's PR #286 at `2c443580`; no blocking findings.
Review: `docs/reviews/2026-10-24-appdata-isolation.md`. Integrated normally at
`2d33c2fd935c6b91754170862eda36b2d64561cc`. Original checkout and its unrelated
untracked work remain untouched. The release worktree's V19 claim is preserved.

Both failed V21 attempts from run 39990706776 are preserved byte-for-byte under
`release/history/2026-10-24-v21-attempts`, including original API responses, raw
case reports and overlays. The ledger pins downloaded ZIP identities; ZIPs remain
outside Git. Retained state.json is the disposable hosted synthetic engineering
state, not a personal export or SQLite database. Attempt 1 failed FIRST-START;
attempt 2 passed six cases, failed UPGRADE-RC1 and never ran DOWNGRADE-REFUSAL.
The earlier FIRST-START cause remains unknown. No old pass satisfies V22.

Seven exact V21 signed/functional inputs are retained under
`release/history/2026-10-24-v21-signed`. Codex independently checked all 287
FileRefs in the prior content review against actual files and Git/LFS identities:
zero mismatches. Runtime and content bytes are unchanged by PR #286.

## Actual checks

- Linux PowerShell 7.5.4: 57 helper checks passed.
- Native Windows PowerShell 5.1: 57 helper checks passed; native PS7.6.7: 58.
  Both native invocations used process-only ExecutionPolicy Bypass. The initial
  PS7 attempt lacked Node on PATH and failed; the successful rerun explicitly
  selected native Node 24.15.0. No persistent environment/policy changes.
- Node workflow tests 6/6, workflow-gate checks 12/12, actionlint exit 0.
- Preparation unittest discovery: 9 tests passed.
- Cargo --no-run and CLI build confirmed the existing cached optimized artifacts;
  no new compiler build claimed. Actual full-pack functional test passed in
  80.568 seconds, peak child RSS 6,206,072 KiB on shared Linux/WSL2 at fixed HEAD.
  The earlier 76.209 s execution recorded launch source `2d33c2fd` but its proof
  recorded metadata descendant `3647efdc`, because the archive was committed while
  it ran. The independent reviewer caught the mismatch; those exact initial bytes
  are preserved under `release/history/2026-10-24-v22-initial-functional` and are
  superseded. The replacement rerun asserts launch, proof and terminal HEAD agree.
  The first
  invocation omitted --ignored and executed no assertions; it is not evidence.
  The successful rerun explicitly used --ignored and produced the pinned proof.
- Regenerated inventory retains 260,113 records and 274,059 accepted unknown
  fields; coverage inventory is byte-identical to V21. Nine observations remain
  pristine pending. No hosted Windows or ordinary-PC performance claim.

Tested source: `3609c2b574d5302711a40cc08128b1e9c76f8955`.
Production fingerprint: `a947d2f092f9ed4921676c008745c87c837325102db04b106460121871295537`
over 3,160 entries, excluding only docs/ and release/ under the existing contract.

## Review, preparation and signed closure

V22 unsigned material preview passed (82 selected files, two packs, preparation
ready). The initial checkpoint `c6cd6749` is superseded by the fixed-HEAD binding.
The old active manifest/identity/receipt were removed after their exact archive
and have now been replaced by reviewed V22 identities. All nine observation rows
remain pristine pending. Both archive hash ledgers and
the tested-source/final-checkpoint production fingerprint were rechecked.
Plain git diff --check reports the retained native CRLF bytes and the original
functional log's final blank line; raw evidence was not reformatted.

The coordinator authored the new proof/binding metadata and cannot describe its
own audit as independent review of that work. The user approved
the read-only reviewer request; `/root/v22_independent_review` independently reran
the full-pack assertions (80.668 s, 6,205,284 KiB), verified source/content/archive
FileRefs and identified the corrected binding issue. Its raw results are retained
under `release/v1/verification-v22/independent-review`.

- Frozen independent review committed `546a54b3`: 15,109 bytes, SHA256
  `6131a419eeaea47e7ab2f7e8d974628dd5991eb9e1637fbbc188f622e06f7190`.
- Unsigned preparation committed `16226936` before any key use: manifest 17,185
  bytes, SHA256 `a912304e5815eb2ae4246b8810f96c91900a136ed514bc7f8dbdc64ddb000935`;
  identity 527 bytes, SHA256
  `744c83712454f01acf138ecafbc03b5abe2a318cde07017c2276d7b8cbe557b5`.
- Reviewer independently reproduced both byte-for-byte from source/preview pins
  and constructed its own expected payload before any receipt existed. Raw
  reproduction evidence committed `8289ae28`, also before key use.
- Receipt committed `c5586b6e`: 1,079 bytes, SHA256
  `79686be6c729e0c694ff9be356b58b8216fc1f5759ddfd79bed62558dd202a91`.
  Existing r2 keys matched; no rotation. Both keys remain operator-controlled.
- Coordinator and reviewer independently verified crypto, separate assembled
  86-file trees (84 resources, 395,738,463 resource bytes), default compiled trust
  without a policy override, and prebuild readiness: 89 evidence files, two packs,
  build_ready true, publication false, exactly nine pending observations and no
  other blockers. Reviewer's altered-signature negative control refused.
  Actual public outputs/ledger: `release/v1/verification-v22/signed-closure`.

The adjacent downloaded-draft verifier's APPDATA assignment is also ineffective,
but its separate fresh hosted job launches no earlier installed app. No analogous
contamination path was found. It is not isolation evidence; inspect actual fresh
draft results. No unrelated production change was made.

## Next

Then require fresh protected checks, per-file coverage, two-platform source parity
and all eight real installed cases. Follow the existing 2026-10-23 continuation
and schema5 downloaded-draft recipe for resulting main, tag, draft verification,
publication, public verification and website deployment. No main merge, tag,
publication or acceptance promotion has occurred in this continuation.
