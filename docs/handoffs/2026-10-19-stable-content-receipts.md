# STABLE-CONTENT-RECEIPTS handoff

Owner/sole writer: Codex `/root/tooling_review`; baseline `5092886`;
branch `feat/stable-bundled-content-receipts`; isolated worktree
`/tmp/navzeroth-stable-receipts`. Claim precedes implementation; root approved the
separate-format, explicit-policy API before code. No actual keys selected/generated
and no release or actual review was signed.

Added domain-separated schema1 public bundled-content receipts and explicit CLI
sign/assemble/verify commands. Exact app/target/manifest/production/tested-source/
review FileRef pins come from the caller. Caller-owned ordered policy requires
operator and review_evidence signatures from distinct nonrevoked strong keys,
with operator-control disclosure. This never asserts independent key ownership or
proves review authorship. Historical local/private formats1/2/3 remain unchanged.
Strict canonical JSON, 64KiB limits and selected bounded file reads refuse malformed,
duplicate, unknown, oversized and ordinary changed/linked inputs. See
[BUNDLED_CONTENT_RECEIPTS](../data/BUNDLED_CONTENT_RECEIPTS.md).

Actual Linux checks:

- Full local-data + CLI test suites under continuous-mode LLVM:317 passed,
  zero failed, three existing ignored. Historical receipt integration suite19/19
  and six new library / two new CLI tests are included.
- Final per-file local executable-line measurements: new crypto215/217 (99.08%),
  CLI adapter68/70 (97.14%), CLI main358/386 (92.75%), local-data lib234/258 (90.70%).
  Declaration-only CLI lib has no executable lines. No thresholds were changed.
- cargo check for local-data/CLI, all-target clippy -D warnings, formatting and
  diff checks: passed.

Coverage used the repository collector's Linux settings: RUSTFLAGS='-C
llvm-args=-runtime-counter-relocation', LLVM_PROFILE_FILE_NAME='nav-%p-%c.profraw',
CARGO_INCREMENTAL=0 and zero dev/test debug data. The first ordinary-profile run
passed tests but produced corrupt profiles from resource-limited worker processes;
its merge failed and is not a coverage pass. Continuous-mode final rerun merged
successfully. Initial tests caught overly permissive printable edition tokens;
receipt validation now enforces an identifier. Clippy's byte-slice/as_chunks style
findings were corrected before the final rerun.

Tests bind every expected pin, reject altered signed payloads even with matching
altered expectations, reject signatures under a historical domain, exercise
ordering/unknown roles/keys/revocation/weak keys, malformed/canonical bounds, and
actual CLI byte-preserving file flow. Only conspicuous synthetic seeds are used;
CLI stdout is checked not to contain them. No actual release files are signed.

Outstanding: root independently reviews this patch, compiles/source-binds real
public identities and expected production pins, verifies actual review/resource
bytes at I/O boundaries, and integrates receipts with the verified shipped bundle
loader and release material gates. Receipt artifacts must remain outside their
own production/manifest fingerprint to avoid circular hashing. Native Windows and
hosted source-platform coverage remain unverified. No push or publication.
