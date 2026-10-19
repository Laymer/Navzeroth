# STABLE-NATIVE-COVERAGE

Owner/sole writer `/root/tooling_review`; baseline ab16316 plus normal merge
3787cad at8b66bbd. Merge conflicts retained schema4 corresponding-source and
bundle evidence additions; no historical contract removed.

Stable1.0.0 Windows native coverage now calls the production trusted CLI assembler
with explicit release/v1/bundled-content-build-identity.json. Missing identity or
source/receipt verification failure aborts collection. Raw bounded single-line
identity is passed only to the native build environment. Inherited identity is
refused. After build, resources are copied to the exact executable's sibling
bundled-content directory (Tauri unbundled resource_dir), then verified by the
compiled-trust CLI again. Existing destinations are refused. No synthetic keys,
receipt-selected expected identity, source readiness override or startup bypass.
RC behavior remains unchanged. Stable startup has20minute readiness and30minute
process bounds; actual performance remains unmeasured here. Source fingerprint
and clean-source collection checks and existing90% gates remain mandatory.

Validation: new mockable adapter test passed (100% lines/branches/functions),
JavaScript syntax and git diff checks passed. Production CLI operations are mocked
in this portable test; it proves invocation/placement/failures rather than actual
cryptography. Actual Windows native coverage collection, final content assembly,
profile closure and changed-file native coverage remain UNVERIFIED and required.
No actual keys or signatures created. No Windows build/start performed.

Independent binding review: root3787cad has no remaining blocker after repairing
parsed-manifest reopen. Controlled regression demonstrated undeclared installed
resource acceptance at34d1b1a through fs.readFileSync injection, then fixed Node
suite2passed independently on3787cad. Rust binding/staging6tests independently
passed (Rust unchanged by fix). Concurrent empty-directory final rename limitation
is explicitly documented, no adversarial no-clobber guarantee claimed.

Next: independently review adapter and merge into final source, issue actual
receipts only after final review, collect native Windows coverage with that exact
verified content, run hosted8cases and exact draft owner smoke before publishing.
