# Stable foundation integration — protected-main candidate

Owner/sole integration writer Codex `/root`; baseline3cb74e6a; claim36230976.
Main's initial five-PR batch is complete; exact212head7de passed CI39382048557,
coverage39382048560, package39382048584 and real source39382048559, then merged
as3cb74e6a5bf7e0697c63b538599fcee87035fe8a at07:10:52Z.

Separate reviewed follow-up PRs merged only into the isolated integration branch:

| PR | Exact reviewed head | Integration merge | Actual root refresh validation |
| --- | --- | --- | --- |
|219|fd531656|d5073e2e|17policy+11synthetic-signing PowerShell5.1 checks|
|220|9b4d28e1|4d309343|6ContentScreen tests|
|221|47fb7294|2872645b|5source-objective Rust+14PlannerScreen tests|
|222|9816c749|1ddb55f3|30release/schema3/schema4 Rust tests|
|223|609f7624|78f7dc10|11content-set Rust+7ContentScreen tests|
|224|9e486f98|f100d284|5role/class/band Rust+7GearScreen tests|
|225|e6fb0f19|e39b3266|6receipt library+2receipt CLI tests|
|228|5147b79d|75ec61f2|168offlinePython tests|

All branches refreshed normally, no history rewrites. World independently cleared
219and228final deltas; tooling cleared220/221and222/223refresh deltas (the latter
are disclosed original-author refresh reviews, supplemented by root's independent
implementation reviews/tests). Root independently reviewed224/225final deltas:
all owned behavior unchanged; automatic lib/command merges retain reviewed
objective/default and release phase APIs. Earlier independent specialist reviews
remain in each task handoff. No unresolved conflicts remain. Initial verifier
test invocations with a wrong WSL distribution did not run tests and are not passes.

This batch supplies source-backed objective projection,9class/level heuristic
coverage, version-reset defaults and persisted inferred choices,16explicit deferred
boundaries, schema4hosted/owner acceptance primitives, unsigned1.0onlydownload
policy, signed-content receipt primitives and bounded reproducible weekly updates.
It does not complete the stable bundle or establish installer acceptance.

A separate protected-main integration PR must pass all10checks and actualsource
Linux/Windows mapping/parity on its exact final head. At this writing those fresh
combined checks have not run. No individual feature-branch integration is claimed
as a protected-main pass. Once main integrates228, dispatch one bounded real weekly
network/replay/draft-PR run and explicitly dispatch missing generated-branch checks.
The generated update remains deferred, never auto-merged for this release.

Next: complete independently reviewed content/native bundling and hosted acceptance
repairs on separate branches; integrate behind fresh main gates; create separate
1.0.0version/release PR, exact resultingcommit stability/source/coverage/package
verification and draft assets. Pause only for the owner's exact verified draft
installer smoke, then publish if all applicable gates pass. ActualWindows/owner
results, real content receipts and stablepublication remain pending.
