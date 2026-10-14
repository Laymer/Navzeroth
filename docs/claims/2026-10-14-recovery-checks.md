# Recovery asynchronous-startup test synchronization

Owner: Codex /root/backlog_audit, sole test writer.
Baseline actual mainf4e964f971f36e68856e9c332b7c40493c008ac9.
Branch feature/wave5-recovery-checks; /tmp/navzeroth-wave5-recovery-checks.
Read STATUS, latest M9.2 recovery handoff and plan18/20.1 before code.
Root explicitly assigns only recovery test waiting behavior and claim/handoff.
No production startup, native, application state or CLI replay behavior changes.

Windows job115763008620 failed the existing async-startup publication test's
five-second deadline amid concurrent recovery/SQLite tests (library suite36.96s).
Replace yield-only tight status polling with a modest sleep between checks and a
bounded generous completion deadline; retain ready/service-preservation assertions.
This is synchronization tolerance, not a startup-latency performance assertion.
Record actual local/hosted evidence; Windows rerun is unverified until it finishes.
