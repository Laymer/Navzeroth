# Installed bundled content contract 1

`crates/app::bundled_content` loads the installer's `bundled-content/` directory.
This is separate from legacy private receipt formats and pack schema2. Its exact
manifest bytes are pinned in a build identity (public receipt Payload); the native
build embeds that identity from validated `NAVZEROTH_BUNDLE_IDENTITY_JSON`.
The two trusted public keys are compiled from `content/bundled/signers-v1.json`.
Both keys belong to the same operator. Independent review is separately recorded
and hash-bound; neither a second key nor its signature establishes independence.

A `navzeroth-installed-content` schema1 manifest declares app version, true target,
review and accepted-origin FileRefs, all resource FileRefs and1–8 pack descriptors.
All packs share one non-fixture scope. Every file is listed exactly once, except
manifest.json and receipt.json, which have fixed roles. Receipt signatures bind
manifest, app/target, production hash, tested commit and exact review FileRef.
Extra, missing, altered, linked or special resources fail closed. Asset paths use
the existing lowercase traversal-free contract. Resource scanning is finite:
8192files,16384entries,16directory levels,2GiB declared total; each resource is at
most576MiB. Manifest/documents are at most4MiB; receipts remain64KiB.
Streamed reads check descriptor/path identity before and after, refuse no-follow
links/reparse points and never claim an ancestor-race filesystem sandbox.

Pack schema2 is verified independently against the pinned manifest. Catalog target,
revision, fixture status and scope must match; accepted origins are decoded from
the exact pinned bytes and every remaining gap/relabel finding refuses activation.
The already verified owned catalog/assets are staged directly, with optional
pinned typed source conflicts, without reopening content. Storage validates conflict
references. Optional `normalization_conflicts` preserves the generator's distinct
raw conflict report and binds its revision/precedence; it is shipped as a checked
resource, not silently converted into the typed source-conflict schema.

All packs stage and compose before the single-scope activation transaction runs.
A failure can leave immutable staged revisions but retains the previous active
pointer/preferences. Defaults and marked travel estimates activate before imports;
within-version choices persist and application-version changes reset them with
prior revision retained for rollback. See the separate bundled-defaults contract.
The initialization worker publishes ready only after validation and activation;
compatible backup restoration reruns the same initializer. Stable versions refuse
missing build identity; prerelease development builds can start without a bundle.

`azeroth-cli check-bundled-content --input DIR --identity FILE [--policy FILE]`
verifies the signed exact resource closure for packaging and reports its explicit
`verification: signed-resource-closure` scope. It does not activate a database or
replace full source/admission/readiness gates. Production packaging uses compiled
signers; the optional policy exists for explicit synthetic/tooling verification.
No receipt selects its own expected identity or trusted keys.

Actual stable bundle assembly, exact production/source bindings, independent final
review, hosted Windows first start/upgrade/recovery/performance and owner smoke
remain release prerequisites. This contract alone supplies none of those results.
