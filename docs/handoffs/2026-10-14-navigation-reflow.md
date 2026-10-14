# Primary navigation reflow

Owner: Codex `/root/release_review`, sole task writer. Baseline origin/mainf4e964f,
normal dependency diagnostics85756db to exercise all11 primary navigation buttons.
Branch `feature/wave5-navigation-reflow`; scope primary-nav CSS plus new focused
browser harness only. App, screen IDs, command handlers and screen source are
unchanged by this task. Read STATUS, M6.6 Atlas handoff and plan M6.6/UI test matrix.

Primary navigation previously forced every button onto one flex row. Four actual
baseline browser cases failed nav scroll-width bounds at320/640px and16/32px root
font. At320px/16px the272px nav had1113px scroll width;320px/32px had2182px against
240px. The640px cases also overflowed. This baseline includes the diagnostics
navigation integration; no production styling had yet changed.

Navigation now wraps flex rows. Buttons use border-box width, bounded max width,
shrinkable minimum width and word wrapping, with a visible Highlight focus outline.
All controls remain in normal keyboard/DOM order; aria-current and labels are
preserved. No overflow hiding, fixed-height clipping or CSS reordering is used.

Four final mocked Chromium cases pass. They assert all11 buttons, visible overflow
mode, nav scroll width, every button's viewport bounds, Tab traversal through every
button and keyboard activation/focus/aria-current on Imports at320/640px with
16/32px root font. Desktop typecheck/lint/build and git diff --check pass.
The first post-style test reached its final screen assertion but expected the
navigation label Imports as a heading; actual existing heading is Snapshot imports.
Corrected only that harness assertion, interrupted the obsolete run, and all four
complete final cases passed. No failed/interrupted run is counted as passing.

Root-font scaling approximates browser200% text, not native OS DPI/zoom acceptance.
Actual Windows native focus/DPI/multimonitor and live gameplay remain UNVERIFIED.
No Rust/schema/dataset/dependency/source permissions changed. Port1420 released
when the final browser run actually completed. Next: source-locked peer review,
exact-head hosted checks and normal integration after diagnostics/backend task
PRs, then the already authorized v0.5 batch release audit.
