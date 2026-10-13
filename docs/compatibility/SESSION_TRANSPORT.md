# Local session-event transport v1

This recorder transport is independent of exchange schema v1. It contains no new
exchange kind and never executes Lua. Supply text explicitly, or select one file;
there is no session watcher or background sampling.

Framing is ASCII whitespace, `ANAVSESSION1:`, one JSON object, ASCII whitespace.
The prefix is case-sensitive. Documents are bounded to 32,768 UTF-8 bytes and
JSON nesting depth 32 before parsing. Raw text permits another 1,024 bytes of
whitespace. The version-1 object is `{format:"anav-session-events",version:1,
fixture:boolean,batch:SessionBatch}`. Demo context requires fixture=true.
The existing core bounds apply: 1,024 events/session, 64 sessions/batch. Empty
batches, conflicting event sequence reuse, wrong format/version and unknown
fields on document/batch/event structs are rejected. This is JSON, despite the
illustrative unquoted property names in the shape above.

SavedVariables imports use the existing literal-only parser (8 MiB whole-file
bound, table depth 32) and `AzerothNavigatorDB / session_export`; the extracted
session document still has the 32 KiB bound. `snapshot_export` is a separate
workflow. Selected text files use the existing `anav1_text` file-format DTO for
plain transport text, but require the session prefix in this command.

Select an already imported character profile. Character, edition, build,
interface, namespace and fixture flag must all match on the server. Sessions
never affect the current snapshot. Per-session events are sorted by sequence
and identical duplicate events removed; canonical identical content is a no-op.
Conflicting session identity reuse fails the entire import transaction. History
holds at most 128 sessions/profile; capacity refusal never prunes history. Delete
sessions explicitly before importing additional history.

Migration v5 persists immutable canonical per-session batches, their SHA-256
and derived analytics. Reads recompute and compare analytics rather than trusting
a stale cache. The current derivation uses a 300,000 ms observed-span threshold
and 5,000 ms UTC-consistency tolerance; methodology records the span threshold. Any missing or untrusted event UTC prevents a certain mapping.
Measured monotonic coverage is distinct from active play and wall-clock duration.
Unknown endpoints, explicit unclassified intervals, uncovered intervals, gaps,
clock flags and UTC uncertainty remain separate. Calibration is deferred.

Native interaction and recorded-client addon/clock behavior remain unverified
until the consolidated live checklist provides revision-scoped evidence.

## Manual addon recording

`/anavsession start <anonymous-profile-id>` creates a new recording identity using
an independent recording counter. Then use `sample`, `pause`, `resume`, `stop`,
`export` and `status`. Recording is opt-in. Sampling reads level explicitly;
quest absence/completion does not establish turn-in, and no quest/equipment event
is inferred. Each running span ends with an explicit unclassified marker. Pause
spans remain gaps. No event or timer samples in the background.

Manual stop records a terminal `recording_paused` event (recording disabled); it
does not claim that the character logged out. The runtime still becomes stopped.

Export requires a stopped recording: an intermediate export under the same
immutable identity would conflict with the final batch. Export before reload,
then reload/logout to save, and import `session_export`. Reload starts idle and
never restores the monotonic timeline. Successful exports survive later failures.

The producer uses the existing smaller JSON encoder ceiling (16 KiB, 2,048 nodes)
and at most 128 events/runtime, below the consumer's 32 KiB/1,024-event bound.
It stops visibly at the first event/encoding limit or unreadable/reversed monotonic
clock. Coverage ends at the last retained event; unobserved time is not invented.
UTC trust uses runtime operator confirmation and a five-second consistency
threshold. A UTC inconsistency is recorded separately from monotonic coverage;
any untrusted/missing event UTC prevents a definite UTC mapping.

Live GetTime behavior, slash commands, reads, SavedVariables persistence and
native history interactions remain experimental until the consolidated checklist
has evidence for the recorded client and installed candidate commit.
