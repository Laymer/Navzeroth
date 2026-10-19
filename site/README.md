# Navzeroth field manual

Five static HTML pages, local CSS/font and optional clipboard JavaScript. No
runtime dependencies or external asset requests. Hosted under `/Navzeroth/`.

Development check: `node scripts/site/validate.mjs --draft` after locked pnpm
installation and Playwright Chromium setup. This explicitly skips public-release
and capture binding, and must never be used in deployment CI.

Final check: `node scripts/site/validate.mjs`. It requires `release.json` with
verified public asset identities and screenshot provenance bound to that release.
The Pages workflow runs it on relevant PRs and gates main-only deployment on success.
Validation covers all five pages at 360/768/1440px, relative links/fragments/assets,
project base paths, overflow, landmarks, alternatives, visible keyboard focus and
reachability, clipboard feedback and navigation with JavaScript disabled. These
checks do not constitute a full manual accessibility certification.

Screenshots must come from the actual verified release executable with labelled
synthetic fixtures. Preserve capture source/fixture/executable/installer/image
hashes, timing, dimensions and workflow provenance. Recreated screens and game
art are not substitutes. No owner-supplied game imagery or personal exports ship.

The previous dragon backdrop had unresolved provenance in its handoff and is no
longer included. The local Inter font and app icon are retained from the repository.
Public RC asset verification and real-app captures remain required before the
separate Pages PR can be delivered.
