# V15 watchdog candidate preparation

Owner /root; claim e32a31bf baseline3314be64. V14candidate97a81de9, actual
PRcheckout8d9fc3922e3433b45755ef68602a62c054faee4e, failed Windows package39497088109
at21:05:43UTC in a synthetic PS7 helper test before installer/native/all8stablecases.
The parent timed out and the child's progress file was absent; precise hosted
startup cause remains unproven. The unmodified test passed locally on actual native
WindowsPS7. V14 signed identities and exact failure are archived3314be64 before
integration. Source39497088032 actually passed full corpus twice on both platforms,
committed bytes and parity; missing-source fallback was skipped. Remaining V14CI/
coverage results are still pending at this recording, not claimed passed.

PR264 final6aaa46398183736f20cc850110088814645a08a2/test21eebc3e has independent
root/tooling CLEAR review. Root independently executed actual native WindowsPS7.6.7
with repo-pinned WindowsNode24.15.0 and LinuxPS7.6.6/Node24.19.0:26checks each PASS.
Tooling independently Linux26PASS. Author actual Windows26/Linux26/PS5compat25PASS;
PS5 explicitly does not run PS7-only parent regression. Distinct synthetic reports
exercise delayed5s startup under actual2s negative and15s positive parent watchdogs:
negative retains false watchdog/no checkpoint; positive retains pending IPC,
nonpassing progress/watchdog and absent final marker after real kill. Exact timeout
exceptions remain mandatory. Production helper/driver/runtime and20/60/170minute
budgets are byte-unchanged. No actual installed recovery or cold-start cause claim.
Root execution logs /tmp/navzeroth-root-watchdog-{native,linux}.log SHA256 respectively
4899433194d65f3f5cca11337175bc0864f80d0de861d0eb14c7d710ebc807d1,
33d457017d390d2cd81ef8606e9981a06c08b97c7c10b93012d7771f619553fd.

PR264 normally merged7667e18c8e3d0d2c1c5d9d49bcb38410c3a5e0ac; root integration
cef17297e3b03e6f2f0470840f3c44ed0ea76310 preserves prior archive and claim commits.
New tested source is cef17297; production fingerprint
6b95b6ba0ffa57f967e0a855e00fef88646ab349966c7bf828736797ad0d4627.
Functional proof remains actual executed d636, SHA256d9c409844872f8369050cd22054aa9a99c498ce6c015f0ab4ca1e0e97bb72070;
no test-only repair restamping or unnecessary runtime execution. Canonical V14
outputs removed only after exact archive-byte comparisons; bundled_content cleared,
all9canonical observations remain pristine unverified. Next independent public
review, actual preparation/reproduction, commit identities before key use and signed
closure, then fresh exact-candidate allten/source/Windows gates. Remote intermediate
7667 has stale V14 bindings and is not a valid release candidate. Main unchanged;
no tag, owner acceptance, publication or website deployment.

Actual subsequent V14 outcome: CI39497088081 and coverage39497088020 were cancelled
when root merged264 before their Windows collectors finished. The PR concurrency
policy cancels older runs. Root did not explicitly cancel these V14 runs, but the
merge triggered their replacement; this was an avoidable coordination mistake.
V14CI8jobs succeeded, RustWindows cancelled, stability skipped; no full CI pass.
Root explicitly cancelled stale-binding intermediate7667 runs39498793266(source),
39498793271(package),39498793347(coverage),39498793308(CI) to avoid spending runner
resources on invalid bindings. Pages intermediate validation succeeded without
deployment. These intermediate runs provide no candidate acceptance. The final
newly bound candidate will receive fresh complete checks without intervening pushes.
