# Addon copy-export dialog — claim and handoff

Owner/sole writer: Codex `/root/acceptance`. Claimed 2026-10-14 before code edits.
Isolated worktree `/tmp/navzeroth-v1-copy`, branch `feat/addon-copy-export`, baseline
`bb3782cbac13b5fb3e6ca8196278ba7fcc427474`.

The coordinator explicitly assigned the missing manual text-export surface after
the user could not easily copy diagnostic chat output. Implement `/anavdiag copy`
for the existing diagnostic JSON and `/anavdiag copy snapshot` for the existing
ANAV1 snapshot as a bounded selectable dialog. Copying is user initiated through
normal selection/Ctrl+C; no automatic capture, clipboard API, imported execution,
source-build gate change or `Version.lua` change is authorized in this task.

Own the new pure copy-data module, Diagnostics binding, TOC entry, affected harness
tests, a dedicated copy-export usage document and this handoff. Root owns version
coordination and other live compatibility work. Reuse bounded guards and preserve
saved exports/unknown states. Read AGENTS/STATUS, final program and wave8 HUD/
reference handoffs and implementation-plan sections 7.1/7.2/12.4/18/20.

Planned checks: meaningful Lua regressions for actual stored export copying,
read-only selection, bounds/malformed input, unavailable/restricted UI, reuse and
load order; full addon harness and focused strict checks. Actual client/UI copying
is unverified until tested on the installed source and recorded build.
