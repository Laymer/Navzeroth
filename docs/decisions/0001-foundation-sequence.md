# ADR 0001: review, CI, and the domain foundation

Date: 2026-10-12
Status: accepted by the owner during planning
Tasks: M1.1 review, M1.6, M1.2; follow-up M1.3

The owner selected Linux and Windows CI before new domain logic, then expanded
M1.2 to all 19 section 6.1 entities, validated Serde JSON, pure freshness checks,
and the entire section 6.6 predicate whitelist. These are data models and a
bounded pure evaluator, not route search, gear scoring, import, storage, or UI.

Alternatives considered: M1.2 before CI; primitives only; Rust-only types;
core boolean predicates only. The selected scope exercises domain assumptions
early, at the cost of a larger foundation change. M1.3 still defines full exchange
envelopes and generated schemas/types; M1.4/M1.5 extend CI as their targets exist.

Freshness thresholds are caller supplied. Stale known inputs require explicit
opt-in and warnings. Unknown, restricted, unsupported, false and zero remain
distinct. Predicate limits default to depth 32 and 1,024 nodes/steps; callers may
lower them. Compatible additive JSON fields are accepted; invalid values and
unknown schema versions/operators are rejected.

Delivery: one branch and draft PR, small task-prefixed commits authored with the
configured Laymer identity. One writer; no delegated agents. The owner authorized
requiring passing up-to-date CI checks on main, including for administrators,
without mandatory peer approval. Main merging and releases remain separate.

The owner also authorized routine feature-branch pushes and draft PRs by default
for future sessions. AGENTS.md is the canonical authorization policy. Published
history is preserved; later main updates do not require rebasing published work.
