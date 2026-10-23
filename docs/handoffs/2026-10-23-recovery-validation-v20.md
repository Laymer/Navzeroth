# V20 recovery validation and bounded diagnostics

Owner desktop_review; baseline9cc774e4f2656f26d544071d6bd174367ccca16a, initial claimb2df7f9f. Isolated worktree /tmp/navzeroth-recovery-guard-v20; branch fix/recovery-guard-v20. Diagnostics CODEfa11d5c8; runtime CODE2659d8c24c2576e58b496f9da1d62b689b527971. No schema, PowerShell, ADR, content, signature, canonical acceptance or timeout changes.

## Cause and corrected scope

The latest V19 attempt2, run39641767585/job119662619835, fails the1200s recovery watchdog after restore and check_database completed, during the subsequent required planning workflow. Four hosted rows passed; offline/RC1 upgrade/downgrade did not run. Attempt1 was cancelled during offline startup without a terminal native diagnosis. They remain distinct historical results. The20min workload and60min suite bounds, destructive candidate validation, independent post-install check_database and post-restore planning/toggles are unchanged.

The initially proposed populated-plan guard optimization was withdrawn BEFORE any Rust edits: every retained fullpack fixture has an empty current_plan, and hosted plan_session computes a result but never pins a plan. No actual hosted database artifact was available to inspect. This branch does not change the RecoveredReference transition or remove its second full validation.

Exact load-count tests instead exposed duplicate decoding: validate_local_foundation called validate_content_dispositions, loading every historical catalog, then recovery loaded all catalogs again for assets/column identities. Recovery now supplies a private callback to that same disposition traversal and performs one strict load per historical ID, disposition-key equality, fresh asset verification and dataset-column identity together. Public validate_content_dispositions keeps its original strict loader. Cardinality/foreign-key closure still covers every historical revision; ordering, active eligibility, schema/integrity, snapshots, features, orphan assets and source rows remain checked.

Validation owns an IMMEDIATE consistent read when entering from autocommit. The only cache reuse is a matching prior SQL-validated immutable Arc under the original unchanged managed generation/schema/data_version/TEMP stamp. Other IDs load strictly without evicting the warm active Arc. Cold typed inputs never populate this cache; arbitrary caller transactions remain strict. Error/rollback scope restoration and cross-connection exclusion are exercised.

Diagnostics add at most32 cumulative command summaries while preserving32 recent events and the16KiB atomic checkpoint bound. Startup polling no longer erases completed recovery operation timings. get_database_status results automatically retain only whitelisted state, error presence and safe nonnegative load/activation measurements; zero differs from unknown. No arguments, imported data, error strings or arbitrary status fields are retained. All checkpoints remain passed:false/completed:false. No Stable-Workload main or producer assertion changed.

## Actual verification and review

- Six new storage regressions cover warm/cold history, no cache eviction or typed-input trust, public strict registration, arbitrary transactions, same-byte writes/rollback, corrupted inactive history, unchanged-schema asset-byte corruption, writer exclusion and real external commits. Initial peek-only tests correctly failed because of the duplicate disposition pass; the final callback fixes that cause.
- An external no-op UPDATE did not necessarily change SQLite data_version. That fixture was corrected to perform a real page write then restore exact bytes before commit, and explicitly asserts version change; no invalidation requirement was waived.
- All storage tests184PASS/0fail across21 binaries. Frozen-source focused6PASS. Initial Clippy found the removed loop's now-unused ids helper; deleting that private helper resolved it. All-target storage Clippy -Dwarnings, fmt and diff checks PASS.
- Actual combined LLVM storage/app479PASS/0fail/3ignored across63 binaries. Existing raw profiles were preserved with --no-clean. Initial unsupported --no-clean+--no-report invocation performed no tests; the corrected JSON report invocation ran the complete suite.
- Exact current-path LCOV plus existing rustTestRanges/lcovLines, excluding cfg(test): content_catalog377/39595.443%; content_dispositions295/31892.767%; recovery430/46293.074%. Report /tmp/navzeroth-v20-production-coverage.json1565B SHA256961dcee4eced1737f39d09d37e8427e324b4d3d1b2174382655a81ba03527d10; raw /tmp/navzeroth-v20-recovery-coverage.{json,lcov} and -tests.log. This is production coverage, not the raw LLVM summary including tests.
- All five workflow Node files58PASS. Affected workload tests28PASS on Linux Node24.19 and actual native Windows Node24.15 (seven exact source files copied into an owned Windows temporary test directory). Progress module Node coverage100%lines/100%functions/90.48%branches. Direct UNC --test lookup failed before execution; the Windows-local exact copy passed and is hashed in /tmp/navzeroth-v20-native-node-input-audit.json. No real network controls or local app installation occurred.
- Root independently statically cleared2659 andfa11, repeated exact focused6PASS0.22s after verifying binarySHA7c92c7d0f018a105c73598e959789afcc11b2d8a9fbb2ca61fc135fb6f0b4728, and ran28diagnostic Node tests. Earlier pre-unused-helper-removal6 results were separately identified; the final execution binds the frozen source. Tooling's additional review is pending, not inferred.

