# Task 6 — explicit addon plan controls and HUD

Owner: Codex Worker A (/root/addon), sole writer. Claimed 2026-10-13.
Baseline 4044a8e (task2 PR48); coordination prerequisite PR47 at 32cec0f.
Updated prerequisite baseline to 2b4e912 after PR47 merged main90026ed.
Branch feature/wave3-plan-controls; worktree /tmp/navzeroth-wave3-plan.
Scope: pure runtime controller, explicit /anavplan paste/import/status/show/hide/clear,
experimental injected UI/HUD bindings, settings and Lua harness regressions.
Coordinator owns TOC wiring. Runtime loaded plans clear on reload; visibility may
persist. Failed strict bounded import retains prior plan. No background observation
or inferred objective completion. Read task2 handoff and existing HUD/plan APIs.
