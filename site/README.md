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
Public RC verification is recorded in `release.json` and the repository publication
receipt. Actual release-app screenshots were captured by hosted run 39249569950
from the verified public executable. Exact synthetic input and image provenance
are retained in `assets/screenshots/`; normal UI actions dismissed onboarding
and collapsed optional reference controls without replacing content or styles.

Inter OFL notice is retained byte for byte at `assets/fonts/OFL-Inter.txt`
from `apps/desktop/src/theme/fonts/OFL-Inter.txt` (SHA-256
`262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a`).
