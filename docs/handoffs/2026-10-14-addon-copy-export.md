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

## Implementation

Claim `aa30bec` preceded production/tests
`f1a2af5ec927f40ff22d2e7c6446a5f7d211ef52`. `CopyExport.select` is a pure bounded
stored-text selector: diagnostic JSON at most 12,288 bytes, character ANAV1 at
most 32,768 bytes. It validates JSON/outer format shape without rewriting bytes
or asserting freshness, compatibility or source truth. Missing, malformed,
wrong-kind, future-version and oversized exports remain untouched.

The injected Diagnostics binding creates one reusable ordinary scroll/edit view,
with a dark background, stored/not-new-capture label, explicit **Select text** and
**Close** controls. It never takes focus on opening, and selection requires a user
click. Edits restore the frozen view bytes; truncating/unavailable methods refuse
and hide where supported. UI callbacks use protected calls and never modify the
SavedVariables source. Opening refuses known combat and unknown/failed/nonboolean
queries, and retries only on another explicit command. Copying stored diagnostics
is independent of the existing 70338 observation gate. No new capture/probe,
clipboard access, automatic retry, event registration or compatibility claim.
Usage is in [ADDON_COPY_EXPORT.md](../compatibility/ADDON_COPY_EXPORT.md).

The coordinator may now own subsequent supported-build work from the frozen
production commit; this task makes no further source edits without coordination.

## Actual checks

Private target `/tmp/navzeroth-v1-copy-target`, with the documented Rust environment
loaded from `/home/laymer/.cargo/env`:

- `cargo test --locked -p azeroth-lua-harness`: **115 passed, 0 failed** at the
  frozen production source; `/tmp/navzeroth-v1-copy-tests.log`. Five new cases
  use actual diagnostic/snapshot producers, byte-preserving Unicode/whitespace,
  missing/future/malformed/executable/wrong-kind/oversized data, explicit selection
  and focus, edit restoration, reload, one-view reuse, 32 KiB to 12 KiB switching,
  combat/unknown gates and missing/throwing/truncating methods. Existing golden,
  load-order, static-source, privacy and runtime behavior checks pass unchanged
  except for the intentional module-load list addition.
- `cargo clippy --locked -p azeroth-lua-harness --all-targets -- -D warnings`:
  passed; `/tmp/navzeroth-v1-copy-clippy.log`.
- `cargo fmt --all --check`, `git diff --check`, and staged whitespace checks pass.
  Changed Lua source lines meet the existing 100-character limit.
- Initial bare `cargo` invocation failed because Cargo was not on PATH; the
  documented environment setup resolved it. The failure is not a test pass.
  Local `luacheck` is unavailable; the pinned hosted Lua static job is still required.

Live 70374 rendering/scrolling/text selection/clipboard capacity/focus/combat
behavior is **unverified**. Synthetic callbacks do not establish client support.
No native Windows or actual game check was run by this worker.

Next: independent frozen-source review and current nine hosted gates; verify the
packaged addon artifact and let the user exercise explicit diagnostic copying on
the recorded build. Real snapshot copying additionally needs successful capture
from separately implemented/evidenced supported-build adapters. No release/tag or
main merge was performed by this task.
