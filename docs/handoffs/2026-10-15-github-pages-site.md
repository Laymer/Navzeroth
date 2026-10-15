# GitHub Pages site handoff

Owner / sole writer: Claude Code cloud session (`docs:`, no task ID). Branch
`laymer/sharp-ritchie-jyscxk`; baseline `5146a32`.

Integrated the owner's Claude Design "Navzeroth" one-page site as a static
GitHub Pages site in `site/`, published by `.github/workflows/pages.yml`
(`actions/configure-pages`, `upload-pages-artifact` path `site`, `deploy-pages`,
SHA-pinned like the other workflows; triggers: `main` pushes touching `site/**` or
the workflow, plus `workflow_dispatch`). See [site/README.md](../../site/README.md).

Decisions:

- The design ships as a React "Design Component" that loads React and Babel from
  unpkg at runtime. It was converted to plain HTML, CSS and about 150 lines of vanilla
  JS instead, keeping the design's markup and inline styles so the page stays
  identical to the design. The Nocturne stylesheet is copied unchanged apart from the
  font import.
- Inter is self-hosted (latin subset, `assets/fonts/inter-latin.woff2`). The design
  linked Google Fonts, which would make every visitor's browser contact Google.
- The newer top-level design file was used, not the older copy under the handoff
  `reference/` folder: it adds the dragon backdrop, embers and scroll scrim.
- Demo atlas PNGs and the app icon are byte-identical to the repo files (checked with
  `cmp`); no `content/forever` imagery is used.
- Removed two design-tool leftovers: the image-slot drop zone in the HUD tour tab
  (now a plain empty frame) and the caption sentence "Drop a screenshot from your own
  client behind it."

Actual checks (Chromium via Playwright, Linux only):

- Full-page and per-tab screenshots at 1400px (reduced motion) were compared with the
  original design rendered by its own runtime: 0 differing pixels on the full page
  and five of six tour tabs; the HUD tab differs only by the design tool's dashed
  drop-zone outline.
- Interactions with motion on: tour tabs and in-window nav buttons, screen/caption/stage
  swap, slice offsets, Atlas layer toggles (Flight replaces Roads; neither shows the
  overview), FAQ (several open at once), scroll reveal and embers; no console or page
  errors. At 390px width there is no horizontal page scroll.
- No absolute paths, no third-party requests, no duplicate IDs, all in-page anchors
  resolve, and every linked repo file and the `v0.8.1` tag exist.
- The status table cells were re-checked against README.md on this baseline.

Unverified / for the owner:

- The workflow has not run. Pages must be enabled once under Settings > Pages with
  source "GitHub Actions"; the first deploy happens after this reaches `main`.
  The expected URL is https://laymer.github.io/Navzeroth/.
- Hero artwork (`assets/dragon-citadel.jpg`, from the owner's upload) has no recorded
  provenance or terms. Confirm it is yours to publish. The design used no Blizzard art
  or logos, and the site has no "not affiliated with Blizzard" line (the design left
  it out; consider adding one).
- No in-game HUD screenshot was supplied; the HUD tab shows the plate on an empty frame.
- Site copy quotes README.md and docs/STATUS.md at this baseline and will go stale as
  the 1.0 work lands (for example the "Download 0.8.1" button).

Next: owner enables Pages, then a human reviews the live site; update the copy and
download button when a new release is published.
