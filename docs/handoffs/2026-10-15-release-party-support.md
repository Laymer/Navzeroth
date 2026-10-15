# Typed party support matrix

Owner: Codex `/root`; baseline `d59aa91`; claim `62be2d8`.
Branch `feat/release-party-support`, isolated worktree of the same task name.

Pure `release_party` schema version 1 defines supported party sizes and eight
required workflows for every declared size: joint planning, unequal progress,
sharing, stale members, fairness, agreement, advancement and review. A fixed size
bound (2–40) is an implementation bound, not a claim of supported target content.
The independently evidenced inventory must still enumerate the applicable sizes.

Rows carry a static procedure, supporting fact evidence and concrete cross-domain
catalog dependencies. Binding requires the exact catalog SHA256 and resolves every
identity to the verified non-fixture content source set. Local profile IDs and
synthetic scenario IDs have no special authority and fail absent real bindings.
Canonical row identities are `[catalog_sha256,"party_support",row_id]`.

Four synthetic tests pass; strict core all-target Clippy, formatting and whitespace
checks pass. Tests cover all workflows, missing/duplicate size pairs, absent content,
wrong catalog/build/schema, unknown fields, source unions and missing evidence.
These are declaration checks, not live/Windows or full-content evidence.

This commit deliberately leaves publication blocked: pack-v2 must explicitly list
and deliver the matrix, CLI must verify its file/evidence closure and actual
catalog identity set, and release inventory must match exact resulting row IDs.
The existing release validator is unchanged and still blocks party records.
No matrix declaring real support is shipped. PR6 is not complete until that
integration and the recorded acceptance scenarios exist. Next: PR4 pack/content
integration followed by dependency binding and reviewed real support enumeration.
