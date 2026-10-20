# Navzeroth field manual

Static publication files live under `docs/site/`, within the existing documentation
exclusion from application production fingerprints. Only website documents and
assets belong here; application, pack and release tooling inputs remain production
files. No fingerprint policy changes are made.

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

The owner explicitly requested restoration of `assets/dragon-citadel.jpg` on
2026-10-19. It is the exact previously supplied decorative artwork, restored from
commit `44df5b8`; it is not a game screenshot or release evidence. The earlier
handoff records unresolved provenance; this request does not establish a new
licence. Dark overlays preserve text readability. `assets/dragon.svg` is an
original vector dragon favicon in the site’s gold/slate palette. The local Inter
font and legacy app icon are retained from the repository.
Public RC verification is recorded in `release.json` and the repository publication
receipt. Actual release-app screenshots were captured by hosted run 39249569950
from the verified public executable. Exact synthetic input and image provenance
are retained in `assets/screenshots/`; normal UI actions dismissed onboarding
and collapsed optional reference controls without replacing content or styles.

Inter OFL notice is retained byte for byte at `assets/fonts/OFL-Inter.txt`
from `apps/desktop/src/theme/fonts/OFL-Inter.txt` (SHA-256
`262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a`).

Publication identity contracts:

- `release.json` describes the current independently verified public download.
  Versioned metadata supports only policy-admitted unsigned 1.0.0-rc.1 and stable
  1.0.0, with exact tag, source, asset set and hashes. Stable includes its material
  archive, RELEASE_MATERIALS.json and RELEASE_ASSETS.json; release notes retain
  the packaging name CANDIDATE_NOTES.md. Validation does not perform network
  verification; retain the actual public verification receipt before updating it.
- `capture-release.json` separately preserves the verified release used for the
  displayed synthetic screenshots. Its identity must match screenshot provenance.
  When it differs from the current download, captions must state that release
  version and label it historical. Never relabel an RC screenshot as stable.
- The optional Pages `capture_release` dispatch remains explicitly a historical
  RC capture, using `capture-release.json`; it does not verify or capture stable.
- After stable publication, update only `docs/site/**` (copy, release metadata,
  measurements, images and provenance) to preserve the tagged application identity.
  Keep `scripts/site/**`, workflow code and other production files unchanged.
  Main-only gated deployment remains required; no draft-validation deployment.

Run `node --test scripts/site/release-metadata.test.mjs` before browser checks.
