# V14 recovery runtime and fresh functional evidence

Owner/root, claim5ab4ff47; actual tested runtime union
`d636c539ef90fed43d237d04bbacc1ab74071ad7`, production
`a3baa49678a3492b9773ede35988a31cfcb2818bd39a341c6eb74182938b3808`,3129 entries.
PR261 merged484fbab8,262 merged5ca9a475,263 mergedd636c539 locally, preserving
normal history. Independent reviews and actual test/filtered coverage attribution:
[repair review](../reviews/2026-10-20-recovery-runtime-repairs.md).

Tooling genuinely rebuilt optimized app dependencies in93s and ran the same strict
full signed-content recovery diagnostic against the unchanged V13 bundle, whose
old production identity remains explicitly separate from new runtime identity.
Baseline721.433693646999s versus rebuilt347.8049998570059s, all exact restore/reopen/
profile/settings125/active revision/inferredfalse/default-marker/integrity assertions
passed. Peak RSS INCREASED from11946608KiB to12508668KiB (11.39 to11.93GiB).
Shared Linux/WSL, uncontrolled cache and shared host; no Windows or ordinary-PC claim.
Actual source, raw logs, measurement and hash-bound audit are retained separately
under release/history/2026-10-20-recovery-linux-{baseline,rebuilt}; root verified
all10 archived FileRefs and Git blobs plus actual raw executable/library hashes.

Root then rebuilt the unchanged stable_bundled_functional test against that new
runtime in1.35s and CLI in7.77s. Explicit actual full-pack execution passed in
78.30829478700616s, peak childRSS6206148KiB. It verifies source-backed quest planning,
54heuristic models/nine classes,18reminders,26dungeons and complete declared quest/
reward associations, preserving unknown eligibility, estimates and exclusions.
Actual proof15568bytes SHA256
`d9c409844872f8369050cd22054aa9a99c498ce6c015f0ab4ca1e0e97bb72070`.
The new proof is semantically identical to archived prior proof except its actual
executed test-source commit. No source data or inventory was regenerated/restamped.

Commands used PATH=/home/laymer/.cargo/bin:$PATH and
CARGO_TARGET_DIR=/tmp/navzeroth-stable-inventory/target:
`cargo test --locked --release -p azeroth-app --test stable_bundled_functional --no-run --message-format=json`
and `cargo build --locked --release -p azeroth-cli`; actual emitted test executable
ran with `--ignored --nocapture`, explicit NAVZEROTH_STABLE_BUNDLE_ROOT and
NAVZEROTH_FUNCTIONAL_PROOF_OUTPUT, finite1200s wrapper bound. Build/execution logs
are /tmp/navzeroth-v14-{functional-build,functional-execution,cli-build}.log.
The compiler-emitted executable path, source/production, actual wall/RSS and proof
hash are retained in release/v1/functional-linux-measurement.json.

Old ef3c/ec335 functional bytes were archived37376baa BEFORE runtime changes.
V13 signed identities and actual failed hosted evidence were likewise archived.
Metadata commite5a53b9d updates exactly4functional FileRefs, tested source and
production; removes old canonical manifest/identity/receipt only after archive
byte equality and clears bundled_content for fresh preparation. All9canonical
hosted/owner observations remain pristine unverified. Every raw archival newline
is preserved; checks disable only blank-at-EOF warnings for retained raw logs.

Next independent public V14 review, actual preparation and independent reproduction,
commit identities before key use, signed closure, then ONE fresh candidate push
and every required hosted gate. Windows recovery remains failed/unverified until
new actual evidence. Main807ef674, draft239 and deferred generated243 unchanged;
no stable tag, owner smoke pass, publication or website deployment exists.
