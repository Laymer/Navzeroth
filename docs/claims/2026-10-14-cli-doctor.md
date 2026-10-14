# Claim: section19.3 CLI doctor

Owner: Codex /root, sole task writer.
Baseline: published v0.4.0 main2d5de646e439885c9eea27f5bfb4e2869a2ada60.
Branch: feature/wave5-cli-doctor; worktree /tmp/navzeroth-wave5-cli-doctor.
Read STATUS, latest release handoff and plan sections16/19.3 before implementation.
Scope: read-only diagnostic command, declared development pins, host metadata and
explicitly selected integration paths without recursive scanning or imported execution.
No account paths/content echo; distinguish unselected/missing/wrong-kind paths and
available tools from unverified runtime versions. A separate PR covers pack/replay.
Checks: focused CLI diagnostic/argument tests, fmt/Clippy; actual outcomes in handoff.
