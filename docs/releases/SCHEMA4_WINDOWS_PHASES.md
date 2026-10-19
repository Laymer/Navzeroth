# Schema 4 Windows preparation and publication

`release-build-readiness` is the explicit prebuild gate. Only pristine pending
Windows observations and the initial unbound installer identity may be deferred.
`release-draft-readiness` requires all eight hosted observations against one exact
installer; only pristine pending owner smoke may remain. Both return
`publication_allowed: false`. `release-readiness` remains the publication gate.
Preparation never grants publication permission. Historical release semantics and
the explicit unsigned policy remain in force.

PowerShell transport defaults to full publication readiness. Stable 1.0.0 package
preparation opts into `Build-Windows.ps1 -ReadinessPhase prebuild`. Draft artifact
verification uses `Verify-ReleaseArtifacts.ps1 -ReadinessPhase draft -Evidence
 dist/acceptance-evidence/acceptance-evidence.json` (one repository-relative path).
`Verify-DownloadedRelease.ps1 -Phase draft` verifies the draft before owner smoke;
its default `-Phase public` requires complete acceptance. Download receipts use
schema 3 and record the selected phase. Receipt phase is evidence, not permission
to publish.

An evidence overlay may change only `checks` and `candidate_installer_sha256` from
the source-bound schema 4 evidence. Content, production, target, review and source
bindings must stay identical. Rust readiness still verifies every referenced file
and acceptance condition. Draft and public artifact verification additionally hash
the actual installer and compare its hash with the selected acceptance evidence.
Changing installer bytes invalidates observations; they cannot be carried to the
new installer. Store later owner/hosted evidence separately from the tagged source
and pass that overlay explicitly, preserving the source identity of the installer.
The manual download workflow accepts an explicit evidence artifact run ID; missing
or stale evidence blocks the applicable gate.

## Signed bundle build inputs

Stable desktop builds require `-BundledContentDirectory DIR
-BundleBuildIdentityPath FILE`. Both are explicit operator-selected inputs, with
no custom signer-policy override. The CLI `check-bundled-content --input DIR
--identity FILE` verifies against the compiled trusted policy and must confirm
nonempty content and the exact app version. Identity is strict canonical JSON,
single-line UTF-8, at most 64 KiB. Tauri receives an absolute resource mapping to
`bundled-content/`; `NAVZEROTH_BUNDLE_IDENTITY_JSON` embeds the verified identity
and is restored after success or failure. Existing resources and signing options
remain in the Tauri merged configuration. Historical previews without these
parameters retain their behavior. No empty stable desktop bundle is allowed.

The bundle assembler/native loader/CLI implementation and package-workflow inputs
are separate integration work. This adapter does not manufacture receipts, keys,
content, production identities or review evidence.

## Hosted case recorder

`Write-HostedAcceptanceEvidence.ps1` requires a disposable hosted Windows runner.
It consumes eight raw case JSON files named from lowercase acceptance IDs in a
selected results directory, copies their exact text into the output evidence
subdirectory, and records root-relative hash/size references. Missing files remain
unverified. It never produces `OWNER-INSTALLER-SMOKE` and never converts legacy WIN
observations or old synthetic smoke output into schema 4 passes.

Each case has schema_version=1, exact id, method=`hosted_windows`, status=`pass`
or `fail`, version, source_commit, production_sha256, installer_sha256, target,
operator, recorded_at (UTC ISO timestamp), environment. A pass requires
completed=true. Additionally:

- HOSTED-UPGRADE-RC1: baseline_version=`1.0.0-rc.1`, baseline_installer_sha256.
- HOSTED-DOWNGRADE-REFUSAL: the same baseline identity, attempted=true,
  timed_out=false and installed_version_after equal to the tested version.
- HOSTED-PERSISTENCE-RECOVERY: restart_verified=true and recovery_verified=true.
- HOSTED-RESOURCES: installed_resources_verified=true and
  source_notices_verified=true.
- HOSTED-OFFLINE: network_disabled=true, from an actually disabled network test.
- HOSTED-PERFORMANCE: workload, hardware, limitations, measurements containing
  install_ms, load_ms, activation_ms, browse_ms, planning_ms, peak_memory_bytes.
  Measurements are finite nonnegative numbers; missing is distinct from zero.

These are recorded claims backed by raw reports, not independent proof that a
boolean corresponds to a real test. Actual harnesses must produce them from
completed tests; reviewers must inspect the reports and run identity. No new
hosted harness or actual hosted result is supplied by this change. In particular,
the current pipeline cannot draft stable 1.0.0 until all required cases exist.
Owner smoke must still run against the independently downloaded draft installer;
publication must then pass full readiness with that exact installer hash.

Material manifest schema 2 allows 2 GiB aggregate/ZIP closure. Only a content_pack
whose input path, byte count and SHA-256 equal one exact catalog_files pin gets
the 576 MiB per-file catalog bound. All other files retain 256 MiB; schema 1
retains its original 1 GiB aggregate and 256 MiB file limits. corresponding_source
and additional_kinds are retained in the exact public manifest; archives include
each selected file once.
