# Independent world-data and release-gate review of PR #212

Reviewer: Codex `/root/world_review`, read-only in the existing datagen worktree.
Checked head: `5d5fef9b9e2c2fcb7b124966b1c1afed4844127f`.
Compared main: `826aa21862f7ef15d290c453d5bab3caa7806474`.
Review precedes the separately claimed STABLE-WORLD repair.

## Findings

1. **P1: mutable revision identity.** `world_to_content.py` derived revision from
   shard digest and target build alone. With the existing synthetic `Run` helper,
   changing equipment's admitted origins changed item count from eight to seven
   and changed catalog bytes while retaining the same revision. Policy, authored
   inputs, remaining target context and mapping output must affect the identity.
2. **P1: source/material closure is absent.** Generated `sources.json` gives all
   sources unknown verified revisions; explanatory license strings differ from
   exact admission license identifiers. The upstream CMaNGOS/wago admission rows
   have no admitted files, and `navzeroth-world-content` is absent from admission.
   Registered `gen-world-*` facts are separate identities. Release binding checks
   in `cli/src/release/admission.rs` correctly refuse these mismatches. The pipeline
   verifies an ephemeral pack then gates an inventory containing no packs.
   Actual `release-materials --root . --version 1.0.0-rc.1` returned empty packs,
   sources and files. Correct admission, materials and signed receipts remain
   mandatory; weakening these checks is not a repair.
3. **P1: historical same-edition build restamping.** Five wago evidence records
   claim structured build 1.60.1.70450 although their notes identify source build
   1.60.1.70245. Their context is the dataset context, not a true-origin field.
   The release classifier checked foreign editions only. A temporary standalone
   harness linked against existing branch Rust artifacts classified the real pack
   as 70,420 accepted origins, 273,929 accepted unknowns, zero remaining gaps and
   zero relabelled evidence. Preserve origin build separately and do not infer a
   grant for historical target-edition builds from the accepted 1.12/Classic grants.
4. **Bounded input prerequisite.** The standalone converter's recursive JSON
   loader uses unrestricted `read_bytes` and `json.loads`. Pipeline verification
   precedes conversion but the documented standalone command bypasses it. The
   STABLE-TOOLING bounded reader must be integrated in a separate reviewed change.

## Other observations

Cross-edition precedence and conflicts are explicit. The report has 5,132
historical item fallbacks and 2,801 disagreements. Authored merge support is
narrow, primarily dungeon title matching, rather than a general authored fact
merge. The real catalog has no dangling quest giver/ender references and no
dangling NPC/object/item loot-source references; the earlier handoff warning is
stale for this head. Unknown non-entity sources remain distinct.

The accepted historical origins, accepted unknowns, inferred travel and removal
of live-client/build-freeze gates are authorized policy, not findings. No signed
content-receipt closure is generated here. Hosted Windows measurements, installed
resource closure, content admission and candidate-bound evidence remain release
prerequisites. No live-client or ordinary-PC performance pass is inferred.

## Checks actually run during review

- Python offline suite: 125 tests passed.
- Existing branch debug CLI `verify-world-data`: 35 files, 475,355 records,
  manifest checked, zero errors.
- Real-pack classification and revision-collision reproduction described above.
- Actual release material selection: empty, as described above.
- Repository working tree remained clean.

Source-parity run 39307501965 for refreshed #214 head
`7ee59f543ae53b7282274f38e0cf07fe1caba13b` was still provisioning dependencies
on both platforms when inspected; this review does not claim its success.
