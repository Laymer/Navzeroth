# Guided Windows acceptance kit

The runner presents the complete [25 acceptance procedures](V1_ACCEPTANCE_RUNBOOK.md)
and every predeclared inventory scenario. It records **unreviewed drafts**, not
release approval. It neither controls the game nor executes imported data.
The candidate-packaging PR must be integrated before running it: an older verifier
fails closed. Missing complete licensed content still blocks candidate production.

Use native Windows PowerShell 5.1 or PowerShell 7 with Git and Cargo, the reviewed
candidate source checkout, its complete downloaded artifact directory, and the
installed addon. Review the candidate source SHA independently of the download.
First install the candidate interactively following WIN-INSTALL, then run the kit;
repeat the installation procedure to retain complete recorded evidence. The kit
checks installed addon bytes before and after the session but does not prove the
installed application's identity: WIN-INSTALL must record that observation.

Prepare a private scenario JSON file before testing. Its inventory hash is the
exact SHA-256 of `release/v1/coverage-inventory.json`. Include at least one case for
every inventory domain/unit; one case does not establish all record coverage.
Reviewers must expand cases to the required identities, applicability combinations,
and representative workloads. This is a format example, not game evidence:

```json
{
  "schema_version": 1,
  "inventory_sha256": "<64 lowercase hex digits>",
  "scenarios": [{
    "id": "SCENARIO-quests-region-one",
    "domain": "quests",
    "unit_id": "<exact inventory unit ID>",
    "method": "live_game",
    "procedure": "<exact records, prerequisites, permitted actions and observations>",
    "expected": "<every required outcome and unknown/stale/restricted case>"
  }]
}
```

Use `interactive_windows` for native scenarios. Predeclare performance workload,
full catalog revision/count, hardware class, sample count and budgets in the
scenario procedure before measurement. Include the integrated differing-progress
duo and required upgrade/recovery/offline/display/accessibility matrix. Unknown
inventories and absent hardware remain unverified; hypothetical cases are not
accepted inventory evidence.

```powershell
.\scripts\windows\Start-AcceptanceSession.ps1 `
  -Root 'C:\src\Navzeroth' `
  -Artifacts 'C:\private\candidate-download' `
  -InstalledAddon 'D:\Games\Forever\Interface\AddOns\AzerothNavigator' `
  -ExpectedSourceCommit '<reviewed 40-character source SHA>' `
  -Scenarios 'C:\private\scenarios.json' `
  -OutputDirectory 'C:\private\acceptance-session-01'
```

The output directory must be new and outside the checkout. The verifier checks
candidate readiness, complete artifacts/material closure and their source binding.
The runner additionally compares installed addon files to the verified ZIP, requires
the observed `1.60.1/70374/16001` tuple, captures OS/CPU/RAM/GPU driver/PowerShell,
and requests the remaining environment and workload details without account paths.
Manual typed observations are assertions for review, not automated client proof.

Each procedure asks for `pass`, `fail`, or `unverified`, actual results/limitations,
and individually reviewed sanitized files. Executed pass/fail rows need evidence;
all mandatory subcases must pass before selecting pass. Preserve raw exports,
databases and screenshots privately; create sanitized copies separately. The kit
requires an explicit review acknowledgement and hashes/copies those bytes but
**cannot automatically detect private information**. Original filenames and paths
are not included in draft JSON. Files are limited to 256 MiB each/1 GiB per session.

`session.json` checkpoints each completed row, retaining zero durations and empty
unverified evidence distinctly. Enter `q` to stop; missing IDs remain unverified.
A later session uses another new directory; reconcile multiple sessions during
review rather than overwriting prior observations. Timing covers procedure/result
collection and is not a performance measurement. Predeclared performance samples
belong in separate sanitized evidence files. Candidate/addon identities are checked
again at session end; an interrupted or failed session has no successful final
identity marker and must be investigated.

Review the session, every evidence file, environment, required IDs, full subcase
coverage and content dependencies independently. Only then translate actual
supported observations into the existing release evidence contract under `docs/`
and `release/`. Never copy a session wholesale as an acceptance manifest or convert
unverified rows to passes. Production changes require a new candidate and renewed
acceptance. Stable verification/publication still requires the full release gate.

Portable helper checks (synthetic only):

```powershell
.\scripts\windows\Test-AcceptanceKit.ps1
```

Native Windows PowerShell 5.1, actual installation, live Forever and full-content
performance remain unverified until executed on recorded target environments.
