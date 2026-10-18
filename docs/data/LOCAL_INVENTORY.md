# Bound guide review and exact imported inventory

Continuation 7 adds research inventory format 1. Its nine ordered domains are
quests, equipment, acquisition, geography, travel, dungeons, class_roles, party
and reminders. It enumerates canonical exact imported identities, not estimates
or a claim to contain the whole game/slice.

## Identities and completeness

Client identities name the reproduced projection digest, record collection/table
and exact ID. Aggregate equipment records, raw mapped rows and unverified budget
models use separate namespaces. Permitted observation identities bind the guide
facts manifest, candidate index and recorded quest identity; false/unknown values
remain in the hash-bound observation object, not interpreted as absence.
Conflicts remain explicit gaps. Authored identities retain the existing canonical
content-record IDs and their catalog digest, labelled `authored_design`.

Every domain reports imported count, exact IDs, source availability and gaps.
An imported count of zero with an unavailable source is distinct from a projected
empty source. Independently required counts remain null and independently complete
remains false, including for domains with imported records. The existing release
inventory's independent enumeration/review process remains separate. Neither this
report nor a matching development digest can fill its proof fields.

Inventory binds the delivery tuple and optional verified authored pack's manifest
and catalog digests. Authored packs must be non-fixtures with exactly one original-
authoring source and matching edition, build, interface and locale. The existing
v2 pack verifier checks its source/material closure and party dependencies before
IDs are collected. Original authoring supplies procedures/models/checklists, not
gameplay eligibility, coordinates or verified proficiency.

No source paths are stored. To reverify an inventory that includes authoring,
reselect the same verified pack directory; a missing or changed pack refuses
closure verification. Existing immutable inventory bytes and active selections
remain intact. A raw authoring directory must first be built using the existing
`build-content-v2` command. The prior format's canonical bytes are unchanged;
VerifiedBundle additionally returns its verified manifest digest and party matrix
so consumers do not reopen unchecked files.

Limits: 250,000 exact identities, 64 MiB inventory bytes, 1 KiB per identity,
4,096 bounded gaps per domain. Private store quotas also apply. IDs are ordered
canonically; pages contain at most 200 records and bind the inventory revision.
Unsupported/unmapped domains remain explicit; catalog tables without supported
projections are not silently counted as reviewed gameplay records.

Commands prefixed with `azeroth-cli local-data`:

- `inventory-build --destination <store> --delivery <digest> [--authored-pack <verified-directory>] [--preset small|balanced|large|max]`
- `inventory-verify --destination <store> --inventory <digest> [--authored-pack <verified-directory>]` (all-nine-domain summary)
- `inventory-page --destination <store> --inventory <digest> --domain <domain> --offset <n> --limit <1-200> [--authored-pack <verified-directory>]`
- `route-designs --destination <store> --inventory <digest> [--authored-pack <verified-directory>]`

## Guide replay and original designs

`bound-guide-review --destination <store> --delivery <digest> --guide <selected-file>
--race <Race> --class <Class> --mode solo|duo` uses the delivery's reproduced
research guide-facts closure. It never executes Lua. The report binds guide hash,
delivery and facts manifest, retains all verifier findings, and never claims
evidence/acceptance. A clean research replay exits 3; findings/refusal exit 1.
Single-character replay cannot verify duo eligibility, fairness or shared credit.
The historical `guide verify` reader/behavior remains unchanged.

[Human route drafts](HUMAN_ROUTE_DRAFTS.md) contain twelve original review designs:
six declared classes in solo/duo modes, linked to existing authored role-model IDs.
The output reports whether those exact authored model records are present. No
verified quest sequence is produced: available observations/client identities do
not establish universal prerequisites, eligibility, rewards, XP or calibrated
travel. Route drafts remain explicitly unverified rather than fabricating those
facts. The launch class list and final target are still pending.

The owner's private generator was inspected as inert AST/text and not executed.
Its model/planner reference QuestieDB and scraped inputs, which remain inadmissible
under the existing local-source policy. No private source, generated route, quest
ID list or third-party facts are copied into Git. An exploratory replay against
empty synthetic research facts is [recorded separately](../handoffs/2026-10-18-private-guide-development-check.md);
it does not validate a gameplay route. Desktop selection/history presentation
continues in task 8; CLI and application-service adapters are available here.
