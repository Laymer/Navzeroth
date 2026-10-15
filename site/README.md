# Navzeroth site

Static one-page site for GitHub Pages. No build step and no runtime dependencies:
plain HTML, CSS and a little vanilla JS. Preview it with any static server, for
example `python3 -m http.server --directory site 8000`.

- `index.html` is the whole page. The recreated desktop screens are copied from
  `apps/desktop` as they render, filled with the repo's labelled synthetic fixtures.
- `assets/css/nocturne.css` holds the Nocturne design tokens and components
  (Inter is self-hosted from `assets/fonts/`, so a visit makes no third-party request).
  `assets/css/site.css` holds page-level rules; `assets/js/site.js` the tour, FAQ,
  Atlas layer demo and scroll reveal.
- `assets/demo-atlas/*.png` and `assets/app-icon-128.png` are byte-identical to
  `content/demo/atlas/synthetic-continent/layers/` and
  `apps/desktop/src-tauri/icons/128x128.png`. Do not use the owner-supplied
  `content/forever/atlas` imagery here; it has separate terms.
- `assets/dragon-citadel.jpg` is the hero backdrop supplied by the owner with the design.

Factual claims and the status table are quoted from `README.md` and `docs/STATUS.md`;
update them when those change. `.github/workflows/pages.yml` publishes `site/` on pushes to
`main` that touch it. The repository's Pages source must be set to "GitHub Actions" once.
