# Actual stable Windows acceptance producers

`Stable-Acceptance.ps1` exercises the exact prepared or independently downloaded
installer selected by its expected SHA-256. Run only in a disposable GitHub-hosted
Windows runner, with PowerShell 7 and the locked Node/Playwright dependencies. The
script refuses ordinary developer environments and requires a new output directory,
no existing application process and an absent installation. It verifies the
source/checksum/material prebuild gate before installation.

```powershell
./scripts/windows/Stable-Acceptance.ps1 -Root . -Directory dist/windows `
  -InstallerSha256 <independently-recorded-candidate-hash> `
  -BaselineInstaller <downloaded-rc1-installer> -BaselineSha256 <public-rc1-hash> `
  -BuildIdentity release/v1/bundled-content-build-identity.json `
  -OutputDirectory "$env:RUNNER_TEMP/native-smoke/stable-cases"
```

The baseline hash and size must also match the committed
`docs/releases/1.0.0-rc.1-publication.json`. Candidate/baseline versions and unsigned
Authenticode status must match their declarations. The build identity must match
the release's production and target. Signed bundle trust comes from compiled public
identities, not from a policy chosen by a receipt.

The package workflow assembles `dist/bundled-content` with the explicit canonical
build identity before stable 1.0.0 packaging. It then runs these cases on the exact
prepared installer. Draft verification independently downloads and binds the same
installer hash; the producer can also run directly on those downloaded assets.
Historical RC behavior is retained. A missing or failed case blocks the package
and draft; preparation never grants publication permission. Owner smoke remains a
separate interactive test of the independently verified draft.

The eight case files use the recorder's schema 1 and `hosted_windows` method:

- Installation: completed silent installation, registry and PE string versions,
  and unsigned native executable.
- First start: successful measured signed-bundle initialization and default
  activation, actual catalog browsing and a nonempty source-backed plan.
- Resources: exact installed tree against source pins, signed bundle against the
  caller's build identity and compiled keys, and dependency notices/source closure.
- Performance: actual first installation, load/default activation, browsing,
  planning, hardware and scoped memory observations described below.
- Persistence/recovery: a real restart, retained profile/revision/settings, SQLite
  backup inspection/restore, integrity check, and retained profiles/settings.
- Offline: actually disable all originally-up adapters, including hidden adapters;
  repeat native startup, browsing and planning; restore the controlled adapters.
- Upgrade: install the exact public rc.1, import the explicit engineering manual
  profile and settings, upgrade with the candidate, then verify persistence and
  version-triggered bundled defaults through the native application.
- Downgrade refusal: run the older silent installer to completion within its bound;
  require unchanged candidate version and executable hash, then repeat startup and
  planning. A hung installer is a failure, never a passed refusal.

Cases become passes only after their actions and required case fields validate.
Failures are retained as failures and thrown. Missing cases stay unverified. No
owner case or historical WIN row is produced/promoted. Case JSON embeds the raw
workload reports; the existing recorder copies and hashes each case into the
source-relative evidence overlay. Additional raw JSON is retained on success or
failure, excluding the large temporary databases.

## Engineering workload and limits

`Stable-Workload.mjs` drives the installed native WebView over CDP and actual IPC.
It requires successful verified-bundle startup and the expected native version.
A private app-data directory isolates each fresh/upgrade scenario. Temporary debug
policy is limited to the owned executable value, rejects an existing value and is
removed after execution. The parent also cleans up after a timed-out child.

The profile is explicitly named `synthetic-engineering-workload`, has
`manual_entry` provenance in the admitted reference namespace, and contains no
addon/live capture claims. Only explicitly entered progress is known; unspecified
progress remains unknown. Source-backed quest-stage objectives must retain evidence
and resolved supported predicates. The limited engineering predicate set is all,
level-at-least (manual level 60), quest-state and objective-state. Unsupported or
unresolved requirements cannot become eligible.

The scenario uses a declared travel edge with evidence, resolved requirements and
a declared duration, distinct source nodes, and at least one returned travel hop.
Candidate quest location at that node, 1-second completion and utility are explicit
manual engineering assumptions, not new gameplay facts. The source edge can remain
a disclosed accepted estimate. A missing usable source goal/edge or empty native
plan fails the case. This exercises actual admitted catalogs and graph consumers;
it does not establish live-client validity or every gameplay/class/level scenario.

Metrics are monotonic actual durations. The database is fresh, but prior resource
verification and installation can warm the OS file cache; cache state is uncontrolled:

- `install_ms`: silent installer wall duration.
- `load_ms`: content receipt/resource verification, pack parsing, staging and
  composition. Database open/migration and WebView startup are excluded.
- `activation_ms`: the versioned default activation transaction.
- `browse_ms`: initial planning catalog plus actual quest/item page requests.
- `planning_ms`: the actual planner IPC call on the selected source goal/edge.
- `peak_memory_bytes`: maximum observed kernel PeakWorkingSet64 for the exact native
  application PID. WebView2 descendants and the installer are excluded. Sampling
  targets 200 ms; allocations after the last successful sample may be missed.
  Raw PID/sample/error counts identify these observations. This is not whole-process-
  tree memory or an ordinary-PC performance claim.

Startup measurements are exposed only in memory in optional successful
`DatabaseStatus.startup_measurements`, never persisted or sent. Failure does not
manufacture zero timings. Initialization runs privately before publishing ready;
status remains readable as checking, and application operations refuse partial
ready state. The UI explains that verification of large packs may take minutes.

Native operations/child runs use generous bounds up to 20 minutes, reduced by the
suite's remaining 60-minute budget. Baseline network download has a five-minute
bound; cleanup and adapter restoration have separate finite bounds. Package jobs
retain 110 minutes off tags / 170 on tags for build and regression overhead. These
limits are development bounds, not measured ordinary-Windows performance claims.
Offline control requires the hosted guard and restores only originally-up adapters
in finally, including after workload errors/timeouts. All restoration attempts run
even after one fails; inability to restore fails acceptance. Host termination can
interrupt cleanup, so these controls never run by default on a user/developer PC.

Actual hosted results remain unverified until the final reviewed source, signed
bundle, compiled build identity and installer exist. Nothing in this document or
the synthetic tests supplies a release acceptance pass.
