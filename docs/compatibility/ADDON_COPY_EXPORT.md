# Explicit addon export copying

The addon can show an already stored export in a selectable window:

```text
/anavdiag copy
/anavdiag copy snapshot
```

`copy` selects `AzerothNavigatorDB.diagnostic_export`, the diagnostic JSON report.
Run `/anavdiag probe` explicitly first if no report exists. `copy snapshot` selects
`snapshot_export`, an existing ANAV1 character snapshot. It never creates a new
capture: the existing `/anavdiag snapshot <anonymous-profile-id>` command and its
recorded-build gate still govern capture. After integration of the separate
70374 adapter, exact `1.60.1.70338` and `1.60.1.70374` tuples with interface
`16001` are enabled experimentally. Copying diagnostics never changes that gate
or establishes readable gameplay fields. Published 0.8.1 lacks the copy commands.

Open the window out of combat, choose **Select text**, and press **Ctrl+C** yourself.
The window does not take keyboard focus when it opens; the explicit selection
button focuses and highlights the text. Paste diagnostic JSON into a private text
file or the requested diagnostic-report destination. Paste ANAV1 snapshots into
the desktop **Snapshot export text** field and select **Import text**. A diagnostic
report is not an importable character snapshot. Close with **Close**, or press
Escape while the text box has focus.

The copy view is a frozen copy of the stored bytes when the command runs, including
the original capture/build metadata. Reopen explicitly after a new probe/capture
to view newer stored data. Edits in the view are discarded, and saved exports,
capture counters, clock trust and loaded plans are never changed. No clipboard API,
automatic capture, automatic probe, background copying or imported execution exists.
Inspect personal data before sharing; do not commit raw character exports.

Diagnostic text is capped at 12,288 bytes and snapshots at 32,768 bytes. Bounded
JSON/format checks reject missing, malformed, wrong-kind, unsupported-version and
oversized stored strings without truncation. These checks do not establish game
truth, freshness or current-client compatibility. Existing exports remain untouched
on every refusal. Future/invalid SavedVariables layouts are not rewritten.

Opening requires a successful boolean out-of-combat query and available ordinary
frame/edit-box methods. Unknown/restricted/throwing queries or unsupported UI
operations refuse with a fixed reason; retry explicitly when safe. The view is
ordinary informational UI, with no protected gameplay action. Its own text edits,
selection and close callbacks are guarded against errors. Failed byte readback or
read-only restoration closes the view where supported and reports unavailable.

Synthetic tests cover bounds, unchanged source bytes, no new captures/focus,
read-only restoration, reuse, combat/unknown refusals and failed/truncated methods.
They do not verify actual in-game scrolling, clipboard capacity, focus, rendering
or restrictions. Those need the exact installed revision, recorded client build,
locale and conditions: open each available export, select/copy/paste, compare exact
byte count/hash, try editing/closing/reopening, and confirm original exports remain.
No live copy acceptance has been performed by the implementation worker.
