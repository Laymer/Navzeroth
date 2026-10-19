# Local archive failures preserve unknown and unsupported

Owner and sole writer: Codex `/root/local_data_gaps`; baseline `826aa21`.
Branch `fix/local-data-archive-failures`; claim
[archive failures](../claims/2026-10-19-local-data-archive-failures.md).

Closed the V2.2c handoff's classification gap: unexplained short reads and generic
native failures now produce `unavailable_unknown`; unsupported archive encoding
produces `unavailable_unsupported`. Actual encryption stays `unavailable_encrypted`.
No failure retains partial bytes or grants playable availability. Projection keeps
the distinct disposition and reason as incomplete-domain gaps.

New research catalogs and checkpoint envelopes write version 2. Verification
accepts old version 1 envelopes with their historical dispositions and preserves
those recorded classifications; the two new dispositions require version 2.
Future versions are refused. Bindings, summaries, decoded tables, metadata and
projection formats are unchanged. An older binary will refuse new catalogs;
re-extraction is required to correct historical ambiguous failure classifications.
Existing checkpoint profile binding still prevents resuming a different extractor.

Actual checks (2026-10-19, Linux):

- `cargo fmt --all --check`: passed.
- `git diff --check`: passed.
- `cargo test --locked -p azeroth-local-data -p azeroth-core -p azeroth-storage`:
  passed; 692 tests, 0 failures, 1 existing ignored test.
- After adding the durable v2 checkpoint regression,
  `cargo test --locked -p azeroth-storage --test local_data_lifecycle`: passed,
  13 tests, 0 failures.
- `cargo clippy --locked -p azeroth-core -p azeroth-local-data -p azeroth-storage --all-targets -- -D warnings`:
  passed on final production and test changes.

Rust commands used `/home/laymer/.cargo/bin` on PATH, isolated
`CARGO_TARGET_DIR=/tmp/navzeroth-local-data-errors-target`, `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`.

Tests cover synthetic native statuses through extraction and hash verification,
absence of retained/decoded bytes for failures, v1/v2/future-version admission,
checkpoint serialization and SQLite reopen, failed downgrade preserving the prior
checkpoint, and distinct projection gaps. An initial focused run exposed an
existing test treating version 2 as unknown; it now tests version 3 instead.
No percentage coverage claim is made.

Native Windows, real installation re-extraction and live WoW checks remain
unverified. No private data was read or committed. Next: exact-head CI/Windows
checks and, when available, operator-authorized re-extraction to replace old
ambiguous catalogs. No merge or release is included.
