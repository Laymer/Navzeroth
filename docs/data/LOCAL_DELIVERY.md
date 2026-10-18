# Bound research delivery and future release receipts

Continuation 6 adds private delivery format 1 and policy/receipt format 3.
Historical policy/receipt formats 1 and 2 retain their 70374 target, domain tags
and original signer rules. The existing release configuration is untouched.

## Immutable research delivery

A delivery binds the exact catalog/product/version/build/selected locale,
reproduced projection and projector profile, selected media manifests, input
manifests, optional guide facts and dependency/profile hashes. Verification
reproduces every selected closure and compares canonical bytes. Guide facts must
bind exactly the selected projection and observation inputs; cross-catalog,
cross-locale, stale profile and substituted media/projection/dependencies refuse.
Synthetic inputs/media and catalogs lacking source hashes remain marked fixtures.
This marker is provenance, not independent proof of actual-client extraction.

The dependency profile closes over first-party analyzer/core/ingest/storage
sources and participating manifests/lockfile. A changed profile requires new
research artifacts; it does not silently relabel old projections as reproduced.
Historic catalog/projection readers remain available with their explicit trust
limits. A refused new candidate does not erase an existing active revision.

CLI commands, each prefixed with `azeroth-cli local-data`:

- `delivery-build --destination <store> --projection <digest> [--media <sorted-comma-separated-digests>] [--inputs <sorted-comma-separated-digests>] [--guide-facts <digest>] [--preset small|balanced|large|max]`
- `delivery-verify --destination <store> --delivery <digest>`
- `delivery-page --destination <store> --delivery <digest> --domain <domain-or-table> --offset <n> --limit <1-200>`
- `delivery-diff --destination <store> --before <digest> --after <digest>`
- `delivery-active --destination <store> --delivery <digest>`
- `delivery-activate` or `delivery-rollback`, with the destination, delivery digest and `--expected <generation>`

Select at most 16 media and 16 input digests, sorted and unique. Delivery manifests
are bounded to 1 MiB and use immutable private `deliveries/<digest>` objects.
Lookup serializes only the requested typed slice, retaining validated ID order.
It reports source states/reasons and gaps; unavailable counts are null, while a
projected justified empty table reports zero. Imported counts remain separate
from independently reviewed completeness.

Private lifecycle SQLite schema 2 adds a separate delivery activation table.
Migration preserves schema-1 selections, jobs and catalog history. Activation is
transactional compare-and-swap; rollback requires a previously active digest in
the same product/version/build/locale scope. Closure validation precedes the
transaction under the writer lock. A failed candidate, stale generation or
rollback refusal leaves the previous selection unchanged. Older writers refuse
the newer private index; immutable artifacts are not rewritten.

## Guarded desktop lookup

The local-data screen can prepare a projection-bound reference using an explicit
installed locale, inspect partial facts, review changed bindings, select a
research reference and restore a reviewed prior revision. It rejects stale and
mismatched page responses. Previously selected revisions can currently be pasted
from a receipt; a full durable history picker and media/input presentation follow
in continuation 8. Desktop preparation currently selects only the projection;
CLI delivery building supports the additional explicit bindings.

The embedded `config/release-delivery-v3.pending.json` has a null target, no
trusted signers and no expected development digests. The indicator is therefore
unconfigured. A reviewed future policy can mark an exact tuple/delivery digest
matched or different. Even a match leaves `actionable_advice` and
`acceptance_ready` false. Research activation never injects these facts into
planning datasets. Relevant build-bound availability evidence, freshness and
normal domain rules remain mandatory for actionable advice; this continuation
obtained no such evidence.

## Policy and receipt format 3

The new policy supports a pending null target and a syntactically validated
future frozen target, plus explicit independent trusted operator/reviewer keys
and development expected digests. Duplicate identities/keys and unknown versions
are refused. Receipt v3 binds its canonical delivery, candidate commit, production
hash, policy hash, target, independent review, scenario/validation/performance
hashes, all nine independently reviewed inventory proofs and mandatory live-game
and interactive-Windows acceptance proofs. Imported counts cannot fill these
proofs. Fixtures cannot be signed. Signatures use a separate V3 domain tag;
revoked, duplicate, reordered or substituted signers refuse.

CLI steps use explicit bounded files only:

- `delivery-receipt-sign --payload <file> --policy <file> --key-id <id> --secret-key <file>`
- `delivery-receipt-assemble --payload <file> --operator <file> --reviewer <file>` (canonical bytes on stdout)
- `delivery-receipt-verify --receipt <file> --policy <file> --candidate <commit> --production <digest> --delivery <digest>`

Verification uses the repository's independently frozen production target, never
a receipt-chosen target. `release::frozen_release_target()` is still None, so
current-release V3 verification refuses. Synthetic future-target unit tests call
the lower-level verifier with an explicit invented trusted target; they are not
operator or release evidence. No keys are generated, discovered or automatically
used. The stable/acceptance-candidate gates and legacy release paths remain strict.
