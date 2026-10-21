# Independent review of V15 recovery cost repairs

Coordinator/root baseline2466b744. Actual V15 failed under unchanged1200s bound;
all historical and destructive-boundary validation remains required. No repair is
claimed to pass Windows acceptance before an actual new candidate run.

Tooling-authored fd722928889a84dce6eba59c91cfeab4ba55fa6f: root static CLEAR.
Private canonical_decoded owns storedString, strict raw wrapper and coredecode;
no caller supplies an arbitrary unvalidated graph. StoredString is dropped before
capacity/provenance canonicalization. Public typed staging retains graphvalidation;
hash/coverage/source/conflict checks unchanged. The counter measures zero EXTRA
storage typedgraphvalidation after decode, not an instrumented core call count.
Root independently ran matched author binaries:3capacity/strictdecode tests and
10catalogintegration tests PASS, including immutable prechange SQLite hash oracle,
strict nestedunknown/duplicate fields and historical compatibility. These were
matched author-built binaries, not a separate root compiler build.
Author actual uncached fullpack standalone wall10.189176768s baseline2466 versus
9.238312249s fd722 (phaseonly9096ms afterrepair), RSS5115384→5115640KiB. One shared
Linux/WSL sample with uncontrolled caches is not a controlled benchmark or Windows
claim, and does not establish sufficient recovery improvement. Root independently
verified baseline executable/library/harness/retainedDB pins and the after binary
0f8d4c622b9dfc88ab5d1cff23c42f32c9e54065c300130e9869bda3fe0c9d9b,
rlib34a417ffdf08bf665ae362960bcf53a580a2e948a9a7f8494d47facc62f9bbaf.

World-authored a9ce8e46d718b87e179be255c41341029d390513, parentfd722: root static
CLEAR; tooling and desktop independently CLEAR. Owned IMMEDIATE transaction starts
before original managedread generation; duplicate branch retains actualSQLvalidated
Arc, never trusts callerinput. Newcontent writes invalidate originalstamp; readguard
drops before defaults writes/commit, and RAII handles errors/earlyreturn. External
writes/schema/temp/shadow/rollback safeguards remain. Allrow/hash/provenance and
noncurrent historical checks remain; duplicatebranch defers only unused document/
report serialization. Root independently ran matched author binaries41storageunit,
7recovery and10catalog tests PASS. Desktop independently21cache+7asset+2consistentread
PASS. Author92affected tests PASS. No new root compilerbuild/fullpack execution claim.
Root logs /tmp/navzeroth-root-duplicate-cache-{unit,recovery,catalog}.log.

Desktop investigation0a843c65 demonstrated coldload10000ms versusreport78–79ms and
compacthash206–210ms, warmArc reads0ms; no metadata memoization productchange. Root
verified exact source/executable/raw/accounting pins. App rlib metadata mismatch
means harness used identical public report/hash expressions, not an app build.
Investigation handoff records exact oldinput/currentruntime scope and setupfailure.

Next: independently review tooling exactcanonical streaming hash change, one combined
existing-target coverage build at exactunion, and actualfullpack/recovery comparison.
Only then reviewed repair integration, fresh functional proof/review/receipts and
fresh allten/source/Windows gates. No timeoutwaiver, owner result or publication.
