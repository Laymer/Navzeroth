# Additive public review V6

Owner/sole writer: Codex `/root/desktop_review`. Baseline1028b5b23b04ef1d656d39b0fc3ab3dc64e42515,
claim71a0cdfe; isolated docs/stable-public-review-v6, `/tmp/navzeroth-stable-public-review-v6`.
Only excluded review/claim/handoff files change. No canonical bundle, acceptance
binding, application, workflow, keys or signing changes.

`release/v1/public-content-review-v6.json` is21594B, SHA256
`d1d79a495b49f6650e30d9f1dad9ed9ae5b107fc25ff87602814c3cdee0cc0c8`.
It binds actual tested sourcefcf170ad50722d8406f4662e6b2173c3c1c756a2 and
production0a359151d582b66d613f2feea6d59352cf48315bc419cd895eba37534675d62a.
The unchanged Git-tree fingerprint algorithm independently reproduced that hash
across3117production entries. All25nested public FileRef occurrences were streamed
from actual hydrated regular resources and matched exact lengths/hashes, with each
corresponding HEAD ordinary blob or LFS pointer independently checked. No network
fetch or private evidence/key access was needed.

The fresh coordinator optimized full-pack proof15568B SHA106cc6e2143aca3aa7cd188668b73587636687ddb5de84c01df946ab6e2dd8d1
and measurement are explicitly current. Actual coordinator execution138.945234232s,
peak6402372KiB on shared Linux/WSL2,32logical CPUs/MemTotal32675952KiB; Linux does
not exercise the Windows-specific descriptor branch. This reviewer did not rerun
that workload. Semantic comparison with actual archivedf391 proof differs only
in test_source.commit. Oldf391 proof/measurement use their real archived FileRefs;
historicalf064 component suites retain their original execution attribution.
V1–V5 review bytes remain immutable and individually pinned as historical evidence.

Repair union review: PR248 was authored by this reviewer, and its independent
review/nativePS5.1/localPS7 each20checks belong to the coordinator. PR249 tooling
implementation has coordinator3targeted tests plus this reviewer's actual2realGit/
bounds tests and narrow diff review. PR250 world implementation has tooling
independent review and coordinatorLinux17+3checks. GNU Windows-target compilation
and native PowerShell/C# API probe are not native Windows Rust execution. No
independent self-review or separate signing-key custody is implied.

The preliminary40ea producer39400861524 failed before all eight cases due to JSON
member order; continue-on-error job success is not acceptance. V4coverage39405119048
retains intermediate32native-smoke checks but final cleanliness failed with missing
dirty-path diagnostics. AebWindows replacement test also failed. No intermediate
result becomes a current protected/coverage/installer pass. All nine hosted/owner
observations remain unverified; publication_allowed_by_this_review isfalse.

Actual review checks: fresh fingerprint,25streamed nested refs plus HEAD pins,
semantic proof comparison,2clean-source Node tests and gitdiffcheck. Current full
suite/Windows Rust/hosted acceptance/signatures were not executed by this reviewer.

Next: coordinator independent review of this immutable public artifact and normal
integration, then actual canonical preparation with the exact review FileRef,
independent compiled-policy/source verification, authorized receipt renewal and
all required fresh source/protected/hosted/owner gates. No future artifact is
represented as already present or passed.
