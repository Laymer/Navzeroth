# Future-schema fixture after schema4 adoption

Owner/sole writer `/root/tooling_review`; baselineb69dabcf (reviewed exact report
LF correction); claimd8f54c1c. Isolated fix/release-future-schema-fixture.
Codefa37db39 was independently reviewed/cleared by root before delivery.

Actual CI39390030412 Linux job118519045498 failed release_readiness.rs344: the
fixture used schema4 as an unsupported future version, although the reviewed
stable contract explicitly supports4. The error occurs at the first schema
assertion, not at the later oversized-manifest check. Foundation still uses its
historical16MiB MAX_RELEASE_BYTES; no size policy was changed.

Independent download of coverage39390030386 Linux artifact12249770482
(coverage-rust-Linux-b9f24e0c63727d003f79119e0c799c26246a96eb,49MB) and bounded
inspection of workspace-tests.log confirmed exactly the same fixture failure,
16passed/1failed. No separate instrumentation fault was shown. Both workflow
failures remain failures until fresh checks run; later coverage stages were not
inferred passed.

Changed exactly one test value:4→azeroth_core::release::SCHEMA_STABLE+1. This
matches the separately reviewed delivery branch fix already at98d07985. Unknown
field, duplicate schema key and MAX_RELEASE_BYTES+1 rejection assertions remain
unchanged. No production decoder, policy, source, data, hash, receipt or role-audit
assertion changed. No additional duplicate contract test was invented.

Actual local validation: old focused test reproduced the same failure. After
correction all17 CLI release_readiness tests pass; all12 exact-input hydration/EOL
tests pass. Rust format and diff checks pass. Root independently reviewed the
one-line correction and is separately rerunning release tests; root's earlier12
EOL tests passed. No broad test suite or real Windows success is claimed here.

This branch also contains the reviewed b69 exact report-LF rule and autocrlf=true
regression for Windows CI39390030412/job118519045423; see that separate handoff.
Both causes are narrowly repaired without read-time normalization or weakened
assertions. Root authorized routine push and a draft repair PR into
release/stable-1.0.0-integration; merge remains root-owned after independent tests.
Fresh exact-head protected checks/source parity remain required. No signing,
publication, latest source acquisition, purchases or quota expansion occurred.
