# Authored v2 content consumers

Owner/sole writer: Codex `/root/acceptance`. Claimed 2026-10-15 before code.
Branch `feat/v2-content-consumers`, baseline `d59aa91`, isolated worktree
`/tmp/navzeroth-content-consumers`. Own app Gear/Party/Dungeon services and desktop
commands/UI bindings. Source worker owns core v2 contract; inventory worker owns
capacity/storage; coordinator owns v2 pack/release bindings. Shared API changes
must be agreed before editing. Read AGENTS/status, model-binding audit, original
Gear/dungeon/party implementations and relevant implementation-plan requirements.

Preserve v1 manual workflows. Authored selection must resolve an exact immutable
active revision, retaining unknown/restricted/stale values and evidence. A selected
model never proves character class/role. Snapshot v1 lacks class/race/faction;
Gear class input is explicitly manual and Party missing facts remain unknown.
No new live binding or corpus is implied. Verification will be recorded below.

Implemented authored model selection (explicit matching manual class plus fresh
observed level, applicability/exclusions, profile/proficiency/layout), authored
Dungeon selection with evidence and unknown omitted timing/notes, and authored
quest access/exclusive/reset facts in Party and Dungeon eligibility. Gear and
Dungeon apply equipment configuration and acquisition availability, rewards,
entitlement and costs. Zero rewards/cost remain distinct from unknown; currencies
without snapshot balances do not become copper-free. No class/race/faction is
inferred from chosen content.

App catalog reads now use the common immutable v1/v2 storage view, including
planning capture/guards, exports, diagnostics, reminders and routes. Existing v1
hash/wire golden tests are preserved; optional selections are omitted when absent.
Strict duplicate/unknown-field v2 staging uses the v2 decoder and leaves prior
activation intact on failure. Local annotation submission remains v1-only and
explicitly refuses an active v2 catalog rather than reading a stale v1 revision.

Checks: full azeroth-app tests passed before latest capacity merge; re-run after
merge recorded in follow-up. Six focused authored Rust checks pass (four service,
two facts). Desktop typecheck, lint and all 197 tests in 34 files pass. Strict app
all-target Clippy passes after current capacity merge. Initial regression runs
caught optional null fields changing v1 goldens and obsolete UI mocks; both fixed.
Native Windows, actual 70374 observations and real corpus performance unverified.
All added content tests use explicitly synthetic fixtures.

Ownership transfer at the accompanying consumer commit: source_admission may edit
app authored_content.rs, gear/planning/joint/snapshots catalog endpoints; native
lib command registration; TS client/contract/demo/tauri/types; Gear, PartyEligibility,
Planner, Joint, RouteAcquisition and AuthoredContentPicker for indexed pagination.
Current authored picker pages at 200 but its service still enumerates before
paging; old catalog list commands still return full arrays. These are explicitly
unfinished PR5 integration, not full-catalog readiness. Source worker will replace
with storage indexed pages. Existing Atlas demo behavior also needs coordinator
review. This worker freezes all app/UI source until that integration handoff.

Post-merge validation: full `cargo test --locked -p azeroth-app` passed after
latest capacity integration; `cargo fmt --all --check`, desktop lint and
`git diff --check` passed. No production edits after this verification.
