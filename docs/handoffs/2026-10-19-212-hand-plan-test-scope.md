# PR212 hand-plan source fixture repair

Owner/sole writer Codex `/root`; baseline359fd4a. Rust Linux39324324502,
job118306112944, failed committed_licensed_materials_build_without_network_on_every_platform
with Cached or retrieved source differs from admitted bytes. Local clean
GIT_LFS_SKIP_SMUDGE worktree reproduced it during a separate coverage run.

The old test keeps all non-gen registry rows. Newly admitted world originals are
also non-gen; ordinary CI correctly leaves them as LFS pointers, and they are not
inputs to this test's hand-written source build plan. Scope the fixture registry
to exact source IDs consumed by its filtered hand plan. Production source-sync,
admission, source permissions and byte closure remain unchanged. Actual world
originals are independently verified by the hosted Linux/Windows parity workflow.

Actual source_pipeline integration suite:9passed0failed, including1685 real
licensed rows with empty PATH, deterministic bytes, false/zero, missing/tampered/
unlicensed refusal and output preservation. Formatting passed. This repair does
not reclassify failed39324324502 or the failed coverage run as success; fresh
all-ten protected-main checks are required on refreshed212. Native/Windows results
for this repair remain unverified. Log/tmp/navzeroth-212-hand-plan-test.log.
