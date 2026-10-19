# STABLE-CORRESPONDING-SOURCE-MATERIALS

Owner and sole writer: Codex `/root/world_review`; [claim](../claims/2026-10-19-stable-corresponding-source-materials.md).
Baseline `b39635b`; isolated branch `feat/stable-corresponding-source-materials`,
worktree `/tmp/navzeroth-stable-materials`. No source-retainer edits or push.

## Contract

Parent approved the global schema4 Evidence.corresponding_source_files API.
It records reviewed retained-source FileRefs separately from source notices,
licenses and permission proofs. Mixed-generator originals, including permitted
SQL dumps, acquire no invented BSD/attribution meaning. Schemas1/2/3 refuse the
new field even null/empty. Historical serialized evidence is unchanged when the
field is absent. Schema4 Some(collection) requires nonempty, at most4096, unique
safe paths and positive lengths. Omission is backward compatible, not evidence
that retained sources do not exist; root must populate the actual retainer list.

The collection participates in release evidence byte checks and material
selection. Existing permission/admission, production Git membership/fingerprint,
private-proof exclusions, exact hash/size and clean-source checks remain. The
same public notice may serve two purposes without duplication: one archive
entry retains source_material and additional_kinds:[corresponding_source].
An otherwise distinct source file uses kind:corresponding_source. Original
license and attribution lists stay unchanged.

Schema4 material manifest is schema2, including corresponding_source_files and
catalog_files (sorted exact verified v2 catalog FileRefs). Historical material
manifest schema1 bytes retain their prior shape. The schema2 catalog collection
is the Windows adapter's exact eligibility list, not a blanket content-pack grant.

Parent additionally authorized schema4-only576MiB v2 catalog maximum, matching
pack::v2::MAX_CATALOG_BYTES, and2GiB aggregate checked input bytes. Only a pinned
verified catalog receives the larger file bound; other source/evidence files stay
256MiB. Schemas1/2/3 stay256MiB/1GiB. Role checks precede the verification cache,
so an oversized catalog cannot become an accepted original source. Release JSON
stays16MiB; actual production inventory size still needs measurement.

## Actual checks

- Material integration suite:22 passed, zero failed, two existing ignored.
  Four added cases exercise exact exported bytes/category, shared notice
  deduplication, hash/missing/duplicate/unlisted/empty/zero refusals, and historical
  null/empty/nonempty field refusal. First new staging check failed because the
  synthetic output parent was absent; fixed fixture before final passing runs.
- CLI release unit tests:8 passed, zero failed. Added boundary test exercises
  historical versus schema4 catalog/aggregate bounds, exact pin matching, cache
  role separation and refusal above576MiB. It seeds a verified cache rather than
  allocating a half-gigabyte synthetic file; actual large production pack material
  selection remains root's required integration check.
- Core release schema1/2/3/4 suites:47 passed, zero failed.
- Core+CLI all-target Clippy with warnings denied, formatting and diff checks pass.
- Local logs: /tmp/navzeroth-stable-materials-tests-final3.log,
  /tmp/navzeroth-stable-materials-cli-unit.log,
  /tmp/navzeroth-stable-materials-core-tests.log,
  /tmp/navzeroth-stable-materials-clippy-final.log.

No production receipt, actual source-rights claim, Windows run or per-file coverage
verdict was created. Existing per-file90% and hosted gates remain required.

## Integration

Root populates corresponding_source_files directly from the retainer's reviewed
corresponding_source.files. The workflow-adapter writer `/root/tooling_review`
owns Release-Materials.ps1 and was sent precise schema2/catalog_files scoped
Windows limits and schema4 preview-recognition requirements; no script edited here.
Root must exercise the genuine world+authored pack through material selection and
verify source-archive closure after integrating both adapters. Native bundling and
production identity/review/signature binding remain independent prerequisites.
