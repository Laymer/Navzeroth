# Windows pack descriptor identity repair

Owner / sole writer: `/root/world_review`; baseline `aeb7fd8f`, branch
`fix/windows-bounded-file-identity`, isolated
`/tmp/navzeroth-windows-bounded-file-identity`.

Actual candidate Windows coverage failed
`pack::bounded_file::tests::replacement_and_changes_between_inspection_open_and_read_are_refused`
before native acceptance: same-size/same-byte replacement could retain the weak
Windows creation-time/attributes/mtime tuple. Evidence remains failed, not retried
into a pass: `/tmp/navzeroth-aeb-coverage-windows/logs/workspace-tests.log`.

Windows component inspection now opens metadata-only access0 handles with
OPEN_REPARSE_POINT and BACKUP_SEMANTICS, obtains metadata and full FILE_ID_INFO
from that same live handle, and refuses reparse metadata. Root/directories use the
same identity query. The selected component stamp and actual read descriptor
stamps compare volume serial, full128-bit ID, attributes, length and modification
time before/after reading. Directory mutable statistics remain normalized, so
unrelated child writes do not invalidate the read. Unix dev/inode/ctime behavior,
read limits, nofollow/nonblocking flags and aggregate budgets remain unchanged.

Existing windows-sys0.61 already supplies GetFileInformationByHandleEx(FileIdInfo)
and FILE_ID_INFO. No new dependency/feature/lock changes, unstable std metadata
methods or dynamic loader. Identity query failure (including unsupported filesystems
or network shares) fails closed, with no timestamp fallback. A path with UNC prefix
still uses component descriptors; arbitrary share support is not asserted. This is
ordinary replacement detection, not an ancestor-race sandbox or protection against
all same-file in-place mutation/time spoofing. Pack integrity hashes remain required.

Structural non-execution tests originally rejected the new unsafe FFI. The revised
test contract pins exactly one complete reviewed Win32 query expression in the
exact file and removes only that expression before the existing deny-all scan.
Any change/duplication to it, other unsafe expression, imported Lua execution or
dynamic-execution API still refuses. The query passes a live owned handle and a
fixed-size initialized identity output; imported bytes do not enter the call.
No whitespace/formatting bypass or module-wide unsafe exemption was introduced.

Deterministic regression explicitly restores equal bytes, length, modification
time and (Windows) creation time/attributes, and tests replacement at all three
inspection/open/read checkpoints. New stable files remain readable. A separate
directory-component replacement test retains the same final file ID on Windows,
and proves unrelated child creation remains accepted. No sleeps or flaky retries.

Actual checks:

- Linux `cargo test --locked -p azeroth-ingest`:138passed,0failed/ignored; includes
  five bounded-file tests and unchanged structural/sentinel non-execution checks.
  Log `/tmp/navzeroth-windows-file-identity-ingest.log`.
- Actual crate `cargo check --locked -p azeroth-ingest --all-targets --target
  x86_64-pc-windows-gnu`:passed with pinned Rust1.99 and installed ordinary std
  target component; all Windows runtime/test API code type-checks. This is not
  linking or native Windows Rust execution. Log
  `/tmp/navzeroth-windows-file-identity-cross.log`.
- Native Windows PowerShell5.1/C# API probe:root+directory metadata-only handles
  support FileIdInfo, unchanged file IDs match, equal-byte/length/creation/mtime
  replacement IDs differ. This checks the actual Win32 API, not the Rust tests.
  External source/log `/tmp/navzeroth-windows-file-identity-api-probe.{ps1,log}`.
- Linux LLVM coverage on final production code before the last test-only directory
  case:bounded_file.rs82/90 executable lines91.11%; all ingest tests then passed.
  `/tmp/navzeroth-windows-file-identity-coverage.{json,log}`. Windows coverage remains
  unverified and required; this is not an aggregate cross-platform gate pass.
- Clippy all ingest targets with warnings denied, rustfmt and gitdiffcheck:passed.
  No native Windows Rust test, full functional replay or installer result claimed.

Primary API contracts:
https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info
https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandleex
The FILE_ID_INFO contract combines volume serial and128-bit identifier; timestamp
values and64-bit nFileIndex alone are not used as replacement identity.

Next: independent review of immutable repair, separate draft PR against the version
branch after review, then normal integration. Runtime production changes invalidate
prior current receipts: parent must run fresh full-pack functional proof, production
review/preparation/signatures and exact hosted Windows tests/coverage before release.
No canonical release files, content/data, keys, signing or publication changed here.
