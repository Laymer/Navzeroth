# Strict hosted restore receipt verification

Owner/sole writer: Codex `/root/desktop_review`. Baselinef3ee076c1732d00c01a07d38e749047b88586c9c,
claim3f4153ff; codeaa2e78f051cf3fef754bef2c99d43d20c015d49a.
Isolated fix/stable-recovery-receipt, `/tmp/navzeroth-stable-recovery-receipt`.

## Demonstrated contract defect and scope

Rust RestoreReceipt has three fields: pending_marker_cleanup_required,
original_files_retained and recovery_directory. recovery.rs519–533 can return an
Ok receipt with pending_marker_cleanup_required=true after successfully restoring
the database but failing marker cleanup. Its DatabaseStatus can simultaneously
report ready and an error; the next startup requires recovery while the marker
remains. The driver ignored the receipt and could mark its individual recovery
verification true based only on currently accessible data/settings.

This is a source-backed contract defect and synthetic regression, not an actual
observed Windows false recovery pass. Actual V8 stopped at performance admission
before recovery ran. A later offline/restart acceptance step would catch the marker,
so no claim is made that this defect alone passed the whole release gate.

## Narrow repair

Capture the actual receipt and require cleanup_required exactlyfalse, retained
originals exactlytrue, and a nonempty recovery directory string. The retention
requirement applies specifically to this workload: it backs up/restores an existing
populated hosted database, which the source moves into the recovery directory.
It is not a change to general user recovery semantics where originals may be absent.

Immediately query the existing get_database_status IPC and require stateready and
error exactlynull. Missing/wrong-type booleans or missing/null/malformed receipt,
empty directory and missing/checking/recovery-required/error-bearing status refuse.
Only then run existing active-content/settings/profile/database consistency checks
before recovery_verified. No new API, file deletion, runtime Rust or source changes.

A callback retains bounded actual receipt/status summaries in raw failure evidence:
three receipt fields, at most512directory characters with explicit truncation flag,
64state characters and error-presence boolean. No full database or arbitrary extra
receipt fields are dumped. The restore mock now returns the actual three-field
shape, rather than the earlier empty object; status mock exposes consumed fields.

## Actual verification

Node21tests passed: existing17 plus successful ordering, pending/missing-eachfield/
wrongtype/retention/directory refusals, ready-error/missing/checking/failed statuses,
and bounded diagnostics. Failed restore/status checks prevent later planning and
consistency calls; existing successful recovery/persistence tests still pass.
Log `/tmp/navzeroth-stable-recovery-receipt-tests.log`,
SHAb6233f69245e1f8b9e9a503541635c6944df3f0298fd899d98a3265d2be566f1.

An external temporary copy using the original f3ee driver with the new negative
test fails with Missing expected rejection, proving the old ignored-receipt path.
The immutable reviewed working tree was never swapped back to old code.
Log `/tmp/navzeroth-stable-recovery-receipt-before.log`,
SHAbc38ba5801f9a8c5ba7c464a12858ee51fa2448f36e5144c12755f48d024215f.
Gitdiffcheck passed. Tooling independently inspected the actual Rust DTO/partial
cleanup path and passed21Node checks; its review is CLEAR. No actual installer,
recovery, offline network control or owner check was executed by this writer.

Only three JavaScript driver/test files changed. RuntimeRust, content/source pins,
metrics, budgets and canonical release/signature/status fields are untouched. All
unrun hosted observations remain unverified. Actualfcf full-pack evidence retains
its original identity/scope; no restamping or performance claim occurs here.

Next: separate reviewed repair PR, normal integration with other independently
reviewed repairs, renewed exact production/source closure and actual remaining
hosted cases. No V10 review/signing, stable tag, owner pass or publication is supplied.
