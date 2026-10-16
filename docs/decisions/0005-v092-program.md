# ADR 0005: The 0.9.2 program

Accepted 2026-10-16. Baseline: `92f3f51` (`origin/main`, the v0.9.1 source).
Recorded by task G1 as plan section 21.8 requires for material deviations from the
implementation plan and the shared rules. The program is: an in-game addon GUI, a
WoW-inspired desktop redesign, end-to-end correctness coverage, release 0.9.2 from
separate PRs, and a root `TODO.md` that lists exactly what 1.0.0 still needs.

This ADR is scoped to that program. It does not widen any permission beyond it, and
it does not weaken a release gate.

## Decisions

### 1. Parallel worktree agents

The owner requested ultracode for this program. AGENTS.md says not to spawn
subagents without an explicit request; this program is that request, limited to
the following rules.

- Each task has exactly one writer, in its own git worktree and branch, with a claim
  in `docs/claims/` recording owner, baseline SHA, files and checks (plan 21.3).
- Tasks with overlapping files do not run in parallel. G1, G2, G2b and G3 own
  disjoint paths; G5a and G5b own disjoint screens.
- Reviewers also run in worktrees, never in the shared checkout (a "read-only"
  reviewer once switched the shared checkout's branch).
- Findings use the blocker / important / optional severities of plan 21.6. Confirmed
  findings are fixed before merge and the review is recorded in `docs/reviews/`.
- Published history is never rewritten; main is never force-pushed.

### 2. New addon entry points

In addition to the existing slash commands, which keep working unchanged, the addon
gains a GUI opened by `/anav`, a minimap button and a page under the game's
Options, AddOns. Each entry point is guarded (a missing API or template degrades to
a plain frame or to the slash fallback, never to an error) and every interactive
widget carries a tooltip. The GUI calls the same actions as the slash commands. It
adds no new data collection, no input automation, no memory access and no bridge
between the addon and the desktop. Desktop state remains imported snapshots.

### 3. Build 70425: development admission only

Build `1.60.1 / 70425 / interface 16001` is admitted for development capture and is
marked UNVERIFIED. Installation metadata reports it, but no addon-run evidence for
it exists. The exact build is recorded in every export.

- The 1.0.0 release target stays Forever `1.60.1 / 70374 / interface 16001`.
- Every LIVE acceptance row stays bound to 70374. A 70425 observation cannot
  satisfy a 70374 row, and editions or builds are never mixed silently.
- `target_tuple_matches` stays false for 70425.

### 4. Bundled OFL fonts

The desktop app may bundle unmodified upstream font files under the SIL Open Font
License: Inter and Cinzel. They are pinned by hash. The OFL texts ship with the
installer and a byte check in the package workflow verifies them, because today
nothing ships or checks a font notice. The native content security policy still
forbids `data:` and remote sources, so fonts load as same-origin files and the
production-build check fails on any `data:` URI or remote asset. Modified fonts are
not shipped (Reserved Font Name rules).

### 5. Original-art backdrop

The desktop backdrop and the site refresh use original, repo-owned CSS and SVG art
in the site's night palette. `site/assets/dragon-citadel.jpg` is not bundled: no
rights are recorded for it, its C2PA metadata says "origin confidence unknown", and
plan M6.5 forbids shipping it. No Blizzard art, textures or client fonts are shipped
by the desktop app or the addon (the addon references client fonts and textures by
name only).

### 6. Merge and release authority

For this program the owner authorizes merging the program's own PRs to protected
main once every check passes on the exact head, pushing the `v0.9.2` tag and
publishing the prerelease, and making every CI check required in branch protection.
This does not authorize anything outside the program, and it does not authorize
skipping or retrying away a failing check. Today GitHub enforces only five checks
(Rust format, Rust Linux, Rust Windows, pnpm Linux, pnpm Windows); the rest were a
process convention.

## Consequences

- Unverified items stay unverified and are listed in each handoff: native window
  interaction, WebView2 visual fidelity, every live-client GUI, HUD, taint and
  focus behaviour on 70374 and 70425, and SmartScreen behaviour for the unsigned
  build.
- 0.9.2 remains a developer prerelease. Stable readiness is unchanged, with the
  blockers `release-readiness` reports, until real content, rights and
  candidate-bound acceptance exist.
- The `release:` commit prefix used by earlier release commits predates the plan
  21.4 convention; program commits use `G<n>: ` or `docs: `.
