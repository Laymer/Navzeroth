# Section16.2 explicit local diagnostic export

Owner: Codex /root/backlog_audit, sole feature writer. Baseline main6eec505
(after released v0.4.0 at2d5de646e439885c9eea27f5bfb4e2869a2ada60).
Branch feature/wave5-diagnostics; worktree /tmp/navzeroth-wave5-diagnostics.
Read STATUS, latest wave5 coordination handoff and plan16.2 before implementation.

Owns new app diagnostics module/tests, DiagnosticsPanel/tests, feature-local command
DTOs, claim/handoff. Root grants only additive pub mod diagnostics in shared app
lib; recovery/root retain snapshots/native/App/global command ownership.
Freeze narrow state capture with root; no shared wiring until ownership transfer.
Output is versioned bounded allowlisted metadata, enum error/event/timing counters,
validated numeric build/interface and dataset fingerprint/coverage, with no raw
exports, arbitrary identifiers/labels, account paths, tokens/chat or profile hashes.
Bundled addon version is separate from unknown installed version. Missing counters
stay unknown; explicit local create-new output refuses overwrite, with no upload.
