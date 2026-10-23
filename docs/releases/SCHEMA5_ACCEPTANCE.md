# Schema 5 automated downloaded-draft acceptance

Schema 5 is additive. Schemas 1–4 retain their existing required rows, methods,
origins, limits and readiness semantics. `SCHEMA_STABLE` remains 4. The new
`SCHEMA_AUTOMATED_DRAFT` is 5. Both stable contracts use the same 64 MiB manifest
ceiling and admitted content/source/bundle rules; historical schemas 1–3 retain
16 MiB. This contract implements the owner's replacement of mandatory human
installer smoke with independently reviewed automated native Windows acceptance.
No human smoke result is created. Target 70450, historical origins, resource
integrity, unsigned installer policy and the 90% coverage gate remain unchanged.
ADR 0009 governs this decision; ADR 0008 governs historical schema 4.

Exactly nine rows are required. The eight existing `HOSTED-*` rows retain
`hosted_windows`. `DRAFT-INSTALLER-SMOKE` uses `automated_draft_windows`.
Schema 4 still requires `OWNER-INSTALLER-SMOKE` / `interactive_windows`; rows
cannot be renamed or coerced to satisfy another schema.

Prebuild permits only pristine pending observations (all nine may be pending).
Draft readiness permits only pristine pending `DRAFT-INSTALLER-SMOKE`, after all
eight packaging observations pass against the same installer. Both preparation
phases always report `publication_allowed:false`. Full publication readiness
requires all nine passed, exact source/material bindings and the independent
structured evidence below. Failures, partial claims, wrong methods, stale hashes
and source/content blockers cannot be deferred. Draft creation precedes the ninth
row; this avoids a circular gate. A rebuild invalidates installer-bound results.

## External overlay

Only schema 5 accepts nonnull `Evidence.draft_installer_smoke`:

```json
{"raw_result":{"path":"dist/evidence/raw.json","bytes":1,"sha256":"..."},"provenance":{"path":"dist/evidence/provenance.json","bytes":1,"sha256":"..."},"independent_review":{"path":"dist/evidence/review.json","bytes":1,"sha256":"..."}}
```

These three distinct FileRefs must appear exactly bound in the final row's
artifacts. Each JSON file is at most 1 MiB. They are external acceptance evidence,
not new signed bundle resources: recording them after the tag does not rebuild
or sign the installer, alter the bundle, or rewrite the tagged source.
Use existing `release-readiness --root . --version 1.0.0 --evidence OVERLAY`.
There is no automatic trust in an overlay or in booleans: the CLI verifies hashes,
strict required fields and the recorded API/source/policy bindings.

All three JSON documents carry `schema_version:1`. Raw, provenance and review
share an exact `identity` object:

```json
{"source_commit":"40 lowercase hex","production_sha256":"64 lowercase hex","installer_sha256":"64 lowercase hex","release_tag":"v1.0.0","release_id":1,"asset_id":2,"asset_name":"setup.exe","asset_bytes":1,"run_id":3,"run_attempt":1,"job_id":4,"workflow_sha256":"64 lowercase hex"}
```

`source_commit` must equal the exact tagged checkout being assessed, and
`production_sha256` the source fingerprint. The workflow hash is SHA256 of exact
source bytes at the policy's workflow path. All numeric identifiers/sizes are
positive unsigned integers. The installer hash matches the overlay's candidate
installer and all passed rows. Producer identity must come from separately
verified release metadata, never from a receipt selecting its own expected pins.

Raw fields (unknown fields refused): `identity`, `phase:"draft"`,
`human_performed:false`, and all true booleans `downloaded`, `unsigned_verified`,
`installation_verified`, `first_start_verified`, `bundled_resources_verified`,
`planning_verified`, `restart_persistence_verified`, `ui_interaction_verified`;
plus `download_receipt` and `native_result` FileRefs (each JSON <=1 MiB).

The existing download receipt is schema 3, exact version/source and draft phase,
installer and application signatures `NotSigned`, verified dependency closure and
addon/source equality, and an asset matching installer name/bytes/SHA256.
Its `workflow_run_id` and `workflow_run_attempt` strings must exactly equal the
identity numbers in decimal form, preventing prior-run receipt reuse or coercion.
Additional verification detail fields are retained. Native result is schema 1,
exact source/installer, `phase:"draft"`, `completed:true`, `passed:true`,
`human_performed:false`, `ui_interaction_verified:true`; `checks` contains exactly
four true fields: `first_start`, `bundled_activation`, `source_backed_planning`,
`restart_persistence`. `detail_files` contains 1–16 distinct FileRefs, each <=8 MiB,
for underlying IPC/UI reports and optional screenshots. All bytes/hashes are
verified; images alone do not establish acceptance. Extra native diagnostic fields
are permitted. Required typed authority fields reject duplicate keys.

Provenance fields (unknown fields refused): `identity`, `repository`,
`workflow_path`, `job_name`, `runner_environment:"github-hosted"`, `os:"Windows"`,
`phase:"draft"`, and `run_api`, `job_api`, `release_api` FileRefs (each <=1 MiB).
These snapshots are original GitHub REST run, job and release responses. Required
API fields are typed; extra provider fields are retained. The run must be a
completed successful workflow_dispatch of the exact source/repository/workflow;
the job must belong to that run/attempt/source, be completed successful and match
policy name, Windows runner label and GitHub Actions runner group. The exact policy-named native test step must occur once, completed and successful;
missing, duplicated or skipped steps cannot pass. Installer asset ID and name
must each identify one asset, and the download receipt must have one matching name.
The recorded
release must be the exact stable draft/tag/release ID with the matching asset.

Review fields (unknown fields refused): `identity`, exact `raw_result` and
`provenance` FileRefs, `status:"pass"`, nonempty bounded `reviewer` and
`recorded_at`, `human_performed:false`. The independently recorded review must
inspect the actual downloaded bytes/native detail and the exact harness code,
API evidence and source binding, rather than copying producer claims.

## Trust boundary and sequence

`config/release-draft-smoke-v1.json` is production-tracked caller policy. It fixes
repository, existing download-verifier workflow/job and hosted Windows provider.
It pins `Download and verify exact release` as the actual native test step.
It cannot be selected by evidence. The coordinator fetches run/job/release API
snapshots from the trusted repository and expected IDs **after** the test job
completes, independently reviews them and retains their hashes before assembling
the overlay. A running job cannot attest to its own final success.

The offline CLI validates recorded bytes and consistency; it does not authenticate
GitHub API snapshots by itself or establish reviewer independence from a string.
The independent review and trusted API acquisition are operational requirements.
Both content-receipt keys remain operator-controlled, not independent custody.
Public verification uses the saved draft-time evidence while separately checking
actual public release metadata and unchanged downloaded bytes.

Generated structural schemas are in `schemas/release/`. Regenerate with:

```
cargo run --locked -p azeroth-core --example export_release_schema -- inventory > schemas/release/inventory.schema.json
cargo run --locked -p azeroth-core --example export_release_schema -- evidence > schemas/release/evidence.schema.json
```

Runtime assessment remains authoritative for exact rows, phase semantics,
versioned field restrictions, inventory accounting, content/source validity and
file/Git identities. This document records a contract, not an executed draft,
human smoke, hosted pass or publication.
