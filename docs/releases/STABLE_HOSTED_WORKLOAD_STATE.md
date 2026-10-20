# Installed engineering workload state, version 2

This is disposable hosted-runner state, not an addon export, public exchange or
release acceptance result. The existing driver bounds the JSON input to 64 KiB.
Version 1 is refused: it cannot prove content preference persistence.

Every state retains the manual engineering context, profile, snapshot sequence,
application settings and workload disclosure. Two phases are allowed:

- `baseline`: records `baseline_version` and deliberately has no bundled-content
  state. Only a restart into a different application version may consume it.
  This is the retained RC upgrade path; the candidate must expose its current
  bundled defaults, active default revision and inferred travel enabled.
- `selected_content`: additionally records `active_revision`, boolean
  `accepts_inferred_travel` and the complete exposed `bundle_defaults` marker
  (`app_version`, nullable `previous_app_version`, `revision`). A restart,
  recovery or offline workload must match those fields exactly before doing work.
  A missing selected-content field cannot fall back to baseline acceptance.

Fresh installation and RC upgrade first require the current version's defaults
and true inferred-travel choice. The engineering plan explicitly uses inferred
travel. After planning, the workload selects false through the existing
`activate_content_set` command with just the already active revision, then reads
back and verifies that revision, defaults marker and false choice. The saved state
records false. Subsequent workloads check that false survived startup; recovery
also checks it immediately after backup restoration. Only after these checks may
repeated engineering planning explicitly opt in again. A finally block reselects
false even when planning fails; a failed check never returns successful state.

The evidence retains the initial content state, final content state and explicit
planning opt-in disclosure. Revision/default-marker changes during either toggle
fail. This tests persistence of a user choice within a version; it does not claim
that a false preference permits inferred routes. Existing source-backed planning,
settings, profile and scope assertions remain required.

Actual native Windows execution, application-version resets beyond the retained
RC-to-candidate path, and installed resource verification require their separate
hosted checks. Mock tests do not establish these environment results.
