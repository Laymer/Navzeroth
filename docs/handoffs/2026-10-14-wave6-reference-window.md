# Wave 6 — M10 ordinary imported reference window

Owner: Codex `/root/release_review`; claim recorded before code at `2ae4e1e`.
Baseline protected main `c0d8580`; branch `feat/reference-window`, isolated worktree
`/tmp/navzeroth-wave6-reference-window`. Normal merged protected main `1fca882`.
Frozen production/test source `fc28613f372ad8f8ce1605a9f499738456e358e0`.
Root approved the bounded ordinary-window scope and actual-webview IPC wrapper.

## Behavior and boundaries

The window is absent at startup, decorated/resizable, initially unfocused and not
always on top. Main desktop controls explicitly show/hide it and optionally request
keep-on-top. Hide turns keep-on-top off. State replies query actual window existence,
visibility and topmost properties; requested preferences are not reported as facts.
Closing uses ordinary window behavior. No focus-setting, hotkeys, game input
interception, live bridge, continuous observations or game controls were added.

The only window label and bundled path are fixed (`reference`,
`index.html#reference`), with no frontend URL/path/label parameters. The reference
bootstrap displays only read-only imported facts, a local 60-second refresh and a
manual refresh. The ordinary desktop can display the same reference even when
native window operations are unavailable. Capture age and level/role/location ages
remain separate; unknown/restricted/stale/zero age are retained explicitly.

The reference service reads the selected immutable imported snapshot under the
application mutex, releases it before pure projection, and advances freshness time
through the read wait. Native service access uses the existing DatabaseHost lease,
so recovery/startup failures remain errors rather than invented empty data. The
bounded 64KiB projection includes character/snapshot identity, exact context/fixture,
level, role, location and freshness only. It omits snapshot extensions, raw exports,
content, plans, diagnostic messages and capability claims.

Application invoke dispatch uses actual Tauri invoke webview metadata. `main`
delegates to the existing generated handler; `reference` permits only
`get_reference_view`; any other label/command is rejected with a fixed typed error.
Window management also verifies the actual main-window caller. This is application
IPC authorization, not game input handling. The unchanged capability JSON is not
claimed to enforce custom application command restrictions.

## Interfaces

- `get_reference_view` → internal schema-version-1 `ReferenceView` (app version,
  nullable selected snapshot and separate capture/field freshness).
- `get_reference_window_state` → schema-version-1 actual exists/visible/always_on_top.
- `set_reference_window` accepts only snake-case booleans `visible`, `always_on_top`
  and returns actual state. Only actual main callers can manage the window.
- Typed frontend client mirrors these commands. Demo window operations explicitly
  return unavailable; demo facts remain synthetic with uncertain ages.

## Actual checks

At frozen source `fc28613`, after the normal main merge:

- `cargo test -p azeroth-app --test reference`: 3 passed. Actual selected-profile
  service switching, missing-clock uncertainty, distinct ages/restricted/unknown,
  extension omission/bounds and production caller policy are exercised.
- `UPDATE_REFERENCE_GOLDEN=1 cargo test -p azeroth-app --test reference` generated
  `fixtures/desktop/reference/imported.json`; normal run checks it against Rust
  projection. This is a synthetic DTO used by the browser cases.
- `cargo clippy -p azeroth-app --all-targets -- -D warnings` and
  `cargo fmt --all --check`: passed.
- Desktop `pnpm test:unit`: 140 tests / 23 files passed. New cases exercise deferred
  old-client responses, manual/60-second reads, default absent state, explicit
  topmost/show/hide and normal fallback on native failure.
- Desktop `pnpm lint`, `pnpm typecheck`, `pnpm build`: passed.
- `pnpm test:e2e e2e/reference.spec.ts`: 2 command-mocked Chromium cases passed.
  Fixed `/#reference` renders only reference UI/read commands, separate ages and
  reflow at 320px/100% and 200% root text. Main controls do not show a window until
  explicitly requested; show true/topmost true and hide false/topmost false are
  verified, with fixed-code native error and usable normal reference fallback.
- Full `pnpm test:e2e`: 27 command-mocked Chromium cases passed. Server stopped;
  `ss -ltnp '( sport = :1420 )'` showed no listener afterward.
- Pinned local Tauri2.12.3 `manager/webview.rs` joins WebviewUrl::App using
  URL joining. An actual temporary offline URL2.5.8 executable confirmed all three
  dev/http/app-scheme bases resolve to `/index.html` with fragment `reference`.
  The native module also includes a URL regression, but that native test is unrun.
- Actual native `cargo check` attempted and failed before app compilation because
  this environment lacks `dbus-1.pc`. Native module tests/window execution are
  unverified; portable caller policy and parser probe are separate evidence.

During development a test incorrectly used unsupported Level(0); corrected to zero
capture age. A TypeScript closure needed explicit snapshot narrowing; the fake-clock
interval test needed clocks enabled before render. Initial browser assertions used
incorrect existing age wording and assumed a single StrictMode mount read; corrected
to existing wording and exactly one additional explicit refresh read, retaining the
read-only command-set assertion. Earlier failing/interrupted checks are not successes.

## Remaining environment acceptance / next task

Native focus, window visibility/topmost behavior, exclusive fullscreen, borderless,
DPI and multiple monitors remain unverified. Exclusive fullscreen visibility is
explicitly unsupported by this experiment. No actual game observations or recorded
client-build validation were performed. The normal desktop remains the fallback;
the permitted in-game Lua HUD remains primary live guidance. No native performance,
latency or window compatibility conclusion follows from mocked Chromium.

Next: independent root source review, exact-head hosted checks (including Windows
compilation), protected-main reconciliation, and release evidence. Environment-bound
M10 focus/DPI/monitor/fullscreen acceptance stays pending actual native observations.

## Hosted native compilation repair (PR89)

Actual Windows package job `115836304674`, run `38559354017`, failed with E0282
at the untyped local `let generated = tauri::generate_handler![...]`. This was a
real compile failure; earlier portable/Chromium checks did not validate it. Root
normally reconciled compatibility main into `b7e0ce3`, preserving the IPC wrapper
and compatibility command. Sole fix ownership was reclaimed in a separate claim.

The generated handler now has explicit type
`fn(tauri::ipc::Invoke<tauri::Wry>) -> bool`. Pinned tauri-macros2.7.2
`src/command/handler.rs` expansion is a captureless `move |invoke|` containing
only static command paths/wrapper macros and an unknown-command false result, so
function-pointer coercion supplies the missing argument type without bypassing
the outer actual-webview caller gate. All registered command entries are retained.
Actual local fmt/diff checks and all three portable reference tests passed after
the annotation. Native compilation remains locally DBus-blocked; no successful
native compile is claimed until fresh exact-head hosted jobs complete. Both Windows
native jobs and all other required checks must be rerun at the pushed fix head.
