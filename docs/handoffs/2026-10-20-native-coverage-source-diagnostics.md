# Native coverage source-dirt evidence and bounded diagnostics

Owner `/root/tooling_review`; baselineaeb7fd8f9fbafbc5d597889d1272f757d84f9bf8;
claimf7ad6a99; immutable diagnostic code2e9443bff16b8406f12c7a88b756bb4186215c9e
(production97afa338, behavioral-test cleanup2e9443bf).

Actual V4 coverage39405119048/f520039c907d963c34d9971941a6c2bf474d7394 Windows
job118569061225 failed at its final clean-source snapshot10:41:11Z. It did NOT
fail at resource hydration, signed assembly, native build or native smoke. Exact
commands.json records expected results for all commands, including:

- Hydration:84 HEAD-pinned resources/395748446 bytes;22 exact LFS paths restored.
- Production CLI assembly success, then compiled-default-policy staged resource
  verification success:84files/2packs.
- Native worker refusal paths returned their required2/1 exit codes.
- Native smoke10:24:46.892Z–10:34:44.150Z completed32passed/0failed/0skipped steps,
 15screens. This is hosted synthetic native smoke, not owner acceptance or the
 eight installer cases. Debug/release coverage commands subsequently exited0.

The final snapshot rejected dirt with a generic error before retaining its Git
status output. No dirty paths were included in the artifact. Thus the writer and
actual tracked/untracked path remain UNIDENTIFIED; no profiler/schema/metadata
hypothesis is promoted to a finding. The coverage gate failed collection status,
and no successful Windows rust-receipt.json or combined per-file90% gate exists.
Linux raw receipt independently confirms exactf520 clean source,1605pass/0fail/
8ignored across207binaries. V4 TypeScript raw report has91source files all>=90%
lines/global95.98%. These separate passes do not repair the failed gate.

A controlled external shared Git checkout at exactf520, core.autocrlf=true and
skip-smudge initially clean, used only the existing local LFS cache to hydrate
all84refs/22objects offline; status remained clean afterwards. This rules out
actual bundle hydration alone in that controlled checkout. No network acquisition
was performed. Temporary clone: /tmp/navzeroth-v4-offline-checkout-diagnosis.

The authorized diagnostic-only change preserves the existing clean-source guard.
A small helper limits Git status output to64KiB and runtime120seconds, escapes
path text with JSON and refuses nonempty/oversized/failed status. The collector
checks after recording each successful command, labelling the boundary. The
existing finally block still retains raw profiles on failure. This enables the
next actual failure to identify the first writer, without ignores, resets or
runtime byte normalization. It is NOT a fix for the unidentified source dirt.

Actual3Node tests PASS: real temporary Git clean succeeds; tracked modification
and untracked generated file refuse with path/boundary and remain untouched;
output bounds, Git errors and control escaping fail safely; the existing native
bundle adapter test also passes. Diff check PASS. A redundant source-text wiring
assertion was removed before final review; behavioral checks remain. No required
coverage workflow was rerun or dispatched on this unbound diagnostic branch.

Single downloaded Windows raw artifact12258357227 is retained at
/tmp/navzeroth-v4-coverage-39405119048/windows (83,067,108 compressed bytes;
uploaded artifact digest e680a108067ff9fae26300a12a9b11a74402e36bbb3fbc90ac71b91b402b1c81).
Job log: /tmp/navzeroth-v4-coverage-windows-39405119048.log. Inspect commands.json,
logs/coverage-bundle-inputs.log, logs/coverage-bundle-assembly.log,
logs/coverage-bundle-verification.log and native-smoke.json for actual outcomes.
Raw profiles were retained,624 entries. No duplicate artifact download, keys,
signing, generated merge, gate waiver or production acceptance promotion occurred.

Next: independent review/normal integration with other owned repairs, release-owner
production/public-review/receipt renewal, then fresh required checks. Actual clean
source and per-file90% coverage remain mandatory; a diagnostic failure remains
failure. The earlier V4 native pass must not be relabelled a final coverage pass.
