# Atlas coherent catalog handoff
Owner /root/inventory; baseline8ee8ad4. Root transfers Atlas files for race fix,
then requests indexed evidence lookup and actual source attribution display.

Selected Atlas listing, projection, marker and asset reads now hold one app lock
and one immediate SQLite read transaction. Marker snapshot, projection and
expected catalog token are checked together. External activation cannot pair old
content with a new catalog index/hash; a second-connection regression proves
activation waits until handoff then succeeds. Generic catalog-page command also
wraps selection and page. Initial generic DTO pages already reject a differing
indexed revision in the same immutable scope; they fail safely on activation.

Stored image listing indexes evidence and source records once per request;
calibration confidence no longer rescans the whole evidence vector per image.
Stored attribution consumes registry publisher, attribution and licence, retaining
unknown/unsupported/restricted/not-applicable distinctly. Combined display is a
String because two independently bounded4096-character legal fields must not be
truncated or rejected; underlying fields remain bounded and DTO remains JSON text.
Legacy demo attribution and golden output stay unchanged.

Checks: two focused unit regressions (external activation and attribution states),
two stored Atlas tests,15 Atlas command tests, five authored-content tests, eight
snapshot-workflow tests and stored URI unit pass. Strict app all-target Clippy
passes after adapting the attribution assertion to String. Initial unused-helper
and redundant-as_str warnings were fixed. Actual Windows scheme/display and full
licensed corpus remain unverified. Coordinator integrates this focused fix into
Atlas draft rather than creating a redundant PR; importer owns other snapshots
blocks. Next: independent review/hosted checks and content import integration.
