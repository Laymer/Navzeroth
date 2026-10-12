# Predicate evaluator parity corpus (v1)

Synthetic engineering fixtures, not game data. Edition `wow_forever`, build and
interface `SYNTHETIC`, namespace `demo`, ids such as `char1`, `q1`, `item1`.
Nothing here is a real quest, item, map or weight.

The corpus pins the behavior of `azeroth_core::evaluation::evaluate` so that a
second evaluator (the addon's Lua module, planned for wave 2) can be checked for
parity (plan sections 6.7 and 18.3: sharing JSON names alone is not enough). Rust
is the reference. Every `expected` value was produced by running the Rust
evaluator and then reviewed against the intended three-valued meaning; if Rust
behavior looks wrong, the fixture records what Rust does and the concern goes to
the handoff, not to a silent fix here.

The Rust consumer is `crates/core/tests/predicate_parity.rs`:

    cargo test --locked -p azeroth-core --test predicate_parity -- --nocapture

## Files

`manifest.json` lists every case file with its declared `cases` and `lua_only`
counts and the corpus totals. The Rust test fails if a `*.json` file in this
directory is not listed, if a declared count is wrong, or if a case id repeats.

| File | Covers |
| --- | --- |
| operators.json | all 14 `PredicateNode` operators on a populated snapshot |
| truth-tables.json | three-valued all/any/not and exact short-circuit steps |
| unavailable.json | unknown/restricted/unsupported/not_applicable (CASE-010) |
| missing-history.json | absent quest history is Unknown, never False (CASE-001) |
| party.json | participant_state, stale and absent peers (CASE-002), party validation |
| freshness.json | stale reject vs allow_stale, uncertain/reversed clocks, same-session elapsed |
| limits.json | exact step, depth and node boundaries, generated deep and wide trees |
| context-mismatch.json | BUILD_MISMATCH incl. short-circuited branches (CASE-005/011/015) |
| equipment.json | slots, variants, modifiers, bytewise key order, number equality |
| inputs.json | rejected inputs, identifier and number edges, decoder strictness |

The `PredicateNode` enum has 14 operators (the task text said 15); the Rust test
asserts that all 14 appear in an evaluated predicate.

## Case file format

    {
      "description": "...",
      "schema_version": 1,
      "snapshots": { "<name>": <CharacterSnapshot JSON>, ... },
      "parties":   { "<name>": <PartySnapshot JSON>, ... },
      "cases": [ <case>, ... ]
    }

Snapshots and parties are shared by name within a file. They are decoded lazily,
per case, so a deliberately invalid snapshot only affects the cases that use it.

A case has these members (all required unless marked optional):

| Member | Meaning |
| --- | --- |
| `id` | unique across the whole corpus |
| `note` | optional, why the case exists |
| `case_ref` | optional, `CASE-001` and so on (plan section 18.2, synthetic engineering versions) |
| `predicate` | a predicate JSON value, or a generator (below) |
| `predicate_json` | instead of `predicate`: the exact JSON text to decode, for lexical cases |
| `predicate_json_trailing_spaces` | optional, `lua_only` byte-budget cases: spaces appended to `predicate_json` |
| `snapshot` | name of the current character snapshot |
| `party` | name of a party, or `null` |
| `freshness` | `{now_utc_ms: int or null, clock_trust: "trusted"/"uncertain", session: ["id", elapsed_ms] or null, max_age_ms}` |
| `stale_policy` | `"reject"` or `"allow_stale"` |
| `limits` | `{depth, nodes, steps}`, each within the evaluator defaults 32/1024/1024 |
| `lua_only` | optional, `true` for decoder-strictness cases the Rust test counts and skips |
| `expected` | one of the three forms below |

Exactly one of `predicate` and `predicate_json` is present.

### Expected outcomes

- Evaluation result: `{"truth": "true"|"false"|"unknown", "steps": N, "reasons": [...], "warnings": [...]}`,
  exactly the serialized `EvaluationResult`. `steps` and the order of `reasons`
  and `warnings` are part of the contract. Reason entries are
  `{"participant", "input": {"input": "field", "field": {...}} | {"input": "equipment"|"variant"|"modifiers", "slot"} | {"input": "participant"}, "reason": {"reason": "missing_record"|"stale"|"clock_uncertain"} | {"reason": "unavailable", "status": "unknown"|"restricted"|"unsupported"|"not_applicable"}}`.
- Evaluation error: `{"error": "INVALID_INPUT" | "BUILD_MISMATCH"}`. Only the code
  is compared, never a message. A step-limit overrun is an error, not Unknown.
- Rejected: `{"rejected": true}` when the predicate, snapshot or party fails to
  decode or validate before evaluation starts.

### Generators

A whole predicate may be `{"generate": {...}}`, which keeps depth and width cases
small and independent of the addon decoder's JSON nesting limit (a depth-32 rule
tree written out would exceed 32 JSON levels once wrapped in a case file):

- `{"kind": "nest", "wrapper": "not", "wrappers": N, "leaf": P}` wraps `P` in `N`
  `not` nodes. The rule depth is `N + 1`.
- `{"kind": "wide", "op": "all"|"any", "count": N, "leaf": P}` builds one `all` or
  `any` node with `N` copies of `P`. The node count is `N + 1`.

### lua_only cases

`lua_only` cases exercise behavior only the addon's stricter bounded JSON
decoder can reproduce, for example duplicate keys or deep nesting inside members
the closed rule language ignores, and the 32 KiB plan budget. Rust accepts these
inputs (checked when the corpus was written), so the Rust test cannot verify
them: it counts them against the manifest, requires their expectation to be
`{"rejected": true}`, and skips them. They stay unverified until the Lua half
runs them.

## Numbers and ordering traps

- JSON number lexemes matter. `22.0` is rejected for integer fields; `1.0` equals
  `1` and `-0.0` equals `0` as modifier values; `1e400` is rejected.
- Map iteration is bytewise (UTF-8) key order, never locale order or UTF-16
  order. `equipment.json` pins this through step counts and reason order.
- Identifier length counts UTF-8 bytes (1..256); Unicode White_Space at either
  edge and any control character (C0, DEL, C1) are rejected; U+FEFF and U+200B
  are accepted.

## Regenerating expectations

Authoring is by hand or script; the expectations must come from running Rust.
Set `PREDICATE_PARITY_DUMP=<path>` when running the test to write the actual
outcome of every executed case as JSON keyed by case id (it writes nothing
otherwise). Review the dump against the intended meaning before copying values
into a case file. Do not edit `crates/core/src` to make a fixture pass.

## For the Lua half (wave 2)

Decode this corpus with the addon decoder using an explicit larger depth and byte
budget for fixture files only (the addon default stays 32 levels and 32 KiB),
evaluate every case that is not `lua_only` and compare the normalized outcome
with deep equality, and run the `lua_only` cases against the decoder's strict
limits. Sort every map by an explicit bytewise comparator.
