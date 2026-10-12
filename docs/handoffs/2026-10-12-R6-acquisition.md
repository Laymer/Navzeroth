# R6 acquisition assessment

Owner: Codex root; single writer. Claimed 2026-10-12 before implementation.
Baseline: 08f6daa. Branch: feature/wave2-r6.
Read gear handoff and plan section 10.5/M5.4.
Pure bounded finite-session assessment; no invented probabilities/conversions.
Checks pending implementation.

Implemented assess_acquisition with full source/effort metadata, finite session
and monetary bounds, caller-supplied incremental travel/formation/attempt/stop
costs and once/per-attempt cost schedules. Declared source probabilities remain
uncertain unless caller explicitly supplies independent identical player
acquisition trial semantics. Only declared p=1 with definite eligibility and
feasible known effort is Guaranteed; false requirements, zero probability,
insufficient time/gold and duplicate credit are Unavailable. Unknown/restricted/
unsupported fields remain separately recorded; not_applicable effort means no
applicable cost. No probability/gold-time/utility/lifespan defaults invented.
Stable finite-trial success formula and separately labelled infinite-horizon
expected attempts only under supplied probability assumptions; zero-duration
attempts bounded by explicit max_attempts <=1024. Budget reservations cover
maximum feasible trials and are not presented as expected time-to-success.
Full ItemKey credit identity prevents duplicate quest/gear opportunity accounting.

Actual checks: six synthetic regressions passed for finite time/gold, guaranteed
vs contested probability, unknown/zero/statuses, duplicate rewards, false
requirements and once/per-attempt costs. Locked workspace Clippy passed after
using checked_div; full workspace tests excluding desktop passed; fmt passed;
frozen pnpm install/contracts:types/check 73 passed; export-schemas no drift;
four committed pointer blobs passed. Development corrections were type fixtures
(Copper and ItemId are strings), missing RuleLimits import and Copper constructor.
Native/live checks unverified. Hosted CI pending draft PR.
Next: connect acquisition assessment to explicitly valued planner opportunities;
practical ranking is not claimed by this standalone assessment.

Review follow-up: distinguish an explicit caller spending budget from captured
money using BalanceBasis. ObservedBalance must have fresh clock evidence under
Reject; AllowStale may calculate capacity but remains uncertain. CallerBudget is
an explicitly authoritative ceiling, not an imported live-balance assertion.
Seventh regression rejects stale observed money and nonzero unaffordable cost;
known fresh balance permits the declared conditional guarantee. All seven tests
and locked workspace Clippy pass after this correction.

Coordinator final verification: hosted [run 38308662161](https://github.com/Laymer/Navzeroth/actions/runs/38308662161) at 2da2a01 completed SUCCESS; all seven jobs passed, including native Windows build/tests. Final integration documentation records the complete PR stack. No native interaction/live/packaging claim.
