# Exact signed normalization report assignment

Owner/sole writer Codex `/root`; baselinebed6fad. The material selector validates
that raw normalization reports belong to accepted catalogs; the bundle source
gate previously checked approved FileRefs and unique attachment but not attachment
to the particular catalog revision. Native installation already rejected a wrong
assignment. A controlled signed-source-context regression demonstrated the source
gate accepted an approved, hashed report carrying a wrong revision.

Source verification now parses exact bounded report bytes and checks format,
schema, array structure, the particular pack revision and exact admitted origin
precedence before prebuild. Uses the same4MiB bound as native installation.
No historical schema, trust identity, data-source grant or acceptance status changes.
Meaningful regression covers valid report, wrong revision, precedence, schema and
non-array content. The initial regression failed before repair (wrong revision
accepted), then7bundle/staging tests passed. All-target CLI Clippy/fmt passed.

Portable LLVM CLI all targets:213passed0failed3existingignored. Modified source
gate266/272=97.79% executable LCOV lines. Initial coverage failed an inherited
hand-plan fixture on clean LFS checkout; exact-source fixture repairad5abe4e was
independently reviewed and cherry-picked asf8e55848 before the successful run.
No production byte validation was relaxed. Logs/tmp/navzeroth-normalization-
{before,after,clippy,coverage}.log; raw/tmp/navzeroth-normalization.lcov.

Independent review still required; no actual bundle/signature/installer/Windows
or publication result is claimed. Integrate this repair with the deterministic
preparation tool before final receipts and all protected source checks.
