# Bundled source namespace binding repair

Owner/sole writer Codex `/root`; baseline963f05e1, claim2b2bb7e1.
World review identified that signed pack keys checked edition/build/interface,
fixture and revision but omitted namespace. Native installed-pack verification
already rejects a key differing from the actual reference catalog.

A controlled signed synthetic manifest changed only its pack namespace from
reference to demo, preserving fixture=false and every accepted manifest/revision/
production/material binding. The existing source gate admitted it: the new
regression failed0passed/1failed before the fix. No actual release keys were used.
The source gate now requires reference namespace alongside its existing identity
checks. This closes the earlier source-gate gap; native validation remains intact.

Actual root validation after the fix: all29CLI library tests passed, including
the signed mismatch rejection and unchanged valid reference fixture. All-target
CLI Clippy with warnings denied, formatting and whitespace checks passed. Initial
format-check difference was corrected and the final check passed. No content,
source policy, public/private identity or historical schema was changed.

Logs: /tmp/navzeroth-bundle-scope-before.log (actual controlled failure),
/tmp/navzeroth-bundle-scope-after.log, /tmp/navzeroth-bundle-scope-clippy.log.
Next: independent review, normally merge this repair into the isolated B
checkpoint, then publish B as its separate delivery PR. Actual Windows, C startup
responsiveness and stable acceptance remain unverified; no signing/publication.
