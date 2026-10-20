# Recovery runtime repair composition

Coordinator/root reviewed integrated commit d636c539ef90fed43d237d04bbacc1ab74071ad7.
Production fingerprint a3baa49678a3492b9773ede35988a31cfcb2818bd39a341c6eb74182938b3808,
3129 entries. No final installer acceptance is implied by these engineering checks.

- PR261 final1b89bd21, code1b8a7b02, production12132a7c; merge484fbab8.
  Desktop authored; world and root independently cleared bounded progress/watchdog,
  fail-closed cleanup and unchanged time/acceptance rules. Root actual54Node and26PS7
  checks passed; world46Node/26PS7. Desktop nativePS5.1 actual25 passed, with the
  PS7-only real parent-kill case explicitly unexecuted on5.1. No recovery pass claim.
- PR262 finalabf142d7/code8730bbc8; merge5ca9a475. World authored; root/tooling
  independently cleared; desktop independently cleared integrated semantics.
  Empty-plan reconstruction retains first full historical validation and avoids
  only the identical second scan with no intervening mutation. Guarded plans still
  demote recovered provenance and undergo the second scan. Destructive-boundary
  validation remains. Root independently executed matched author binaries:51storage,
  13host and1guarded-plan recovery tests, all passed. Tooling independently7+1;
  its attempted shared counter binary no longer contained that test and returned
  zero selected tests, which was NOT counted as a pass. Root's earlier execution
  included the actual counter test before that shared binary was replaced.
  Root independently derived current-source production LCOV414/449=92.2049%, excluding
  cfg(test) and all other worktree paths. Raw LLVM482/525 is not the gate metric.
- PR263 final2d851bf3/codeb3dfe191; merged at d636c539. Desktop authored; root/tooling
  independently cleared strict borrowed-value decoding and consuming owned catalog
  canonicalization. Bounds, unknown/duplicate/optional rules, provenance/conflicts,
  canonical bytes/hashes and borrowed caller ownership remain. Root and tooling each
  independently executed matched author binaries:21core pass/1ignored,10catalog pass,
  2owned-capacity pass. Author affected suites663pass/2ignored, Clippy/fmt pass.
  Root independently checked the retained pre-change SQLite oracle:13677 bytes,
  document223f8539f445e4edb759e87dcd4a17f0e3cf6a26f1b59f0e0fbb7ff29451acf0,
  content10d068a2aea2fdad6c31a49e62cf1edd2928fd717e1d572f52b5f760b92db95e,
  coverage8181ce0eddfc80940a44598c13ed713dee1ec7ccb0dc1835065607d52a43647f.
  Root independently derived exact current-source production LCOV43/43 core and
  350/372 storage=94.0860%, excluding inline tests and old cached worktree paths.

Independent executions above used matched author-built test binaries, not separate
compiler builds. The separate optimized comparison genuinely rebuilds runtime code.
Desktop's exact integrated union review independently confirms262 guard semantics
and exact261/263 integration without assertion, default, UI, content or time-limit
changes. Its263 authorship is distinct from root/tooling independent approval.

Historical V13 failure and signed/functional inputs were preserved before repairs.
The first baseline archive commit omitted globally ignored .log files; the immediate
normal child33cbe58e explicitly added them and root verified all five FileRefs and
Git blobs. Original trailing blank lines are preserved as raw evidence; no blanket
whitespace-pass claim is made for these logs. No published history was rewritten.

Next: actual rebuilt full-pack comparison and functional proof, renewed independent
public review and signed content receipts, then fresh exact-candidate hosted gates.
