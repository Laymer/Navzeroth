# Continuation 5 — private media and bounded tooltips

Owner/sole writer: Codex `/root`; 2026-10-18. Initial baseline `8b4c249`,
continuation-4 PR #194; integrate resulting main before PR. Claim:
[media/tooltips](../claims/2026-10-18-media-tooltips.md).

Implemented exact-layout map/icon candidate discovery, isolated file-ID-only
archive reads with full catalog source fingerprints before/after, bounded BLP2
palette/BGRA/BC1/BC2/BC3 top-mip decoding, deterministic private PNGs, versioned
media manifests, explicit read/decode provenance, revision-bound pages and opaque
PNG serving IDs. CLI/app adapters share the private pipeline; desktop native
entry recognizes the isolated worker argument. No automatic activation.

Tooltip interpolation is inert plain text with an allowlist, explicit missing/
unsupported outcomes, dependency cycle detection and bounded expansion. Only the
requested description closure is loaded, so unrelated catalog descriptions do
not exhaust the evaluation document quota. Projection rereads are hash checked.
[Supported formats, limits, commands and gaps](../data/LOCAL_MEDIA.md).

Actual checks: workspace Rust excluding desktop passed 1,290 tests (four ignored,
178 result suites) before the final closure-loading helper. After that helper,
46 affected tests passed; after the large-catalog regression, all 22 projection
integration tests passed. Workspace Clippy with warnings denied and formatting
passed. No TypeScript changes. Native desktop compilation is left to hosted CI.
Synthetic cases cover all supported pixel encodings/alpha modes, opaque BC1
endpoint orderings, block placement, mip range/length errors, resource quotas,
research trust flags, scalar zero/unknown, token injection refusal, dependencies,
cycles, large-catalog closure selection, immutable determinism, substitutions,
opaque revision separation, CLI equivalence and native worker refusal.

Two independent read-only reviewers found three issues: BC1 opaque three-color
handling; tooltip reread hash binding; source-free native refusal rows rejected by
verification. Fixed with pixel regressions, reread hash/validation and explicit
ReadRefusal provenance plus manifest/verification regressions. Both reviewers
confirmed resolved; no remaining actionable findings in their reviewed scopes.
Final hosted exact-head checks/merge receipt remain pending.

Unverified: ordinary Windows media extraction, actual client pixel correctness,
map/floor transforms and calibration, rights for redistribution, live tooltip
amounts and interactive desktop display. Media stays local-only and unavailable
for planning. Native read refusals are recorded outcomes, not re-proven file
contents. Full-image fixtures are synthetic; no private client images are retained
in Git. The owner can keep playing; this task did not reopen mutable caches.

Next: continuation 6, versioned delivery/profile/dependency bindings and guarded
consumers, explicit diff/activation/rollback and development match indicators.
Preserve historical policy/receipt formats 1/2 and their original 70374/signers.
