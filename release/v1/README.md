# Forever release requirements (manifest versions 1, 2 and 3)

> **Schema 3 is in effect ([ADR 0007](../../docs/decisions/0007-world-data-automation.md),
> task V2.8e, 2026-10-19).** The committed manifests are schema 3. The target is not
> frozen in code: it is read from `world-data-target.json` (currently
> `wow_forever / 1.60.1 / 70450 / 16001`), and inventory and evidence must equal it.
> The 15 `LIVE-*` rows and the live slice matrix are no longer required; the 10
> `WIN-*` rows are unchanged. Evidence binds `accepted-origins.json` by FileRef hash;
> the gate accepts evidence of the listed origins and reports accepted origins and
> accepted unknowns instead of counting them as coverage gaps. Hashed evidence,
> receipts and "unknown is never zero" stay mandatory. Details:
> [WORLD_DATA.md, release gate](../../docs/data/WORLD_DATA.md#release-gate-adr-0007-task-v28e-2026-10-19).
> `release::upgrade_inventory_v3` and `upgrade_evidence_v3` are the explicit v2 to v3
> migration. The schema 2 text below (pending target, LIVE rows, matrix) is historical.

> **Schema 2 is in effect ([ADR 0006](../../docs/decisions/0006-v1-program.md),
> task V1.1, 2026-10-17).** The committed manifests declare the 1.0.0 slice and a
> **pending target** (`"target": null`): the target becomes the first stable live
> build after the 4 Nov 2026 launch, frozen in code by task V4.2. Schema 1 (frozen
> 70374 target, whole-world scope) remains readable for historical manifests and tests.

## Schema 2 (task V1.1)

- `target` is always present. `null` means pending. A manifest can't choose its own
  target: `release::expected_target` returns 70374 for schema 1, and
  `release::frozen_release_target()` (currently `None`) for schema 2. While it is
  `None`, every gate reports "Release target is pending the first stable live build
  after launch (ADR 0006)", and no content can bind to it. Evidence must carry the
  same schema version and target, including `null`.
- `slice` (required) declares races, zones, dungeons, level band, classes and party
  sizes (1 to 5). `classes: null` means the launch class list is unknown and blocks
  separately from an invalid slice.
- Each domain may list `out_of_scope` boundaries (`id`, `scope`, `reason`). A
  boundary is neither `unknown` nor `unsupported`, and it never satisfies a unit:
  units inside the slice still need exact required records and passing coverage.
- Evidence `matrix` (task V1.4): live rows with `class`, `level_min`/`level_max`,
  `zone`, `party_size` and the usual status/method/operator/time/environment/
  artifacts. The gate derives the required coverage from the slice: every class
  in every ten-level band (`release::level_bands`: 1-10 and 11-20), and every zone
  and party size in at least one row. Rows outside the slice, duplicate IDs,
  failed, automated or partial rows block; pristine `unverified` live rows are
  pending observations. While `classes` is `null` the matrix can't be checked and
  blocks once; the committed matrix is empty until the class list is known.
- `release::upgrade_inventory` and `upgrade_evidence` are the explicit v1 to v2
  migration. Units, sources and evidence rows are kept; the target becomes pending.
- Other release modules (party matrix, local-data delivery policy, inventory
  investigation) still name 70374. Task V4.2 moves them to the frozen target.

Committed slice: Human; Northshire Valley, Elwynn Forest, Westfall, Redridge
Mountains; the Deadmines; levels 1 to 20; classes pending launch; solo and duo.

The sections below describe the shared rules. Where they say 70374, read the frozen
target of the manifest's schema.

Historical schema 1 target: `wow_forever / 1.60.1 / 70374 / 16001`, reported by the
user on 2026-10-14. It identified the client; it did not establish addon, content or
Windows acceptance. The developer preview application version is 0.9.2.

`coverage-inventory.json` and `acceptance-evidence.json` are release contracts,
separate from exchange schema v1. Their initial contents deliberately block a
stable release. No source is accepted for distribution yet. The nine broad
inventory rows are **enumeration tasks**, not a claim to enumerate every game
record. Replace them with audited region/model/party coverage units before
claiming completeness. Unknown required IDs are `null`; a justified known empty
unit uses `[]`. Neither means unsupported, false, or silently omitted.

## Contract and review requirements

The pure validator is `crates/core/src/release.rs`. Schema version 1 rejects
unknown and duplicate object fields, unsupported enums and oversized documents
(16 MiB), and bounds units (4096) and required records (2,000,000 since ADR 0007; 250,000 before). Domain and
acceptance lists are fixed in code: removing a mandatory row from JSON does not
make it optional. Any later format change requires an explicit version/migration.
Existing exchange, source-registry and content-pack formats remain unchanged.

- Inventory: every domain must have a complete, evidenced enumeration; each unit
  has an ID, scope and exact required record IDs. IDs are domain/record qualified
  strings selected by the adapter mapping. Source IDs and immutable Git revisions
  are frozen here only after source review. A source count is not a target-world
  completeness claim. The final review must establish exhaustive target scope.
- Coverage: one passing row per inventory unit, exactly matching its required
  IDs, with no unresolved conflicts or unsupported corrections. Validation files
  must record reference/prerequisite checks, coordinates, directed connections,
  acquisition links, model provenance and representative scenarios as applicable.
  An unexplained `pass` cannot be accepted in review.
- Sources: preserve origin edition/build separately. Accepted sources require a
  pinned `admission_registry` FileRef bound to reviewed compatible rights, checked
  permission evidence, exact original source files, pack metadata and allowed
  fields; see [offline admission](../../docs/sources/source-admission-v1.md#offline-release-binding).
  The registry is private evidence and is never a release material. Each accepted revision needs
  exact raw-file hashes, license and attribution files, redistribution evidence,
  and validation for the frozen target. `unknown`, `conditional` and `prohibited`
  permissions block publication. Cross-edition data does not become verified by
  changing an edition string. Required corresponding source stays under upstream
  terms and must be included in the release asset manifest.
- Acceptance: the 15 `LIVE-*` and 10 `WIN-*` rows in the
  [runbook](../../docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md) require `pass`, their
  actual method (`live_game` or `interactive_windows`), operator, time, environment
  and hashed evidence. `automated` cannot satisfy either method. All real class,
  role, level, region and party-size units also need representative coverage
  evidence; the aggregate duo run alone cannot establish full-content acceptance.
- Offline packs: list each ordinary content-pack manifest, hash and dataset
  revisions. The CLI must rerun the existing pack verifier, reject fixture packs,
  nonzero coverage gaps, mismatched target and distribution-ineligible sources.
- Only official API, party messaging, combat-log analysis and hosted distribution
  may be explicitly deferred. Their reasons do not waive core local workflows.

Every `FileRef` has a relative portable `path`, exact `bytes` and `sha256`. The CLI
must check files beneath its selected repository root, reject symlink traversal,
and impose per-file and total read limits. Reading a report never executes it.
Manifests cannot prove that a human observation is true; independent review of
the actual evidence remains a release requirement.

## Tested revision binding

Evidence records `tested_source_commit`, its `production_sha256`, and the exact
inventory SHA-256. A later documentation-only evidence commit must not require
retesting unchanged software. The CLI verifies that the tested commit is an
ancestor of candidate HEAD and has exactly the same production fingerprint.

The production fingerprint hashes the sorted Git tree entries for **all tracked
files except `docs/` and `release/`**. The serialization is the raw `git ls-tree
-r -z --full-tree <commit>` entry bytes, retaining mode, type, object hash, path
and NUL terminator for each included entry. SHA-256 over that sequence is the
fingerprint. Workflow, installer, application/addon version, tests and content
changes therefore invalidate previous evidence. Git LFS pointer blobs identify
their immutable objects without requiring image downloads for this comparison.
The CLI must also reject tracked working-tree modifications to included files;
untracked product files cannot be included in a release candidate.

The candidate's actual source SHA remains in `SOURCE_COMMIT.txt`; it must never be
replaced by the tested ancestor. A shallow checkout that lacks the tested commit
cannot establish ancestry and cannot pass stable readiness.

## Enforcement

The batch-2 CLI boundary and workflow integration provide:

```sh
cargo run --locked -p azeroth-cli -- release-readiness --root . --version 1.0.0
```

Its JSON report separates `stable_ready` from `publication_allowed`. Existing
0.x developer previews and explicitly named prereleases such as `1.0.0-rc.1`
may retain honest blockers; they must remain GitHub prereleases. Stable versions
must fail when any required evidence, inventory, source permission or pack check
fails. Malformed versions/manifests are errors, not a route around the gate.
The stable gate is required before stable packaging/tag draft creation and
again before publication. All nine existing CI checks and independent review on
the exact merge candidate remain separate requirements.

No 1.0.0 tag or stable publication is justified by these initial manifests.

## CLI record bindings and supported boundary

The CLI generates canonical JSON-array strings from verified typed pack payloads,
then compares the complete set to inventory required IDs. Strings must be exactly
canonical; changing whitespace or inventing an ID cannot create a matching record.
The leading element is the dataset revision, followed by collection and identity:

| Domain | Collections and identity suffix |
| --- | --- |
| quests | `quests, quest_id`; `quest_stages, quest_id, stage_id`; `objectives, objective_id` |
| equipment | `items, item_id, variant` |
| acquisition | `acquisitions, acquisition_id` |
| geography | `map_spaces, map_id, floor, nullable_instance`; `map_images, image_id`; `map_pins, image_id, pin_id`; `map_embeddings, child_map_id, child_floor, nullable_child_instance, parent_map_id, parent_floor, nullable_parent_instance` |
| travel | `nodes, node_id`; `edges, edge_id` |
| reminders | `rules, rule_id` |

For example, an item identity is `["dataset-r1","items","item-123","base"]`.
IDs preserve typed string/nullable identity; they never use ambiguous joined
separators. Dataset revisions may not be reused across packs. Each coverage unit's
source IDs must match the union of sources cited by the datasets supplying its
records. Every pack registry source must be accepted at exactly its pinned revision,
and the union of shipped registry sources must equal the accepted inventory.

No validated pack yet supplies class/role, party-scenario or dungeon preparation
records, although the v2 pack path (`crates/core/src/content_v2/records.rs`,
`crates/cli/src/release.rs`) can now bind them. The CLI explicitly blocks those
mandatory domains when they are absent;
a dungeon acquisition row is insufficient to stand in for a verified dungeon
package. Real binding requires actual existing/approved contracts and representative
tests, never invented models. Passing pure declaration tests does not imply the
full CLI can accept records that no validated pack provides.

The [release material delivery boundary](../../docs/releases/RELEASE_MATERIALS.md)
selects verified pack closures and declared raw source/license/attribution files,
stages exact bytes, and emits a separate material ZIP plus public manifests for
nonempty selections. Packaging and the tag workflow verify the exact material and
artifact sets against current source. This replaces the temporary generic
unsupported-delivery blocker; missing actual coverage, rights, models and
acceptance continue to block stable readiness. Empty-content previews keep their
existing artifact layout. Local pack verification alone is not delivery evidence.

Packs and all their datasets, registries and imagery must be tracked production
files outside `docs/` and `release/`. Otherwise an evidence-only commit could change
content without invalidating the tested production fingerprint. The gate rejects
untracked or excluded pack payloads. Generated packs in an ignored output directory
remain useful local artifacts but cannot supply stable acceptance. A future scheme
for separately stored content needs an explicit tested-content digest binding.

The I/O boundary caps one evidence file at 256 MiB, cumulative reads and verified
packs at 1 GiB, and Git output at 32 MiB. Manifests keep their stricter core/pack
limits. It rejects symlink/reparse path components, traversal, unsupported schema
versions, version mismatches, missing/changed file hashes, unrelated tested commits
and production changes. Evidence file reads are deduplicated by exact path/hash/size.
It never executes referenced source or evidence. Concurrent hostile filesystem
replacement is outside this local release workflow.

`release-readiness` emits JSON; exit 0 means publication is allowed for the supplied
version, exit 1 means a failed gate or invalid input, and exit 2 means invalid CLI
options. For previews, `stable_ready=false` and the blocker list remain visible even
when publication is allowed. The six source declarations and eight internal
`azeroth-*` Cargo.lock versions must match the requested version. Stable packaging calls the
gate before touching output files, including with `-AddonOnly` or `-AllowDirty`.
It rejects ignored/untracked addon inputs and symlink/reparse entries because the
ZIP and installer recursively package that directory; an ignored file cannot
silently become a source-bound artifact. Stable builds rerun readiness after
building, before checksums are finalized, to detect changes made by build tools.
The tag workflow reruns the gate before creating any GitHub draft. Stable packages
require Cargo; existing preview addon-only packaging retains its no-toolchain use.

## Explicit acceptance candidates

`candidate-readiness` accepts the same arguments as `release-readiness`, with a
separate exit decision: exit 0 requires `candidate_ready=true`, and its output
always has `publication_allowed=false`. Report version 2 adds `candidate_ready`,
typed `pending` requirements and the exact inventory digest. Publication behavior
of `release-readiness` is unchanged.

Only pristine `unverified` mandatory live/Windows rows (correct method, no claimed
operator/time/environment/artifacts) and an initially absent pair of tested commit
and production fingerprint may be pending. Missing/duplicate rows, partial claims,
failures, stale existing bindings, malformed evidence, changed hashes, incomplete
content, unsupported model bindings and unresolved rights still block candidates.
A recorded pass without a tested binding is invalid, not pending. All referenced
files and packs undergo the same integrity and material checks.

`Build-Windows.ps1 -AcceptanceCandidate` writes to `dist/acceptance-candidate` by
default, includes a checksum-covered `ACCEPTANCE_CANDIDATE.json`, and requires the
candidate gate before output mutation and after building. The marker pins source,
production and inventory and explicitly denies publication. The normal verifier
rejects it; candidate verification requires an explicit switch and reruns the gate.
Stable verification independently reruns full readiness, including if someone
removes the marker and rebuilds checksums. No tag job uses candidate mode.

The Windows workflow exposes a manual, non-tag `acceptance_candidate` input and
labels its uploaded artifacts separately. Default PR/main/tag packaging retains
its full stable gate, with no fallback. The currently unknown content means no
1.0 acceptance candidate can yet be built. This implements packaging separation,
not the separately required typed party support matrix or full v2 content contracts.

## Stable 1.0.0 declared exclusions (ADR 0008)

Trainer-spell gameplay, talents, professions and enhancements are deferred in the
versioned `deferred-features.json`, displayed in Content and recorded as inventory
boundaries. Browsable source records do not enable recommendations for those
systems. Boundaries never satisfy missing supported records. See
[scope preparation](../../docs/releases/1.0.0-scope.md); historical acceptance
results remain unchanged. The broader stable contract is implemented separately.
