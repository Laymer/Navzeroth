# STABLE-BUNDLE-RELEASE-BINDING

Owner/sole writer Codex `/root`; baseline and scope in the corresponding claim.
Isolated branch `feat/stable-bundle-release-binding`, built on native084b76b,
schema4 acceptance and corresponding-source material contracts.

Schema4 requires explicit bundle manifest, receipt and public review FileRefs.
Historical schemas reject the new field even when null; historical material and
private hybrid-receipt contracts remain separate. Manifest and receipt live under
release/ so their exact bytes do not create a production fingerprint cycle.
The trusted signer policy remains compiled production input. Source verification
requires tracked exact resources, actual candidate production identity and tested
ancestor, inventory target/pack revisions, accepted-origin policy, public review,
and every independently verified selected material. Private admission registries
and undeclared private validation evidence are refused as bundle extras.

`assemble-bundled-content --root REPO --output NEWDIR --identity FILE` selects
and verifies source materials before copying signed resources to a temporary
sibling directory. The output's parent must exist. Checked file descriptors stream
hashes and copies, failed creations are removed, existing outputs are preserved,
and the assembled tree is verified before publication by rename. It reports
publication_allowed:false. It never signs, activates or clears acceptance checks.
The native build's expected identity is an explicit canonical receipt Payload;
final release path is release/v1/bundled-content-build-identity.json.

The hosted installed-tree verifier now includes the dynamic bundle manifest,
receipt and every declared source-pinned resource. Hashing is streamed with size,
handle and path checks; linked components, extras, missing files and altered bytes
are refused. This Node verifier establishes byte closure, with the Rust source
receipt verifier establishing trusted cryptography separately. No ancestor-race
sandbox is claimed.

Actual targeted checks:15 core release-v3 tests;5 source binding/staging/copy
unit tests;2 CLI bundle commands;2 Node installed-resource tests. All passed.
All-target core/CLI/local-data Clippy with warnings denied and formatting passed.
An inherited future-schema test incorrectly used newly accepted schema4 as a
future version; corrected to SCHEMA_STABLE+1. An initial full coverage run failed
that assertion and was not a passing run. Coverage rerun is recorded below.
Shared synthetic fixture names now include module identity to avoid collisions
when used by independent test modules in the same binary. Synthetic keys are only
in private test helpers; production commands use compiled public trust identities.

No actual content receipt was issued, no installer was assembled from final
release content, and no hosted or manual Windows acceptance result is claimed.
Independent review, integration with genuine inventory/normalization conflict
contracts, exact-source checks, final signed content, hosted Windows measurements
and exact draft installer owner's smoke remain required.

Final portable LLVM run:799passed,0failed,5existingignored across CLI/core/local-data
all targets. LCOV executable lines excluding existing cfg(test) ranges:
release962/1064=90.41%, sourcebundle227/232=97.84%, staging90/96=93.75%,
bundled-resource122/131=93.13%, CLIbundle77/80=96.25%, main359/397=90.43%,
core release798/808=98.76%, selected materials395/415=95.18%.
No coverage threshold or production trust policy was relaxed. Cohesive source
assembly now uses the same verified Context as source receipt validation; private
synthetic helper tests exercise the complete source-binding/copy path and ensure
production trust rejects those keys. Final targeted source/assembly suite6passed.
Logs /tmp/navzeroth-release-binding-{coverage,tests,cli-tests,clippy}.log;
raw LCOV /tmp/navzeroth-release-binding.lcov.

Hosted harness integration additionally needs native coverage collector adaptation:
coverage-built stable native application must receive the exact canonical build
identity and verified staged bundle before startup, without a synthetic bypass.
