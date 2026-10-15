# Reference content v2 contract handoff

Owner: Codex `/root/source_admission`; baseline `d59aa91`; claim `d6c10f1`.
Branch/worktree: `feat/reference-content-v2` / `/tmp/navzeroth-reference-content-v2`.
Coordinator approved separate v2 contract and nested unchanged v1 reference.

Implemented `content_v2::ReferenceContent` and `decode(bytes)`, strict nested
unknown/duplicate member refusal, bounded parsing and typed extension graph.
Collections: models, equipment, quest_facts, acquisition_facts and dungeons.
Reference envelope is the sole context/revision/fixture authority. Models hold
actual validated RoleProfile/ProficiencyTable/SlotLayout values, applicability,
class and level interval; explicit source provenance resolves into base evidence.
Quest facts add access requirements and exclusive branches with a required
ChoiceTaken trigger. Quantity, false, zero and unavailable observations are kept
as authored. Dungeon predicates, checklist evidence, travel edges, preparation
quests and rewards resolve across the complete base graph. Conflicted evidence
is refused in both directions. Acquisitions use an indexed lookup. No live facts
or rights are inferred. Party matrix ownership transferred to root/release layer.

Checks: seven focused `cargo test --locked -p azeroth-core --test content_v2`
tests passed; strict all-target core Clippy, fmt and whitespace checks passed.
Initial compile/test errors (missing NodeRef import, test borrow, numeric Copper
fixture instead of string) were fixed and are not counted as successful checks.
Tests cover valid synthetic graph, unknown/restricted/false/zero distinctions,
wrong versions, byte limits, missing/duplicate/conflicted provenance, cross-domain
refs, mixed context, undocumented model sources, invalid reset and strict nested
fields/duplicate keys/explicit-empty imagery defaults. No Windows/live checks run.

Important limitations: this is PR4's core contract portion only. Root owns pack
v2 and release binding, acceptance worker owns app consumers, inventory worker
owns PR5 capacity/index/migration. Nested v1 currently retains 1,024 collection
limits; PR5 will introduce a separate validated catalog payload, preserving v1
bounds/hashes. Outer intermediate byte ceiling is 16 MiB until that work. Optional
null forms omitted by canonical serialization are refused by strict subset; known
empty imagery arrays and empty dependency declaration are explicitly supported.
No generated public JSON schema artifact is supplied in this foundation. No
production corpus, rights admission, desktop integration or full PR4 completion
is claimed. See [design](../data/REFERENCE_CONTENT_V2.md).

Next: peer review exact contract, integrate downstream consumers/pack/typed release
identities, provide schema and full-capacity graph validation, then test combined
behavior. Release remains blocked on complete licensed corpus and real acceptance.

Schema follow-up adds JsonSchema derives for reused pure gear/access types,
`content_v2::schema`, a deterministic generated v2 schema and a synthetic fixture.
Strict Ajv2020 validates the fixture and rejects six nested/version/availability
cases. Eight focused core tests pass (one explicit fixture generator ignored);
existing v1 schema drift tests still pass. Schema generation closes fixed nested
records while preserving typed maps; duplicate JSON keys remain Rust's boundary.
`node scripts/validate-content-v2-schema.mjs` is the repeatable structural check.
The earlier missing-generated-schema limitation is resolved by this follow-up;
full-capacity and app/pack integration limitations remain.

Semantic follow-up rejects known reset facts contradicting the base quest's
completion semantics and acquisition origins contradicting the base source kind.
Nine focused tests now pass (one generator ignored); strict Clippy/fmt/diff pass.
This adds no fields or format changes. Both base and extension observations may
remain unknown; absence is never substituted with a compatible factual default.
