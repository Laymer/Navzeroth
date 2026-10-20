# STABLE-PACK-READER handoff

Owner/sole writer: `/root/tooling_review`; baseline `5d5fef9`, isolated branch
`fix/stable-pack-descriptor-reads`. Claim recorded before implementation. No push.

The v1/v2 shared pack byte budget now uses a descriptor-bound reader. Unix opens
use O_NOFOLLOW/O_NONBLOCK; Windows opens use FILE_FLAG_OPEN_REPARSE_POINT. The
opened handle must describe the previously inspected regular file. Reads stop at
the lesser of the per-file limit and remaining aggregate budget plus one byte,
then revalidate handle size/modification metadata and inspected path components.
Pack digests are computed from these returned owned bytes, not a later path read.
Unrelated child changes in shared ancestors do not invalidate directory identity.
Empty data remains empty; interpretation and canonical v1/v2 validation are
unchanged. Imported Lua is never executed.

The helper makes no ancestor-race sandbox or filesystem-isolation promise. Unix
stamps include device/inode/change time; Windows stamps use creation time and
attributes alongside length/write time. These reject ordinary changes and are not
an adversarial proof of immutable Windows file identity. Actual Windows descriptor
behavior remains unverified here and requires hosted checks.

Actual validation:
- All ingest targets: 130 tests passed, including the unchanged nonexecution and
  Lua dependency-closure guard.
- CLI pack/content_pack_v2/forever_sources/authored_slice: 34 tests passed.
- New controlled checkpoints exercise replacement before open, growth after open,
  mutation/removal after reading, symlink replacement, and regular-to-FIFO swap.
  The FIFO has no writer; the call returns an error without waiting for one.
  Exact bytes/empty input, per-file and aggregate bounds are covered.
- cargo clippy --locked -p azeroth-ingest --all-targets -- -D warnings passed;
  formatting and git diff --check passed.
- Local LLVM coverage uses pinned cargo-llvm-cov 0.9.1 and repository Linux
  continuous-profile settings. Final successful collection contains the 164 tests
  above: pack.rs 740/790 lines (93.67%); bounded_file.rs 75/83 (90.36%). No gate
  threshold or coverage inventory was weakened. Full release coverage not claimed.

Initial expanded tests failed the exact dependency allowlist and source unsafe
ban after the file-open flag dependencies and synthetic mkfifo were added. The
allowlist now includes only libc/windows-sys file-open flag dependencies; Lua
reachability and execution bans stay intact. Test-only mkfifo code moved under
 tests/support, outside production sources. Initial failed coverage profiles were
cleared before final collection. A command combining --no-clean with --no-report
was rejected by the coverage tool and is not a successful check.

Next: independent review/cherry-pick into native bundle integration; rerun native
bundle/CLI tests and hosted Windows checks. This does not supply the eight hosted
acceptance harnesses or any actual release acceptance result.
