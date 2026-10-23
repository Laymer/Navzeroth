# Bounded hosted Windows recovery engineering probe

Owner tooling_review; claim9150750e, baselineb601bb8f9e2b4323cf8fc30036e88a4b1708bcda
includes independently reviewed runtime2659/PR275. Worktree
/tmp/navzeroth-hosted-recovery-probe, ordinary branchverify/hosted-recovery-probe.
Initialcode3cc63dfd; reviewed ownership/process fix90086c215484a17c47fdf36d25ea99bc61380fe7.
Root owns the actual opt-in trigger. No existing package/download workflow,
application runtime, canonical release files, signatures or content were edited.

## Inputs and boundaries

The new workflow runs only on explicit verify/recovery-probe-* pushes or manual
dispatch when registered. Its distinct job is not a protected or acceptance gate.
Permissions are contents:read/actions:read, standard Windows2025, finite60min job;
compilation<=1200s and the WHOLE actual Rust workload<=1200s including startup,
backup, inspect, restore, state/integrity checks and explicit close/reopen. No
20min/60min acceptance or coverage threshold is raised. The branch used for
ordinary review does not match the trigger pattern. No dispatch happened here.

Fixture policy fixes artifact12405859488/run39641767585,46,082,416B, SHA256
be713e2cba0c2446d9c9d36a1918dbdcc4c1eaa9e17ca16cd47b602c3a52d5cd.
Artifact API head is9cc774e4f2656f26d544071d6bd174367ccca16a. Actual downloaded
SOURCE_COMMIT.txt and installer identify PR merge checkout
90e0d4dd3e574b1b0760e8f4a7a77f28933b02cc. They are not relabelled as one commit.
Their trees independently match9378020ca6dd9d727665d458e7e35af3f43559a1, confirmed
against the actual GitHub commit API and local9cc tree. The8-member ZIP directory
and source member were downloaded with bounded ranges. The actual installer
member was independently streamed and hashed:26,058,654B, SHA256
ffcb98661a1310458a663aa06451e765b99bdb07dba40a997f1e57aad5fc3225.
No local installer executable was written or run. API/download audit remains
/tmp/navzeroth-recovery-probe-historical-installer-audit.json.

The hosted wrapper fetches only that immutable artifact, checks API/whole ZIP/
installer/source hashes, and stages the OLD disclosed unsigned installer solely
to obtain its signed84-resource fixture. The actual test process links the NEW
reviewed Rust runtime. Old policy/build identity/synthetic snapshot metadata come
from exact historical Git blobs with independent byte/hash pins (git_path fields,
not misleading current-root FileRefs). The historical signed production identity
2aefc67b and the actual new runtime/harness commit are printed distinctly.
There is no compiled release-trust bypass or new installer acceptance claim.
Public old fixture policy is an explicit caller input to the existing app API.
No new key use, source-data acquisition, broad LFS hydration or paid runner occurs.

The ignored app integration test preserves the existing external recovery
harness assertions: fresh verified full bundle, manual engineering profile,
settings125, false inferred-travel choice, backup/inspect/restore receipt complete,
ready/errornull, exact profile/settings/content/default state, integrity checks,
explicit reopen and repeated persistence/integrity checks. It is ignored in
ordinary CI and requires explicit fixture environment and compiled commit identity.
Windows memory is NOT measured by this probe (null, not zero); this engineering
result cannot establish UI timing, ordinary-PC performance, live-game correctness
or any hosted acceptance observation. Existing installed-native/full-suite gates
still apply to the final signed candidate.

## Ownership and process lifetime

Review found two concrete bugs before any hosted dispatch. Initial3cc marked
installation owned before preflight; cleanup could touch a refused pre-existing
app. The final PS helper checks fresh directory/registry, absence of unowned
portable/WebView app processes, installer hash and unsigned status, then creates
a new nonce/source/hash/path marker immediately BEFORE installation. Both Python
and PS cleanup require that exact valid marker, preserving partial-install cleanup
without touching a pre-existing refused app. Marker JSON uses bounded opened-stream
reads and strict field types; it is an ownership record, not a secret credential.
These are owned ephemeral files, not an adversarial filesystem sandbox.

Desktop's actual synthetic reproduction also showed parent exit left an inherited
pipe child alive and exceeded a0.2s request by~20s. The final process helper uses
an owned POSIX process group, or a Windows kill-on-close JobObject. Windows creates
its own child suspended, assigns the job BEFORE resuming the own primary thread
(exact suspend count1), and tracks active descendants and pipe capture until the
original deadline, even after the root exits. It never selects an existing process
for job control. [Microsoft job ownership](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject)
and [suspended creation](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags)
document these primitives. No game-process access, injection or input automation
is introduced. Real inert native tests verify root exit0, a live owned child,
deadline failure and actual child termination through kernel exit-code queries.

## Actual checks and limitations

[Checks and public fixture/tool pins](../reviews/2026-10-23-hosted-recovery-probe-checks.json).
Author:8 Python regressions on Linux and8 on genuine native Windows;6 real-wrapper
inert ownership checks on LinuxPS7.6.6 and6 on native WindowsPS5.1. All passed.
These cover actual synthetic child/process-tree watchdogs, output caps, API/source
identity, ZIP traversal/duplicates/symlinks/normalization and ownership refusal,
partial marker cleanup and tampering. No real installer/process control is used
by ownership tests: actual wrapper/marker bytes call inert native-operation stubs.
Root independently reviewed90086 and executed Linux8/PS7six/actionlint/diff.
Desktop independently reviewed90086 and executed native8/PSsix. No separate
compiler build is attributed to either review.

The ignored test genuinely compiles in optimized app scope (initial44.37s; final
cached no-run after explicit-match cleanup). Default execution remains ignored.
All-target app Clippy-Dwarnings, fmt, actionlint and diff check passed. Initial
Clippy correctly rejected direct option_env!.expect; explicit runtime matches
preserve default compilation without diagnostic environment, without a lint allow.
An initial native ZIP regression incorrectly used a backslash name normalized by
Windows zipfile; the fixture now actually encodes the raw malformed name and the
reader rejects orig_filename/filename differences, including hidden normalization.
All eight tests passed again. Former ownership/orphan failures are not passes.

Native inert tests used a single official Python3.14.0 embeddable64 ZIP outside
Git, not an installer/Store action. SHA256 matched the message digest in its
publisher-hosted Sigstore bundle; the native executable's Authenticode status was
Valid with Python Software Foundation signer. Sigstore certificate/signature
verification was NOT independently performed. Exact URLs/hashes are in the checks
record. No registry or persistent interpreter policy was changed.

The hosted workflow retains bounded logs/phase results only, never SQLite backups,
profile data, installer ZIP/executable, compiled binary or Cargo cache. The result
always declares acceptance_observations_promoted:false. It still needs ONE actual
reviewed opt-in hosted execution by root; no hosted probe or current installer/UI
acceptance has run as part of this author task. Successful engineering results
must not replace any protected, source-parity, stability or final draft gate.
