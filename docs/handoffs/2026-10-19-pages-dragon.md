# SITE-DRAGON — citadel background and favicon

Owner/sole writer: Codex `/root`; baseline `6901cf2`, branch `fix/pages-dragon`.
Claim: [task ownership](../claims/2026-10-19-pages-dragon.md).

Restored the exact previous owner-supplied dragon-citadel JPG at the user's
explicit request. Shared CSS displays it behind all five pages with dark reading
overlays and a mobile crop. Added an original gold/slate vector dragon favicon
on all five pages. Updated the site README and preview server JPEG/SVG MIME types.
Release screenshots, provenance and release metadata are unchanged.

Actual checks: final `node scripts/site/validate.mjs` passed all five pages at
360/768/1440px, including release/capture bindings, links/assets, keyboard,
semantics, clipboard and no-JavaScript navigation. Chromium previews at 390 and
1440px and the SVG icon were visually inspected; a browser check confirmed the
background CSS and favicon reference. No native or live-game checks apply.

The historical artwork provenance limitation remains recorded in site/README.md;
the user's restoration request is not a new licence assertion.
Work is isolated in `/tmp/navzeroth-pages-dragon`; original untracked files remain
untouched. Next: review the draft PR, authorize protected-main integration, then
verify the gated Pages deployment. Live deployment is not claimed here.
