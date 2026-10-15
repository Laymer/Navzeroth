# Offline source pipeline handoff

Owner and sole writer: Codex `/root`, branch `feat/offline-source-pipeline`.
Claim: `24646a6`; baselines `11e701a` (PR144), `99f2e85` (PR146), main
`bbd5140` (PR143). Existing requested workers reviewed read-only; no new agents
were spawned. Source ownership remains with the coordinator.

The user requested independent automated sourcing/builds, evidence-backed gap
filling and one converged view prioritizing reliability and freshness. Implemented
`source-sync` (explicit bounded fetch, otherwise offline) and `source-build`
(offline even for direct Rust callers), shared strict JSON, pure bounded source
adapters and a versioned uniform knowledge catalog. Reliability ranks before
trusted observation freshness; provenance, conflicts and origin edition/build
remain intact. All nine inventory denominators remain explicitly unknown.
This is not a playable pack, desktop activation or full release acceptance.

Admitted three narrow published grants without requiring email: selected original
AtlasLootClassic declarations (GPL2), Mapzeroth Retail EK node literals (MIT),
and a WoWSims paladin source pointer (MIT). Retained seven exact originals/licence
texts (269,701 bytes; ~263 KiB), mapped in `content/source-materials/NOTICE.md`. AtlasLoot-derived
JSON carries explicit dated transformation/GPL notices. CI bundles the complete
transformation repository source alongside exact original/licence bytes. Eight
other candidate/excluded registry entries are never fetched. ForeverDB remains
unadmitted; its contact reply is optional, not an offline build dependency.

The plan produces 1,681 facts: 718 acquisition declarations, 896 geography fields,
66 dungeon/source-group fields and one model reference. States: 1,397 known,
258 unknown, 26 unsupported. No verified target-usable facts for Forever70374.
Other domains are present as unknown inventories, not fictitiously empty units.
Model coefficients, drop entitlement/rates, actual availability and target
eligibility are not derived. No imported code was executed or installed.

Automation: PowerShell offline build entrypoint, Linux/Windows twice-built
artifacts and platform parity, weekly/manual separately permitted GitHub metadata
checks. Metadata policy is independently reviewed/bounded and bound to registry
SHA256. New revisions require admission/adapter review; network failure leaves
freshness unknown. Rust toolchain/dependencies require separate provisioning;
this work does not vendor Cargo or claim a cold-machine air-gapped product build.

Actual checks on the implementation before PR:

- Portable workspace Rust tests: **1,014 passed, 0 failed, 4 existing ignored**
  across 144 result summaries (`cargo test --locked --workspace --exclude
  azeroth-desktop`). Includes Lua parity and existing schema/content tests.
- Strict workspace Clippy excluding desktop, Rust formatting and whitespace pass.
- Eight CLI pipeline integration tests pass: no-network determinism/rebuilding
  from bundled closure, rights/field/transform/material refusal, tamper/missing
  inputs, old output preservation, strict metadata policy and direct-library
  network prevention, unsafe paths/links and actual committed inputs.
- Nine core convergence tests and three adapter tests pass, including >1,024
  facts, trusted-time precedence, cross-build isolation, strict/nested duplicate
  JSON, numeric/depth bounds, false/zero and unvalidated confidence refusal.
- Two actual PowerShell builds using offline CLI compilation produced exactly
  13 byte-identical output files, including all seven original/licence objects.
  Catalog SHA256: `ac2d4ecac0a62082aef56830abc2bcc9f59ceda10f2a8c6f594ab1c574bca00c`.
- Explicit bounded pinned-source acquisition succeeded for the seven blobs;
  independent licence reviewer verified their bytes/sizes/hashes. A separately
  permitted three-request metadata check found all three retained revisions
  current at check time. No continuing freshness guarantee is inferred.
- Contract/transport fixture validation passes (73 contract and33 transport
  fixtures); generated v1 exchange schema is byte-identical. Readiness at1.0.0
  refuses version mismatch; preview0.8.1 reports stable/candidate readiness false.
