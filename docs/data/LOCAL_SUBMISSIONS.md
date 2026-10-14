# Local observation queue, schema 1

This device-local format records explicit declarations for review. It is separate
from exchange schema1/ANAV1 and adds no upload, accounts, hosting, game messaging
or automatic capture. Acceptance means local queue review; it never activates a
dataset, upgrades confidence, authenticates a source or verifies gameplay.

A declaration contains schema_version1, full context/build/namespace, fixture flag,
base_revision, registered source and evidence IDs, optional signed-safe UTC time,
and one closed payload. IDs must belong to the exact stored immutable revision;
the referenced quest/item must cite the registered evidence. No names, character
or session IDs, chat, paths, account data, free text or extension fields are accepted.

Payloads are quest_state (registered quest plus declared not_started/accepted/
completed/turned_in), price (registered item plus copper), or acquisition_outcome
(registered item, acquired, opportunities, successes, group_eligible and
personally_received). Every value explicitly reports known(value), unknown,
unsupported, restricted, stale or not_applicable. Known false and zero remain
values; absent opportunity counts never become zero. Declared successes cannot
exceed declared opportunities; the queue never computes drop probability.
Amounts/counts use checked nonnegative JavaScript-safe integers. Observation time
is a declaration, with no inferred trusted clock or freshness classification.

The maximum input/document is16KiB. Canonical typed compact JSON determines the
SHA256 row ID and fingerprint, independently of input whitespace/member order.
Schema/field changes require explicit format revision rather than changing this
identity silently. Explicit per-submit opt-in is required. At most1024 rows,
including erased tombstones, are retained; pages contain at most32 rows.

Pending declarations may be accepted/rejected locally. Pending, accepted and
rejected declarations may be withdrawn or revoked. Withdrawal erases the document;
a disclosed opaque fingerprint, base/source identity, timestamps and decision/CAS
revision remain to prevent duplicate recreation. Revocation is terminal; its
payload may subsequently be erased without removing the revoked state. Repeated
submission returns the existing record, including an erased tombstone, and never
resurrects data. Every decision checks the exact opaque ID/revision guard.
Local operation timestamps may be unknown, zero or signed-safe UTC; logical CAS
revision, rather than clock ordering, protects decisions.

Explicit export requires unchanged locally accepted state and its exact current
active eligible base. The source must record redistribution=Permitted, nonblank
license and attribution, the edition, exact verified_revision, the applicable
allowed_fields token (`local_observation.quest_state.v1`,
`local_observation.price.v1`, or `local_observation.acquisition_outcome.v1`) and
transformations token `local_observation_export.v1`. Unknown, Conditional and
NotPermitted never permit export. These are recorded policy declarations, not
independent legal authentication. No real permission is supplied by this feature.

Export is a maximum32KiB plain JSON document with format
`azeroth-local-observation`, schema_version1, explicit local-review/policy scope,
accepted row guard, content/policy hashes, disposition revision, recorded license/
attribution and the original declaration. It is handed back for manual local
inspection/copying. No filesystem publication or network action is performed.
Initial/final immediate SQLite read transactions surround bounded serialization;
final checks bind accepted CAS, active exact base, immutable hash, source policy
and disposition revision. Restrict/release/reactivate ABA invalidates a pending
export even when content bytes are identical. The final writer reservation spans
validation and handoff; no nested public transaction is introduced.

The committed synthetic-price example uses zero and the demo-r1 fixture. Its
ordinary registered source has unknown permission, so local review is possible
when that revision is staged, but export remains refused. Positive-policy test
fixtures are explicitly synthetic and confer no real source permission.
Native interactive and live client acceptance remain unverified.
