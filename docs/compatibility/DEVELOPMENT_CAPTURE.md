# Development capture inspection

Use this before a full acceptance candidate is available. It does not bypass the
candidate gate or satisfy a live/Windows release acceptance row. Game actions,
visual comparisons and reloads remain operator driven.

1. Exit the game. Back up the installed addon and its SavedVariables privately.
   Install an identified, checked addon revision with the existing Windows addon
   installer. Compare every installed file with that revision and retain the
   source identity outside the game. Version text alone is insufficient.
2. Log in, out of combat, and run `/anavdiag probe`, `/anavdiag status`, then
   `/anavdiag snapshot <anonymous-profile-id>`. Record visible errors and compare
   the character level with the actual character display. Run `/reload` to save.
3. Inspect only the selected project SavedVariables file:

   ```sh
   cargo run --locked -p azeroth-cli -- inspect-capture --input '<selected AzerothNavigator.lua>'
   ```

The command reads at most 8 MiB, parses Lua literals without executing them,
rejects duplicate keys, links/reparse paths, oversized files and unsupported
formats, and validates a present character snapshot using the existing contract.
Diagnostic JSON is bounded to 12 KiB; snapshot JSON to 32 KiB. Diagnostic version
fields accept only bounded numeric components or the explicit `unknown` value.
Unrecognized diagnostic fields and error details are never printed. Unsupported
capability status vocabulary refuses rather than accepting a claimed pass.

Output contains the diagnostic build tuple and counts, plus an optional validated
snapshot summary: fixture marker, target-tuple match, level status/value, clock
trust and counts of quest/equipment/objective records, including readable
completion flags, separate true/false completion counts and known equipment-slot observations. A present session export
also passes the existing bounded session contract and yields only fixture/target
markers, event count, quest turn-in event count and maximum session-relative elapsed milliseconds. This
maximum is not active play time or a total across sessions. Missing exports remain
explicit. Raw strings, character/account identifiers, item/quest identities,
locations, paths and arbitrary diagnostic details are not copied into the report.
The level/count summary still describes player state; review before sharing.

A matching tuple does not establish edition, installed source identity, freshness,
field truth or gameplay coverage. `live_acceptance_verified` and
`installed_revision_verified` stay false. Installation verification and the
operator's actual visual comparisons require separate evidence. Diagnostic
capability rows stay untested/failed; no presence probe promotes them.

Retain raw exports and backups privately. Commit only reviewed, sanitized evidence
and precise limitations. Final release acceptance still uses
[Start-AcceptanceSession.ps1](../../scripts/windows/Start-AcceptanceSession.ps1)
and the [release runbook](V1_ACCEPTANCE_RUNBOOK.md) against the frozen full-content
candidate, including its inventory and production fingerprint.


For opt-in turn-in observations, use `/anavsession start-quests <anonymous-profile-id>`
only on an identified checked build. `/anavsession start` retains manual-only behavior.
The addon requires supported normal event registration. It observes only the bounded
quest identity from `QUEST_TURNED_IN` while this mode is recording; it ignores
rewards and does not observe events while paused/stopped. Build changes or unreadable
identities stop the recording with a fixed diagnostic. Time before a turn-in remains
unclassified; the event establishes no objective mapping, loot, productive duration,
quest history or automatic plan advancement. Capacity refusal retains the prior valid
batch. Stop/export/reload remain manual; recordings never resume across reload.

For an operator check, start this mode, turn in one available quest yourself, then
stop, export and reload. Compare the observed event count with your actions. If no
quest is available, leave actual dispatch/payload behavior unverified. Keep raw
exports private and do not promote a development observation into final acceptance.
