# Schema 4 stable acceptance contract (ADR 0008)

Schema 4 inventory/evidence and readiness reports preserve schema 3's target,
accepted-origin policy, content, source, rights, coverage and deferral contracts.
They do not require live-client observations or a launch-build freeze. Schema 1,
2 and 3 retain their previous meaning; no observation is automatically migrated.
Current repository manifests remain schema 3 until an explicitly reviewed migration
records and retains historical evidence.

Schema 4 replaces the ten historical WIN rows with exactly these nine rows:

| ID | Required method |
| --- | --- |
| HOSTED-INSTALL | hosted_windows |
| HOSTED-FIRST-START | hosted_windows |
| HOSTED-UPGRADE-RC1 | hosted_windows |
| HOSTED-DOWNGRADE-REFUSAL | hosted_windows |
| HOSTED-PERSISTENCE-RECOVERY | hosted_windows |
| HOSTED-RESOURCES | hosted_windows |
| HOSTED-OFFLINE | hosted_windows |
| HOSTED-PERFORMANCE | hosted_windows |
| OWNER-INSTALLER-SMOKE | interactive_windows |

The upgrade row means upgrade from 1.0.0-rc.1. Performance evidence must record
full-pack installation, loading, activation, browsing, planning, peak memory,
workload, hardware and limitations. Hosted evidence cannot establish ordinary-PC
performance or interactive display/accessibility acceptance.

Evidence adds optional `candidate_installer_sha256`; each observation adds optional
`installer_sha256`. Every passed schema4 row must identify exactly that candidate
installer, with existing source revision / production fingerprint binding and
operator, date, environment and hashed artifact evidence. The owner row covers
installation/upgrade, first start, bundled activation, representative planning and
restart/persistence of the independently verified draft installer. Missing, failed,
wrong-method or mismatched-installer observations hold stable publication. No
installer executable is committed. Actual binary/hash verification remains the
independent download-verification boundary, not a claim made by this pure model.

Historical schemas reject populated installer bindings and hosted_windows methods.
A pristine unverified row retains unverified status. Existing source/hash/file and
independent-review boundaries remain unchanged. Relevant production changes revoke
the existing source/fingerprint binding and require new installer observations.

Artifact preparation is explicit and never authorizes publication:

- `release-build-readiness`: schema4 prebuild permits pristine pending hosted and
  owner rows. Source / production identity must already be bound. Initial missing
  installer identity is permitted only while no passed observation requires it.
- `release-draft-readiness`: all hosted rows must pass against one candidate
  installer; only pristine pending OWNER-INSTALLER-SMOKE may remain.
- `release-readiness`: unchanged final publication gate, including owner smoke.

Both preparation commands perform the same full source/content/file/receipt I/O
verification as release-readiness, then expose `phase`, `build_ready`, and
`publication_allowed:false`, retaining the detailed report with its own publication
flag forced false. Failure, partial unverified claims, wrong methods, stale source
or installer hashes, initial unbound source identity and other content blockers
cannot pass preparation. Historical schemas cannot use these preparation adapters.

Outstanding integration: packaging must use prebuild, draft verification must use
draft readiness and public verification/publication must retain full readiness.
Those workflow adapters remain unimplemented by this contract change. Current
stable packaging still uses full readiness, so draft preparation stays blocked
until that explicitly reviewed adaptation. Hosted evidence producers and actual
results, manifest migration and exact-draft owner smoke remain outstanding.

## Reviewed corresponding-source materials

Schema4 evidence may declare `corresponding_source_files`, a nonempty reviewed
collection of exact FileRefs (at most4096, unique safe paths, positive bytes).
Populate it from the admitted retainer's `corresponding_source.files`, including
mixed-generator source such as permitted SQL originals. This is a source-material
identity, not a claim that original bytes are BSD-licensed or an attribution notice.
Schemas1/2/3 refuse this field even when null or empty. Omission preserves existing
schema4 manifests; it does not assert that any unlisted corresponding source exists.
The release preparer must enumerate the actual reviewed retained-source closure.

All references participate in release evidence byte verification. Export requires
tracked production files outside docs/release, exact hashes and sizes, the tested
Git production fingerprint, and existing permission/admission checks. Missing,
altered, duplicate, unlisted or private-proof-only paths refuse selection.

Schema4 exports RELEASE_MATERIALS.json schema2 with the sorted
`corresponding_source_files` collection and `corresponding_source` file category.
A file already exported as a notice/source material stays one archive entry;
`additional_kinds:["corresponding_source"]` retains the additional purpose.
Its original source license and attribution declarations remain unchanged.
Older release-material manifests retain schema1 and their prior serialized shape.

Only schema4 may use a576MiB catalog allowance matching the v2 pack contract,
and only for an exact verified pack catalog FileRef. Material manifest schema2
exposes these exact pins in `catalog_files` for the Windows archive verifier. The aggregate checked-input
budget is2GiB. Original/corresponding source and other evidence files retain the
256MiB bound. Schemas1/2/3 retain256MiB per file and1GiB aggregate. Cached reads
cannot turn a catalog allowance into permission for oversized source resources.
Release JSON remains bounded to16MiB pending actual inventory measurements.

Schema4 whole-world inventories may declare `slice:null`: their scope is the exact
admitted corpus, not a claim of complete Forever races, eligibility or game data.
All nine domain/source inventories, genuine boundaries and functional validation
still apply. Schemas2/3 retain their historical slice requirement. Measured compact
real inventory/evidence are about28MiB each; schema4 permits64MiB, while historical
schemas remain16MiB before typed decoding. Other manifest bounds stay unchanged.

Only schema4 may accept exact admitted non-observational sources for materials
without a target-validation pass. The I/O boundary must verify every occurrence:
an authored source must satisfy authored-only rules in a single-source pack, or
all evidence citing an inferred source must be Inferred under the applied inferred
policy, with no directly attributed map-image assets. Mixed observational evidence
or a stronger occurrence in another pack revokes this eligibility. Every existing
file, field, transform, source, rights and pack-classification requirement remains.
The checked source set is supplied to the pure schema4 assessment; it never changes
`target_validation:unverified`. Historical contracts do not accept that set.

Schema4 `normalization_conflict_files` pins versioned raw world-normalization
reports. Material schema2 exports category `normalization_conflicts` with matching
pins. Format, version, selected catalog revision and precedence must match the
actual applied policy; the records are not coerced into typed SourceConflict rows.
Historical evidence rejects this field, including null. It is separate from the
mixed-term corresponding generator source and notices.

Party support schema3 binds an explicit well-formed target to its exact catalog;
pack/release consumers independently compare the actual target. Schemas1/2 retain
their original historical/frozen-target meanings. The authored catalog revision
`navzeroth-authored-slice-r1-release-contract-v2` changes release metadata and pins,
not the authored gameplay heuristics; later role-coverage work must regenerate its
catalog/source/party pins rather than silently reusing this revision.
