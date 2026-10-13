# Task 6 — explicit addon plan import and HUD

Owner: Codex Worker A (/root/addon), sole feature writer.
Claim baseline 4044a8e (PR48); updated prerequisite 2b4e912 includes merged PR47
at main90026ed. Branch feature/wave3-plan-controls; /tmp/navzeroth-wave3-plan.
Coordinator supplied TOC PlanControls registration and luacheck global declaration.

## Delivered

Pure runtime PlanControls wraps strict existing Plan.importTransport/Hud. Failed
replacement keeps the old plan; snapshot replacement suspends advice and requests
revalidation. Imported plan is runtime-only and reload clears it. No dependency
version identity or authored objective mapping is invented. HUD descriptions show
expiry, uncertain observation age, missing checks/mappings and revalidation.

Diagnostics binds explicit /anavplan paste/import/status/show/hide/clear on the
recorded 70338 tuple. Paste opens a manual edit box; an Import button applies it.
The 32 KiB validator covers the whole transport; the edit box allows one extra
character so overflow is rejected rather than silently accepted after truncation.
Guarded experimental frame/font methods preserve command use if UI support fails.
The informational HUD can move; imported markup is escaped in HUD/chat descriptions.
Only visibility preference persists in Settings. Explicit capture/import/status
refreshes state. No background sampling, new events or completion inference.

## Actual checks

- Full locked Lua harness: 89 tests passed, zero failures. Four added tests cover
  mocked paste/button controls, direct successful import, invalid replacement,
  snapshot supersession, reload/visibility, no frame fallback and an actual Rust
  application-generated plan passed through the new pure controller.
- cargo fmt --all --check and scoped all-target Lua-harness Clippy passed.
- Lua 5.1 scratch luacheck: 15 files, zero warnings/errors after fixing one long line.
- git diff --check passed.
- Initial test runs exposed exact module/global/settings expectations and ASCII
  source policy; updated those harness assertions for the added module/setting.
  One global chat escaping change altered existing usage text; limited escaping
  to imported plan descriptions and reran successfully.

## Limits and next task

Live 70338 paste capacity/frame/focus/drag behavior, combat restrictions, taint and
actual import/HUD behavior remain UNVERIFIED. Frame mocks and protocol tests do not
establish live acceptance. HUD refresh occurs only after explicit actions; expiry
is evaluated at refresh, without an automatic timer. Missing stage mappings never
advance any step. Dataset/ruleset dependency checks stay unavailable and advice
requires revalidation. Native/real-content workflows remain separate evidence.

Next separate task9 branch: conservative equipment observations and position-read
diagnostics. Session worker supplies task8 recorder binding patches independently;
Diagnostics remains owned by A to avoid concurrent edits. Coordinator reviews and
merges only after prerequisites/current-head hosted checks pass.
