# Typed gear and acquisition explanations

Owner: Codex /root. Claimed baseline d48f883; released739a822/v0.6.0.
Source a05c557, normally reconciled with protected main76802f5. Separate
feat/gear-explanations PR; no backend scoring or game dataset changes.

Gear now shows declared weights, per-attribute score contributions, model certainty,
unmodeled effects, baseline slots, eligibility reasons and bounded configuration
pages. Acquisition views preserve declared probability assumptions, reserved versus
infinite expected attempts, lossless copper, conditional costs and unknown lifespan.
Route opportunities show typed reached/hop/exclusion traces and explicit incomplete
searches and geometry gaps. Raw captured details remain available as advanced data.

Independent review found fixed-point display loss at the safe-integer boundary and
an omitted Known resource label. Both were fixed before source freeze: BigInt
quotient/remainder preserves exact positive/negative milli values and adjacent units;
Known and Unknown render distinctly without inferred boolean availability.
Six targeted cases cover those boundaries, zero/unmodeled effects, reported trial
states, pagination beyond32, incomplete/false resource traces and future eligibility.
Peer review at exact a05c557 passed all six focused cases with no remaining blocker.

Actual checks: the first script filter unexpectedly ran the full suite; all158
unit tests across27 files passed, plus typecheck and lint. The browser script also
ran the full suite: all30 mocked Chromium cases passed, including ordinary Gear
and route explanations and narrow reflow. A production build passed after normal
main reconciliation. Logs: /tmp/navzeroth-explanations-focused.log,
/tmp/navzeroth-explanations-final-{typecheck,lint}.log,
/tmp/navzeroth-explanations-browser.log, /tmp/navzeroth-explanations-build.log,
/tmp/navzeroth-wave7-gear-peer.log. No failed invocation is counted as a pass.

Native Windows interaction, actual loot observations and recorded-client gameplay
acceptance remain unverified. These views explain supplied models; they do not
invent combat damage predictions, road geometry, measured probabilities or sources.
Next: require fresh exact-head hosted checks and integrate, then combine with the
remaining local state/governance tasks and typed recovery for the next preview.

Final wire review at a51b76c exposed a separate real DTO mismatch: Rust Field is
nested/tagged, while the first frontend DTO/test incorrectly used a flat string.
Actual-shaped stale input reproduced a missing identity label. Source4e53411 uses
the tagged level/role/quest+state/preference+key/objective shape, equipment paths
and participant identity, with bounded existing issue pages and no raw private
metadata echo. The regression covers acquisition and excluded-edge issue paths.
Final15 focused Gear/Route tests, typecheck and lint passed; prior full/browser
results above remain explicitly bound to their earlier source. Hosted checks must
run again on the final pushed head. Reproduction /tmp/navzeroth-gear-a51-wire-repro.log;
corrected checks /tmp/navzeroth-gear-wire-{fixed,typecheck,lint}.log.
