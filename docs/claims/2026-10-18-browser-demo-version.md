# Browser demo application version correction

Owner/sole writer Codex `/root`, baseline `9d046b7`, branch
`fix/browser-demo-version`. Read STATUS and desktop handoff before changes.
Coverage preparation found the browser demo's get_app_status response still
hard-codes 0.6.0 while the package version is 0.9.2. Use the existing package
metadata import, matching the reference view. Keep this independent product fix
in its own reviewed PR; refresh the coverage branch after it reaches main.
Preserve original checkout and `.claude/`. No release or acceptance claim.