- JSON and metadata-policy/registry digest binding checked. Personal exports,
  reply email and raw contact receipt remain outside this change.

Independent review found and resolved unsafe float number spellings, promotion
of explicitly unvalidated facts, a registry reread race, missing Cargo workflow
path filters, separate metadata retrieval permission, actual dated GPL derivative
notices, a direct-library offline bypass and transport that was not clamped to remaining
aggregate retrieval capacity. Reviewers confirmed their original
findings closed; the last library guards have focused regression coverage; independent review
also confirmed the remaining-budget transport fix.

Hosted source workflow [38767261441](https://github.com/Laymer/Navzeroth/actions/runs/38767261441)
passed both OS builds and platform parity against synthetic merge
`fbf540b2e03b65bb0b5edff0bbcc642d2fd589a2`. Downloaded both complete artifacts:
all six output documents match each other/local output, all seven objects match
retained bytes/SHA256, and bundled transformation source matches reviewed code.
These results do not establish interactive native Windows acceptance.

Integration at this record: PR143 merged as `bbd5140`, PR144 as `a845127`;
all nine resulting-main checks passed for each. PR145 passed all nine and merged
as `06503c9`; all nine resulting-main checks passed. PR146 passed all nine and
merged as `d9c4fab`; its resulting-main checks are pending. Pipeline PR147 head
`654a1e9` passed all nine existing checks and all three source workflow jobs.
This continuation incorporates main plus documentation synchronization and the
independently reviewed VMaNGOS material-closure research. No implementation or
retained input changed. Current head needs its own checks before protected
integration, then resulting-main verification. Native
interactive Windows and real70374 gameplay acceptance remain **unverified**.
Preview0.8.1 and all release gates remain unchanged; do not publish1.0.

Next: integrate checked PRs, verify hosted Linux/Windows parity and download the
complete source artifact. Continue review of independently licensed database
scopes and original permitted70374 captures with an independently evidenced
inventory. Use the uniform contract for additional admitted adapters; never
substitute historical reference counts for target completeness. Product content
consumption/acceptance is separate work once target facts are evidenced.


Additional bounded research while checks ran: three pinned VMaNGOS text files
(10,087 bytes) trace the separate base-database dependency. Public metadata/tree
inspection showed ten archives and no root licence/notice; archive internals and
release payloads were not downloaded or independently verified. Mutable master
and moving-tag metadata do not bind exact base bytes. Independent source review
confirmed text hashes and the qualified conclusion; root tree response was not
independently reopened. Kept source unadmitted without creating a new required
permission reply. Recorded details in `docs/sources/vmangos-closure-review.json`.
The admission-contract documentation now reflects three current scopes rather
than its original empty baseline; coordinator ownership explicitly recorded.


Archive verification follow-up: comparison of every archived file against the
Git tree found that `git archive` expanded four LFS pointers into images even
though checkout used `lfs:false`. Knowledge documents, seven originals/licences
and transformation code remained correct, but that archive was not an exact
source snapshot and depended on unintended LFS retrieval. Withdrew **eight**
generated source artifacts from the six pre-fix workflow runs (no releases or
published history changed). Prior artifact links retain workflow logs but no
longer supply those withdrawn downloads. Local copies remain only as regression
fixtures outside Git.

The archive command now disables LFS process/smudge filters explicitly. A local
archive matches all **1,068** committed Git blobs. A new fail-closed CI guard
checks every LFS path against its exact Git blob before upload. Its actual local
regression accepts the pointer archive and refuses the original expanded-image
archive. Independent review caught hidden producer failures in an initial
process-substitution guard; replaced that with checked temporary-file commands
and a NUL-safe loop. Reviewer confirmed the fix, with Windows confirmation
pending. No Rust implementation or source-data pins changed; current exact-head
hosted checks and replacement-artifact verification remain required.
