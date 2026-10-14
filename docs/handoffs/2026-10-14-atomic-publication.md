# Atomic shared planning publication

Owner: Codex /root. Claim77153f4 at protected-main cb7e03d; released f525911.
Dependency: recipient-export PR85 storage consistent-read service. Dependency
source was merged normally, preserving task histories and separate ownership.

A source-locked new regression reproduced a real interleaving: after the shared
PublicationGuard validated current content, its result handoff callback activated
another revision through an independent SqliteStore. The old guard succeeded
without preserving its validated view. Baseline log
/tmp/navzeroth-atomic-publication-baseline.log records the failing expectation:
independent activation succeeded while handoff still held the app mutex.

Correction50478f1 wraps validation and bounded handoff in the existing immediate
SQLite read transaction. It reserves the writer through that view/handoff, releases
on success/error, and retains elapsed-clock advancement through both app mutex and
SQLite reservation wait. A second writer now returns StorageError while reserved,
then succeeds after publication and exposes the new revision. This does not
provide authentication or protect against a hostile process editing files.
Caller handoff remains bounded as required by the original publication contract.

Actual fixed focused test passed; final app/storage/Lua full tests and all-target
strict Clippy passed. Final test count317,0 failures. Workspace fmt/diff passed.
Logs /tmp/navzeroth-atomic-publication-{fixed,final-tests,final-clippy}.log.
Peer /root/atlas_dense read the exact correction, independently ran the actual
second-writer test and found no blocker; log
/tmp/navzeroth-peer-atomic-publication.log. New test intentionally uses SQLite's
existing busy timeout; its roughly5second duration is not a performance claim.
No native/live tests were performed and those acceptance checks remain unverified.

Next: merge separate recipient dependency after fresh exact-head checks; update
this separate correction PR with main and require all nine checks before merge.
