# Confirmed client target for experimental manual capture

Owner/sole writer Codex /root. Baseline `bb3782cbac13b5fb3e6ca8196278ba7fcc427474`.
Branch `feat/forever-70374-capture`, worktree `/tmp/navzeroth-v1-build`.
User reports GetBuildInfo `1.60.1 / 70374 / 16001`, Windows 11, out of combat,
no errors from probe/status, addon version 0.8.1. This allows preparing a guarded
experimental adapter for the identified target; it does not verify field reads,
clock trust, SavedVariables, HUD behavior or plan progression.

Read STATUS, final-program audit, current acceptance runbook and plan sections
3.3, 7 and 20. Own `Core/Capabilities.lua`, `Core/HudAppearance.lua`, focused
harness tests and task docs initially. Update Diagnostics build guards only after
the copy-export writer freezes that file and ownership passes back explicitly.
Centralize exact build matching; preserve 70338 support and reject unreported
versions/interfaces. All permitted reads and combat gating remain guarded, and
no imported or stored state is relabeled. Test both admitted tuples and wrong
build/interface, unknown/failed inputs and transport context isolation. Record
actual client checks only after new candidate installation.
