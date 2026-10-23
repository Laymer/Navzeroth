# ADR 0010: Hosted offline acceptance by firewall isolation

Accepted 2026-10-23 by the owner during the PR239 candidate repair.
Baseline: `2c6fe762d975a558542230b9383a990948afd9eb` (V20 candidate).
Owner: Claude Code release continuation.

## Context

`HOSTED-OFFLINE` disabled every network adapter on the GitHub-hosted runner for
the whole offline workload. With the full V20 pack that workload needs about
270–310 s (content load alone about 228 s). Three package attempts were
cancelled by the platform shortly after the adapters went down, with no user
cancellation annotation, and the later cases never ran:

| Run/attempt | Offline started | Cancelled | Into offline |
| --- | --- | --- | --- |
| 39641767585/1 (V19) | 18:34:26Z | 18:39:04Z | 278 s |
| 39931563703/1 (V20) | 10:50:13Z | 10:55:19Z | 306 s |
| 39931563703/2 (V20) | 13:11:52Z | 13:16:32Z | 280 s |

V17's offline workload finished in 267 s and passed. The likely mechanism is the
runner losing its job lease while it cannot reach GitHub. That cause is inferred
from this timing pattern; no runner diagnostic log confirms it.
Attempt 1 of run 39931563703 was later attributed to a manual cancel by the
owner; attempt 2 was not. The final gate correctly refused every cancelled step.

## Decision

For hosted acceptance, isolate the runner with Windows Firewall instead of
disabling adapters:

- Every profile is enabled with default-deny inbound and outbound.
- Every enabled persistent allow rule is disabled. Only the isolation group's
  rules may remain enabled. They allow outbound traffic for:
  - the exact `Runner.Listener.exe` and `Runner.Worker.exe` executables found in
    the current process ancestry;
  - the hosted VM's own agents when present (`hosted-compute-agent`, `provjobd*`,
    `WaAppAgent`, `WindowsAzureGuestAgent`), by exact path, each refused if it
    matches a workload executable;
  - UDP/TCP port 53 for the DNS client service (`Dnscache`).
  All program paths are expanded to their long form, because the firewall
  refuses 8.3 short paths.
- An external TCP connection to an address resolved beforehand must succeed
  before isolation, be refused during isolation and after the workload, and
  return after restoration.
- Enabled outbound allow rules in the effective (ActiveStore) policy outside the
  isolation group are inventoried. Isolation is refused if any rule is unscoped
  (any program and any service) or names `azeroth-desktop.exe`,
  `msedgewebview2.exe`, `node.exe`, `pwsh.exe` or `powershell.exe`.
- Cleanup restores the profiles first, then re-enables the snapshot rules, and
  removes the isolation rules last. If the profiles cannot be restored, the
  runner's rules are kept so the job can report the failure. The restored state is
  re-read and compared to the snapshot. Cleanup failures never replace the
  original failure.

The case records `network_isolation=windows_firewall_runner_and_host_agents_only`,
`network_isolated`, `external_connection_refused`, `network_restored`,
`allowed_programs`, `allowed_infrastructure_programs` and `dns_allowed`. It does not claim `network_disabled`.
The validator accepts exactly one of the two shapes and refuses mixtures.

## Consequences and limits

- The direct proof is a refused TCP connection from the PowerShell process that
  drives the case, plus the absence of any persistent or effective allow rule that
  would admit the app, its WebView2 processes or the Node driver. Under
  default-deny that makes the app's external connections fail. It does not prove
  the machine had no network at all: the runner and hosted VM agents stayed
  connected and name resolution through the system DNS client remained possible.
- Loopback stays available, as it was under disabled adapters.
- Service-scoped effective allow rules (for example Dnscache) stay active. They
  apply only to their service, not to the test processes.
- Historical adapter-based cases, including V17's passing `HOSTED-OFFLINE`,
  keep their `network_disabled` meaning. The adapter helper and its engineering
  probe remain for that purpose.
- The 20-minute workload and 60-minute suite bounds are unchanged.
- The opt-in `windows-offline-firewall-probe.yml` holds isolation for 1260 s,
  past the 1200 s workload bound. It is engineering evidence, never an
  acceptance observation.

## Engineering probe results

- Run 39972105651 (`674e1bd6`): isolation and exact restoration worked (180 allow
  rules disabled, four isolation rules left), but the probe's own hold timer
  replaced its case-insensitively equal `$Hold` parameter and failed after 38 s.
- Run 39972424100 (`19f9ca28`): passed. The job survived 600 s of isolation with
  40 checks of refused external TCP and working loopback, 177 allow rules disabled,
  exact restoration and returned connectivity. Its 35 s "restore" figure was
  mostly a per-rule re-verification loop (one firewall query per snapshot rule)
  sharing the 60 s connectivity window. Independent review found this. The rules
  are now read once and connectivity has its own timer. The allowed agents were
  `C:\actions-runner\cached\2.338.0\bin\Runner.Worker.exe` and `Runner.Listener.exe`.
  Before isolation, `hosted-compute-agent`, `WindowsAzureGuestAgent`, `WaAppAgent`,
  `provjobd`, `lsass`, `svchost`, Defender and the Start menu host also held
  connections.
- Runs 39975150461 (`a4f93b69`) and 39975523710 (`c0c2f6c6`), both runner-only
  with a 1260 s hold, were lost. Neither recorded a step result, logs or artifact,
  and both were cancelled at the 40-minute job limit. Runner-only isolation keeps
  the job for at least 600 s but not for 1260 s, so the hosted VM agents are now
  allowed too.
- Runs 39982055447 and 39982387411 refused to isolate with "The parameter is
  incorrect" and restored cleanly. Run 39982567901 created disabled test rules
  and showed that the firewall refuses `provjobd`'s 8.3 short path
  (`C:\Users\RUNNER~1\...`) and accepts its long form. `hosted-compute-agent`
  (no extension) and the Azure guest agents were accepted as given.
- Run 39982856528 (`c9d0d418`): passed. The job survived 1260 s of isolation, past
  the 1200 s workload bound, with 84 checks of refused external TCP and working
  loopback. Results: 177 allow rules disabled, exactly eight isolation rules
  enabled (two runner, four host agents, two DNS), zero other effective outbound
  allow rules, exact restoration (re-verification 421 ms), and connectivity back
  within 8 ms.
