# Exact unsigned preparation assessment

Owner/sole writer Codex `/root`, baselinebeb37015; tooling reviewer independently
proved that deleting required unverified coverage rows, or replacing them with
empty passed record/source/proof accounting, previously allowed unsigned bundle
preparation. Final signed/publication gates still blocked the malformed evidence.
No release was signed/prepared/published through this defect.

Core now exposes an explicitly unsigned preparation assessment, reusing the full
existing logic and exempting only absent bundle artifacts. It always returns
publication_allowed=false/stable_ready=false. Ordinary assessment still requires
signed bundle metadata. CLI material schema2 adds unsigned_preparation readiness
plus SHA256 of the same bounded evidence bytes parsed by its input loader. Readiness
also runs actual verified catalog bind_records and verifies evidence FileRefs;
accepted non-observational sources come only from the validated catalog. Schema1
omits the field. Schema4 prerelease previews remain inspectable with readiness
false. Python requires a positive exact-evidence-bound result before writing outputs.

Controlled original repro is recorded by independent tooling review. New core
regression covers valid unsigned preparation with nine pristine pending Windows
rows, still-blocking final prebuild, missing/duplicate units, missing/extra records,
empty sources/proofs, unsupported corrections, partial pending observations,
unverified functionality, stale production, historical schema and invalid version.
Eight Python tests pass, including missing/false/stale/nonboolean trusted assessment
rejection. Final core+CLI all-target portable LLVM run:698passed0failed5existing
ignored. All three modified Rust files exceed90% executable-line coverage:
core/release846/859=98.49%; CLI/release1346/1452=92.70%;
CLI/release/materials467/491=95.11%. Final all-target Clippy and fmt passed.
Logs:/tmp/navzeroth-preparation-{coverage-final,clippy-final,python-tests}.log;
raw:/tmp/navzeroth-preparation.lcov. Zero-test examples emit LLVM counter-bias
warnings; executable tests/report completed successfully. Independent rereview
remains required.

Initial expanded coverage caught three schema4 RC material fixtures that must stay
inspectable; corrected the helper to return non-ready RC assessment rather than
erroring merely for prerelease. This failure is not a passing run. No trust keys
or receipt bytes accessed; actual functional/Windows/manual results remain unverified.
