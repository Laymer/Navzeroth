# Synthetic Lua gear scoring corpus (R11)

This independent manifest covers 10 synthetic catalogs, 32 selected-set score
comparisons and 157 arithmetic vectors. It is not game data or a registered
exchange schema. Build/interface are SYNTHETIC, namespace demo, fixture true;
weights have the synthetic_test source. No equipment observation is claimed.

`cases.json` stores raw Rust profile/candidates and caller-selected sets,
compiled scoring-only DTOs, sorted-catalog indices, and Rust-anchored expected
score breakdowns/order. `arithmetic.json` pins value-to-thousandths conversion
and exact multiply/divide/round behavior. Deterministic extra vectors exercise
large positive/negative operands and weights around the scale; no random code
runs in the addon or test.

Regenerate deliberately with:

```
UPDATE_LUA_GEAR_FIXTURES=1 cargo test --locked -p azeroth-core --test gear_lua
```

Normal tests never write fixture files. Rust tests verify the compiled DTO,
components, statuses, certainty, errors and ordering unchanged. Two tie fixtures
also compare against the existing rank_theoretical configuration implementation.
The Lua harness consumes each compiled catalog with production decoder limits
and compares every contribution, total, certainty reason/effect and selected-set
ordering. It also runs all arithmetic vectors, including safe integer boundaries,
negative halves, per-item rounding and out-of-range errors.

The compiler converts raw attributes to integer thousandths once using the
existing Rust rule. Lua's lightweight scorer uses exact base-1000 integer limbs
for multiplication; a direct double product is insufficient at the boundary.
The standalone Lua toFixed helper is separately compared with Rust's initial
IEEE-754 multiplication/rounding rule.

Selections are scoring inputs, not equipment recommendations. Lua does not
search configurations or establish their legality, acquisition or eligibility.
The caller supplies sets; ordering is total descending, certainty ascending,
then sorted full item keys, consistent with Rust. Automatic live reevaluation,
real weights/catalogs and client equipment events remain unverified/future work.
