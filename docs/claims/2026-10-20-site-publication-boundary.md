# Stable website publication boundary

Owner / sole writer: `/root/world_review`.
Baseline: `0376a265`; branch `fix/stable-site-publication`.

Move only static site assets into `docs/site`, preserving existing public RC and
capture identities. Adapt Pages/validator paths and strictly version publication
metadata before tagging, so later website-only publication updates do not change
the existing production fingerprint. No application/pack inputs move, no policy
exclusions change, no release publication, signing or deployment is authorized here.
