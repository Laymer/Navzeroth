# Deterministic unsigned bundle preparation

Preparation does not sign content, activate it, satisfy acceptance, or authorize
publication. The production Rust `release-materials` preview independently checks
source admission, rights, exact resources, target, records and pack revisions.
Schema2 material previews now optionally include `bundled_packs`, with directory,
manifest FileRef, actual `ContentRevisionKey`, typed conflicts (currently null),
and admitted raw normalization report FileRef. Metadata derives from already
validated v2 catalogs; the tool never reparses huge catalogs or guesses keys.
Schema1 output omits this field. Existing PowerShell material consumers access
known properties rather than deny unknown keys; archive byte closure still binds
the entire exact manifest. The separate Windows schema2 adapter remains required.

After integrating and independently reviewing production changes, commit them,
run required source checks, and record the actual tested production ancestor.
Commit public review evidence with its independent review claims and exact checked
revisions. Review evidence signatures do not establish independent key control.
Then update release metadata's tested-source/production/inventory bindings. Files
under `docs/` and `release/` are excluded from the production fingerprint; modifying
production content or code changes that fingerprint and requires renewed tests
and review. This fixed point does not classify missing Windows evidence as passed.

Run with explicit values from that recorded review and verified source checks:

```sh
python3 scripts/release/prepare-bundle.py \
  --root . --version 1.0.0 \
  --tested-source "$TESTED_SOURCE_COMMIT" --production "$PRODUCTION_SHA256" \
  --review-path docs/releases/actual-public-review.json \
  --review-bytes "$REVIEW_BYTES" --review-sha256 "$REVIEW_SHA256"
```

The command requires installed pinned Cargo tooling, invokes the production Rust
preview, verifies tracked resources, explicit committed review bytes, exact tested
ancestor production, target/revision/scope compatibility, clean production, private
registry exclusion and bounded same-handle hashes. Failed/partial pending checks
refuse preparation. Pristine pending acceptance remains unverified. Typed conflict
inputs require a future explicit reviewed preview contract; raw normalization
reports remain separately identified and attached to their actual revision.

Outputs are new files only:
`release/v1/bundled-content-manifest.json` and
`release/v1/bundled-content-build-identity.json`. Content paths remain repository
relative, including actual source notices, corresponding-source indexes and raw
reports. Files and packs sort by exact path. JSON has no whitespace or trailing
newline; identity field order matches the Rust receipt Payload serialization.
Repeat with the same verified inputs produces identical bytes. Existing outputs
are preserved; remove only task-owned drafts before an intentional regeneration.
No adversarial filesystem sandbox or multi-file atomic publication is claimed.

Review these outputs against the immutable public review artifact. Separately
issue, assemble and verify the signed content receipt using the documented
external-key CLI and independently supplied expected identity. Only then record
all three exact manifest, receipt and review FileRefs in matching schema4/5
`Evidence.bundled_content` and commit metadata. The receipt hash cannot be known
before issuance; keep the receipt outside its own hashed manifest. Preparation
never accesses a key. The compiled-trust source gate and assembler must
validate the final signed bundle. Native/hosted checks, exact downloaded draft
verification and the applicable final installer smoke remain mandatory before
publication: independently reviewed automated UI/native draft smoke for schema5;
historical owner interactive smoke for schema4. See [schema5](SCHEMA5_ACCEPTANCE.md).

Schema2 previews additionally include `unsigned_preparation` with `ready` and
`evidence_sha256`. The CLI reuses the full core release assessment, actual catalog
record/source accounting, and exact evidence-file verification. It permits absent
bundle artifacts at this phase; only pristine Windows observations may remain
pending. Missing/duplicate coverage, missing/extra records, empty source/proof
accounting, conflicts, corrections or stale source bindings make readiness false.
Historical schema1 previews omit this field. Schema4 prerelease material previews
remain available but cannot satisfy stable unsigned preparation. No readiness
result here permits publication. The preparation script requires the trusted
positive result bound to the same exact evidence bytes it parsed, in addition to
existing inventory/source/target/production checks. Final signed prebuild and
publication assessment still require the actual bundle and receipts.
