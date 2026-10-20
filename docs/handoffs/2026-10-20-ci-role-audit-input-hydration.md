# Exact role-audit test inputs in Rust CI

Owner/sole writer `/root/tooling_review`; baseline1702e387 (sourcefix04 plus
world's documentation-only role-audit investigation); claimf262c8e5.
Isolated fix/ci-role-audit-input-hydration. Code5a915d3e is frozen for root review.

Failure39386917359 remains a failed Linux/Windows CI run: expected quest SHA256
f5a0f006... but observed285c609b..., the exact133byte Git LFS pointer. World proved
audit and canonical authored regeneration are byte-identical; no game data needed
correction. None of the four authored_slice assertions or the report, retained22
code files, sources/admission/packs or generator outputs were edited here.

New scripts/hydrate-role-audit-inputs.py uses existing strict parity primitives.
Its fixed allowlist is ONLY the historical CMaNGOS quest part0000 and run-manifest;
no caller-selected path, wildcard, imagery, full corpus or latest data. Both inputs
must have canonical HEAD LFS pointer OID/size, individually at most8MiB. Working
files must already match verified bytes or the exact HEAD pointer, with regular
file/ancestor checks. The pinned role report must name that exact quest pin before
fetch. Restore only the manifest first, verify its hash and exactly-one quest
registration against the report/HEAD pin, then restore the quest if necessary.
Postchecks repeat exact resource identities, provenance/count, manifest and role
report identity and unchanged Git HEAD/pins. Integrity failures never become
acceptance. No adversarial multi-process filesystem sandbox guarantee is claimed.

Normal CI Linux/Windows Rust jobs and the repeated stability job run this helper
before workspace tests, with a five-minute step bound. lfs:false stays explicit;
only these immutable existing Git LFS objects may be fetched. The standalone Rust
coverage collector also runs the same helper before workspace integration tests,
with a240second process deadline and retained collector log. Existing Git calls
are bounded to120seconds; --offline uses local checkout only. The optional manual
non-SkipTests Build-Windows path is unchanged; callers may explicitly run the
helper first. CI packaging already uses SkipTests and runs Rust CI separately.

Actual checks:

-11 synthetic tests pass, including real temporary Git/LFS pointer checkout→local
  cache hydration, idempotence/no fetch, wrong report/path/pointer/size/working
  bytes, bad/duplicate manifest preventing quest fetch, missing/corrupted restore,
  post-restore manifest mutation, linked/oversized files and CI/collector order.
- Own actual two-file checkout rehearsal replaced ONLY these two owned worktree
  files with their exact HEAD pointers, ran --offline and confirmed original
  bytes fully restored from existing local LFS objects. Both inputs independently
  match audit/manifest pins: quest2026036bytes, run-manifest12069bytes.
  [Actual offline receipt](../data/ci-role-audit-offline-hydration-v1.json).
- Correct unchanged CLI authored_slice target:4passed. No synthetic pointer hash
  was substituted, no assertion dropped and no audit regenerated.
- actionlint1.7.12 on CI/coverage workflows, collector JavaScript syntax and diff
  checks pass. No full native coverage or Windows workflow execution is claimed.

No broad LFS pull, latest source acquisition, permission/grant change, signing,
push, purchase, quota expansion, manual smoke or release-readiness pass occurred.
Next: root independent review; normally combine with sourcefix before PR232 fresh
all-ten checks and actual source-platform parity. Missing Windows/coverage/stability
checks remain unverified until those real workflows run successfully.
