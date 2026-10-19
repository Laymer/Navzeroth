# PR212 hand-plan test fixture scope

Owner/sole writer Codex `/root`; baseline359fd4a3850679c65f16ac1ff9e74df1db4e5540.
Rust Linux39324324502 failed because the hand-plan offline test retains all
non-gen sources, including newly admitted world originals that ordinary CI leaves
as LFS pointers. Scope that fixture to exact hand-plan source IDs, preserving its
1685 real licensed rows, inert PATH and byte/tamper tests. Actual world closure is
separately checked on both platforms. No production permission or gate change.
