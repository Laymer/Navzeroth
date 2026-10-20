# Durable bundled defaults (STABLE-DEFAULTS)

Storage schema 13 records inferred-travel acceptance by immutable content revision,
and the last application version that applied bundled defaults by full content
scope (edition/build/interface/namespace/fixture). Activation and its travel choice
commit in one immediate SQLite transaction. Rollback selects the prior retained
revision and restores that revision's saved choice; failed activation leaves both
selection and preference unchanged.

The native resource loader must verify the shipped manifest, pack hashes, source
closure and receipt policy before using these application APIs:

1. `stage_verified_content_bundle(path)` verifies/stages each extracted pack.
2. `stage_content_set(keys)` validates/composes stored v2 revisions without moving
   active selection, returning the composed immutable key.
3. `configure_bundled_defaults(app_version, keys)` applies one default key per
   scope. On first setup or a different application version, it activates the key
   and accepts marked travel estimates transactionally with the version marker.
   Same-version calls preserve the user's current selection and travel choices;
   changing the configured bundled default key within that version is refused.

Call configuration before imports on fresh installation, and again at every
startup with the actual application version. Configuration may initialize known
bundled scopes before any character profile exists. New scopes get defaults;
additional profiles sharing a configured scope inherit its persisted within-version
choice. Content activation is already shared by scope, so creating another profile
does not reset the choices of existing profiles. This is not per-profile selection.

Each scope's reset is transactional. A multi-scope configuration may commit an
earlier scope before a later scope fails; retrying is idempotent for earlier scopes.
A previous active revision is retained for rollback when it differs from the default.
Changing only the travel choice of the same immutable revision does not invent a
second revision for rollback.

Active-content inspection exposes the last applied version, previous application
version and default revision, plus the current revision's persisted travel choice.
Content displays this reset history. It remains visible after manual selection or
rollback as the last defaults event, not a claim that the default is still active.

No actual shipped manifest/resource loader is added here: shipment/receipt closure
is separate integration work. These APIs do not authenticate arbitrary staged data
as shipped content. Source-backed records, estimates and unresolved states retain
their existing meaning. Hosted/native Windows acceptance remains separate.