## Same-fixture engineering measurement

Both runtimes were genuinely rebuilt optimized: baseline9cc app Cargo-reported19.50s, repair2659 app17.21s. The unchanged actual recovery harness /tmp/navzeroth-v16-fullpack-recovery-harness.rs SHA2566e94b838250f2319e3f61258db3dd11f72d6af9c784d3c12f339bb768eeda10c (historical harness3447bcbf) was independently linked for each. Both use the SAME explicit OLD signed V13 bundle/public policy and declared manual engineering snapshot; they are not relabelled as V19/new-candidate bundle acceptance. Audit /tmp/navzeroth-v20-historical-source-audit.json records its three public input files.

Baseline binary /tmp/navzeroth-v20-whole-recovery-baseline-9cc SHA256c59df223c8af31b115d45708e6a6012924f88c1705e07e5b57a17b1c851ab792; repair /tmp/navzeroth-v20-whole-recovery-repair SHA2568b03f4e9411fc7d06e119c3bda44a6cce631296962bcf6b7bdc931b126054298. Both complete all backup, inspect, restore receipt/ready, profile/settings/content/false-choice equality, full integrity check, explicit close/reopen and repeated check assertions inside the same1200s wrapper. No fullpack jobs or heavy builds overlapped either timed run.

| Observed phase | Baseline9cc | Repair2659 |
| --- | ---: | ---: |
| Whole diagnostic |305.786843632s |210.229088706s |
| Inspect |33.492s |17.708s |
| Restore and initializer |82.353s |65.554s |
| Post-restore full check |30.871s |7.685s |
| Explicit reopen |81.937s |66.297s |
| Reopened full check |29.958s |7.662s |
| Peak process RSS |12174412KiB |12239288KiB |

Both produced byte-identical463335424B backups SHA256f4efd8953593da041cf66e69bee7a57e9d46f3573df840f4f19769fd42837823, with empty plan revision1; this also equals the retained genuine three-catalog input. /tmp/navzeroth-v20-backup-input-comparison.json records the comparison. Shared Linux/WSL2,32 logical processors, MemTotal32675956KiB, uncontrolled caches/shared host: observed runs are not controlled speedup, memory improvement, ordinary-PC performance or Windows acceptance. The repair's measured peak is slightly higher, not lower.

Exact raw logs and measurements: /tmp/navzeroth-v20-whole-recovery-{baseline,repair}.log and corresponding -measurement.json; phase summary /tmp/navzeroth-v20-recovery-phase-comparison.json. Baseline measurementSHA9ae795804eee2a552997d613dfcba3382670c8a561f1249470548f482805968d; repairSHA c2450afce1927c8193c8a4587f73859fb2cf194eaadb71f01287ba88870dead2. Earlier strict cold reconstruction32.124s/peak5250668KiB and per-catalog microtimings are recorded in the claim, separately from whole-app timings.

## Next

Root/tooling own a bounded hosted Windows engineering probe using the exact reviewed runtime, the explicit historical fixture and the same strict assertions before a complete renewed installer candidate. Such a service probe cannot establish actual installer/native UI acceptance. Preserve baseline binaries, raw profiles, logs and inputs. Coordinator may reuse the optimized target; no baseline artifacts should be deleted.

After normal integration: fresh combined-source functional proof, versioned automated exact downloaded-draft installer smoke (owned separately), new source/production bindings and receipts, all protected/source/Windows gates, exact resulting-main stability and draft/public asset verification. No current hosted result, automated draft result, manual owner result, tag, publication or website deployment is claimed by this work.
