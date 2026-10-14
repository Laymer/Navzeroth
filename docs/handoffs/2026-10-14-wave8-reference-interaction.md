# Wave 8 — focused reference shortcut and explicit interaction requests

Owner: Codex `/root/release_review`, sole task writer. Claim `35777a1` preceded
implementation. Published baseline `035f28ece68b0b81680ec42bbe2a792b8a95140d`
(v0.7.0); coordination `f0fbdad9e17d93a4e05415da7d33220e9451caed` was normally
merged. Branch `feat/reference-interaction`, worktree
`/tmp/navzeroth-wave8-reference`. Relevant plan: section 12.5 / M10.

## Behavior and boundary

The optional Ctrl+Shift+F8 shortcut is disabled by default and enabled explicitly
for the current desktop session. It operates only in the focused main desktop,
ignores repeated/composing events, editable targets and pending reference requests,
and neither registers an OS/global shortcut nor observes game input. The native
command independently verifies the actual main caller and main-window focus.
No command requests focus or moves the cursor.

The fixed ordinary reference window supports explicit interactive or pass-through
requests. Interactive dispatch requests focusability and pointer events;
pass-through requests non-focusability and ignored pointer events. Passive requests
refuse an actually focused reference window or an unavailable focus query. These
requests are applied before showing a hidden window. Changing interaction does
not recreate, show or focus a window. Hiding is attempted before resetting topmost
and interaction, so a later reset failure does not prevent the hide attempt.
Legacy show/hide also resets interaction explicitly.

Receipt fields distinguish the requested mode, each setter's accepted/failed/not
attempted dispatch, overall refusal/partial failure, and queried visibility/topmost.
There is no observed-mode getter in the pinned Tauri API, and setter acceptance can
represent queued native work; receipts therefore do not claim verified hit testing
or focus behavior. State-query failures return unknown state. Errors use fixed
reason codes rather than native messages or paths. Normal desktop reference content
remains usable when native window control is unavailable.

The existing fixed label/bundled URL, imported-only read-only content, default
absence/topmost false, and actual-webview invocation gate remain intact. The
reference webview can invoke only `get_reference_view`; both new commands remain
main-only. The generated handler retains the explicit function-pointer type from
the previously verified Windows compile repair. No capability JSON change is
claimed to restrict custom commands.

## Interfaces

- `toggle_reference_window { always_on_top, interaction_mode }` returns a
  schema-version-1 `ControlReceipt`. Actual visibility sampled under the existing
  native controller mutex selects show/hide; hide requests reset interactive mode.
- `set_reference_interaction { interaction_mode }` returns the same receipt and
  refuses an absent window without creating it.
- `InteractionMode` is the closed `interactive | pass_through` enum. The bounded
  receipt contains only fixed enums, booleans, nullable requested visibility and
  nullable actual `ReferenceWindowState`; it includes no arbitrary strings.
- The portable app state machine owns ordering/failure policy. The native adapter
  supplies actual main/reference focus, fixed-window state and supported setters.
  The typed frontend SDK mirrors the two commands. Demo mode explicitly reports
  native window control unavailable.

## Actual checks

At production `380309a8b1917a98acc2be7e8a3356d33cb157ed`:

- `cargo test -p azeroth-app --test reference_controls`: 6 passed, exercising
  production ordering, default creation, no recreation on mode changes, focus and
  absent-window refusals, injected setter/query failures, hide despite reset failure,
  actual caller-policy denial of reference/unknown callers and omission of raw errors.
- `cargo clippy -p azeroth-app --all-targets -- -D warnings` and workspace fmt passed.
- Full desktop unit suite: 186 tests / 32 files passed. The 11 affected reference
  tests cover opt-in shortcut filtering, immediate pending suppression, deferred
  old-client invalidation, explicit mode dispatch, truthful partial receipts and
  normal fallback. Typecheck, lint and build passed.
- Actual native `cargo check` was attempted and blocked by missing `dbus-1.pc`
  before application native compilation. Log:
  `/tmp/navzeroth-wave8-reference-native.log`. This is not a native compile pass.

After normal merging protected main `fbbda8fb8cf2fc25a3e5218d34a8e29cac6fb412`
at `27c39d7`, `pnpm exec playwright test e2e/reference.spec.ts` passed all three
command-mocked Chromium cases. These exercise the fixed read-only fragment,
explicit show/topmost/hide and native-error fallback, shortcut disabled by default,
editable-target filtering, explicit mode partial failure with separate actual state,
and two shortcut toggles. Reference controls fit 320px at 100% and 200% root text;
the standalone read-only reference page retains its whole-document reflow checks.
The server stopped and `ss` confirmed no listener on port1420. Actual result log:
`/tmp/navzeroth-wave8-reference-browser-final.log`. Test mocks are not native
focus/hit-test evidence.

During implementation lint caught duplicate demo-adapter command cases; these were
removed before the successful checks. The initial new browser assertion assumed
Playwright `selectOption` focused the editable select; actual focus remained on the
shortcut checkbox. The test now explicitly focuses and asserts that target before
the chord. Its initial whole-main reflow assertion also reproduced existing main
headings overflowing at 320px/200% (H1 scrollWidth356/client240; imported-character
H2 261/client240; Capabilities H3 248/client240). The coordinating root owns that
separate section12.6 fix. This feature's assertion covers its controls, while the
standalone reference-page whole-document check remains. Diagnostic log:
`/tmp/navzeroth-wave8-reference-browser-reflow.log`. No failed check is counted as a pass.

## Evidence limits and next task

Native compilation requires fresh exact-head hosted Windows checks. Actual OS
focus transitions, queued setter behavior, mouse hit testing, window-manager
differences, DPI, multiple displays, fullscreen and gameplay combinations remain
unverified. Exclusive fullscreen remains outside this ordinary-window experiment.
The permitted addon HUD remains primary live guidance; no live game observations
were performed. Section 12.5 native interaction acceptance remains evidence-bound.

Next: independent source review, exact-head hosted checks, protected-main
reconciliation and release verification by the coordinating root agent.
