# Verified active catalog imagery storage claim

Owner/sole writer: Codex /root/inventory. Baseline `908aef2` (PR5 frozen backend).
Branch/worktree: `feat/catalog-assets`, `/tmp/navzeroth-catalog-assets`.
Coordinator explicitly assigns a separate required Atlas storage task from the
user's full nine-domain release plan. Source worker owns active Atlas consumer/UI;
coordinator owns pack v2 validation/import closure; this writer owns storage and
migration only.

Implement immutable catalog/image/layer byte closure, full bounded PNG decoding,
hash/dimension checks, atomic bundle stage, exact active-revision image retrieval.
No arbitrary local image path reads; unknown/missing assets remain explicit.
Preserve v1 history and single atomic active pointer. Tests use synthetic PNGs,
never real licensed imagery or personal exports. Native/live imagery and actual
full-content Windows performance remain unverified.
