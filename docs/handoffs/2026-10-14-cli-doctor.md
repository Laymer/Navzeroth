# CLI doctor handoff

Owner: Codex /root, sole writer. Baseline published v0.4.0 main2d5de64.
Claim committed524a17b before code; branch feature/wave5-cli-doctor.

Implemented read-only doctor with human/default and bounded report_version1 JSON.
Reports compiled app version/OS/architecture, embedded Rust/Node/pnpm development
pins, fixed tool presence (without execution) and three explicitly selected path
kinds. Tool presence is not runtime-version verification; no selected content is
read or echoed. Unselected,missing,wrong-kind,symlink,unreadable remain distinct.
Missing dev tools are informational because packaged desktop needs no dev install;
explicit invalid selected paths fail1 and malformed/duplicate options fail2.
No recursive account scanning, system mutation or native/live compatibility claim.

Actual checks: cargo fmt --all passed; locked CLI doctor+arguments tests8 passed,
0 failures; cargo clippy --locked -p azeroth-cli --all-targets -- -D warnings passed.
Unix test proves detected executable is never run and selected symlink is refused.
Tests verify private names/content not printed, imported Lua is untouched, malformed
options fail safely and absence does not become verified. Windows-specific presence
and hosted checks remain pending at handoff creation.

Next: source-locked review/hosted checks; separate versioned deterministic content
pack build/verify task and bounded replay task. Runtime tool-version verification,
native dependency readiness and live client behavior are explicitly unverified.
