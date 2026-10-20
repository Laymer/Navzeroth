# Actual weekly network and cache-only replay

Owner `/root/tooling_review`; baseline13e4ad3df4747459a50ffa8ab2b4c86c84d6d81a.
[Run39394989774](https://github.com/Laymer/Navzeroth/actions/runs/39394989774)
completed successfully2026-10-20T09:08:18Z after starting08:39:40Z. Downloaded
job logs and artifact12253535907 (`world-data-logs-39394989774`,33417 compressed
bytes) independently inspected. Exact retained log hashes and dispatch IDs are in
[the evidence receipt](../reviews/2026-10-20-weekly-network-run.json).

Actual network-enabled acquisition selected fishing, CMaNGOS and Wago generation;
Wowhead sources were disabled. Wago stayed explicitly policy-pinned to historical
1.60.1.70245, with target70450 kept distinct. Generation26.1s, verification45.0s,
content293.9s, registration480.7s and gate0.1s all completed. The second pass ran
explicit `--offline`, not a second network acquisition: generation19.7s,
verification5.2s, content296.3s, registration474.8s, gate0.1s. Both passes ran all
required stages; detect was intentionally skipped. Enhancements/professions were
explicitly excluded/skipped, not passed. Each generated35shards/475355records.

The reproducibility receipt reports exact equality for280 published material
paths/1624318292 bytes and129 unpublished build products. Only the explicit target
readiness report and pipeline/cache logs outside published paths are excluded.
The published-material set is broader than the installer bundle; these byte
counts are not installer size or application memory measurements. No material
bytes were normalized to conceal differences. This audit inspected the retained
receipt/logs; raw generated corpus was not included in the log artifact and was
not independently reacquired.

Publication stage actually pushed generated head
c23dd2b5e1e8566df519ac58f921395c2f2c768e and opened
[draftPR243](https://github.com/Laymer/Navzeroth/pull/243). Three changed files:
target readiness report, source admission registry and source update metadata.
This is not a no-op run; generated changes remain deferred and unmerged.

Important historical distinction: the pipeline prints “Release readiness: ready”
from the1.0.0-rc.1 report's historical `publication_allowed:true`. The actual report
has `stable:false`, `stable_ready:false`, `candidate_ready:false` and53blockers.
This is not a stable readiness or acceptance result. An initial audit shorthand
calling it candidate-ready was corrected after parsing the actual gate JSON.

Initial `gh pr checks243` showed no reported checks. Later automatic PR runs had
`action_required`, not pass. Explicit workflow dispatches were therefore issued
once each on exact generated headc23: CI39398609139, Coverage39398612016,
Independent source39398615432 and Windows package39398618648. Their head identities
were checked; results were queued/in progress at this handoff, not passed. No
second world-data dispatch, acquisition or generated merge occurred. Next: retain
these exact-head check outcomes, review generated source changes separately, keep
PR243 deferred under the release agreement. No quota expansion, purchases,
signing or owner/Windows smoke claim is made.

Local evidence remains `/tmp/navzeroth-weekly-39394989774.log` and
`/tmp/navzeroth-weekly-run-39394989774/`. This documentation does not alter the
release/version/signing worktree or its frozen production identity.
