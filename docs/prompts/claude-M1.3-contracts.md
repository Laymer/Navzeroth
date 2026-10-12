# Claude Code prompt: M1.3 exchange contracts

Copy the following prompt into Claude Code in this repository or its GitHub clone.

```text
Implement M1.3 of Azeroth Navigator: snapshot, plan, evidence, dataset and gear
exchange contracts. Proceed through implementation, validation and handoff in
small, coherent commits.

Context: PR #3 is merged into main at
514d4b01cab34b4d85e8a9e05e5d5de926d6e4f1. M1.1 review/corrections, M1.6 Linux/Windows
CI and expanded M1.2 are complete for their scopes. All five CI checks passed on
that merge revision (GitHub Actions run 38241935794). M1.2 supplies all 19 domain
entities, validated Serde JSON, observations/freshness, structured errors and the
full bounded predicate whitelist. Its entity JSON is not the final exchange format.

Start by reading AGENTS.md, CLAUDE.md, docs/STATUS.md, the latest relevant handoff,
docs/handoffs/2026-10-12-M1.2-domain.md, docs/DOMAIN.md,
docs/reviews/2026-10-12-M1.2.md and docs/decisions/0001-foundation-sequence.md.
Read implementation-plan sections 6, M1, 18.3, 19 and 21. If the documentation
continuation branch is not yet merged, this prompt supplies the updated merge
state; the older draft-PR wording in main's records is historical.

Inspect the working tree and preserve unrelated changes. Fetch origin/main and
start a fresh feature/M1.3-contracts branch from its latest revision. Before
implementation, claim M1.3 as Claude Code in STATUS with the exact baseline,
expected files/subsystems and acceptance checks. Use one writer; do not spawn
subagents. Ask focused questions where a consequential decision is unresolved;
continue independent work while awaiting answers. Resolve routine choices using
the plan and existing conventions.

Deliver:
1. Complete versioned exchange envelopes for all five contract families, with
   explicit producer, build/edition, namespace, provenance and dependency metadata
   as appropriate. Cover the plan fields required by section 6.5. Document absent
   fields, defaults, reference-resolution boundaries and compatibility behavior.
2. Bounded structural and semantic validation reusing the existing validated
   domain types. Reject unsupported schema versions, invalid known/value states,
   unknown operators and incompatible contexts. Preserve accepted additive
   metadata. Keep schema, dataset and application versions independent.
3. Generated JSON schemas and TypeScript types from Rust where practical. Check
   custom Serde representations and domain invariants explicitly: generation alone
   does not prove equivalence. If generation is unsuitable, document the reason
   and verify explicit schemas/DTOs against the same fixtures, as section 6.7 allows.
4. Focused synthetic positive/negative fixtures, round trips, boundary checks and
   reproducible generation/drift detection in the existing Linux/Windows CI. Verify
   actual TypeScript handling. Document which Lua compatibility/parity checks are
   implemented or deferred; shared field names alone do not establish parity.

Preserve unknown, unsupported, restricted, not-applicable, stale, false and zero
distinctly; per-field provenance and each participant's timestamps; safe numeric
ranges, string IDs/copper and explicit coordinate spaces; and demo/reference and
edition/build separation. Imports are bounded data, never executable Lua. Keep
core logic pure. Use synthetic data only; do not invent game evidence or add a
desktop live-state bridge, memory access, injection or input automation.

Keep scope on M1.3 and its necessary contract tests. Leave substantial UI, transport,
storage, route search, gear scoring and broader M1.5 fixture work to their tasks.
Correct any reproducible domain inconsistency that blocks these contracts in its
own small commit and explain it; avoid unrelated refactors.

Run the documented checks, including cargo fmt --all --check,
cargo clippy --locked --workspace --all-targets -- -D warnings,
cargo test --locked --workspace, pnpm install --frozen-lockfile and the new contract
checks. Document exact generation/check commands. Check generated output is
deterministic and CI detects drift. Report actual results and limitations; native
desktop and live-client behavior remain unverified without their environments.

Use M1.3-prefixed commits and the configured Git author. Update STATUS and write a
handoff with baseline/head, decisions, commands/results, remaining risks and the
next dependency-ready task. Fetch main before publication and preserve published
history. Push the feature branch and create a draft PR under the standing project
authorization. Do not push main, merge or release. Finish with the PR link, commit
summary, verification results and review focus.
```
