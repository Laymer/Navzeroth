# Stable website publication boundary

Owner / sole writer: `/root/world_review`. Baseline `0376a265`, branch
`fix/stable-site-publication`, isolated `/tmp/navzeroth-stable-site-publication`.

Before tagging stable, relocate only the static website from `site/` to
`docs/site/`. The production fingerprint policy remains unchanged: `docs/` and
`release/` exclusions retain their existing meaning. Application, source packs,
release tooling, Pages workflow and site validator stay production-bound. This
one preparation change requires renewed production review/receipts/checks before
release. After publication, website copy/assets/metadata changes exclusively in
`docs/site/**` do not alter tagged application production bytes. This is not a
waiver of protected-main checks or main-only gated Pages deployment.

All original eighteen non-README site resources were compared with the baseline:
HTML/CSS/images/font/captures/provenance remain exact bytes. `release.json`
retains every original public RC identity/hash, adding explicit versioned metadata
(schema1, version, prerelease and unsigned policy). README explains the boundary.
A separate `capture-release.json` retains that same RC identity, allowing the
current download to advance later without falsely relabelling RC screenshots.
No new stable-public or installer-acceptance claim appears in the site.

The publication validator supports only source-policy-admitted unsigned
`1.0.0-rc.1` and `1.0.0`. It requires exact version/tag, prerelease/signature
policy, public verification run, source/executable hash, positive asset sizes,
exact asset URLs/hashes and the exact version-specific asset set. Stable has
three additional material/source assets; release notes retain the packaging name
`CANDIDATE_NOTES.md`. Separate screenshot identity must match existing provenance;
each displayed caption states its exact captured version, synthetic status and,
when different from the current download, historical status. Existing capture
workflow remains an optional historical RC capture, reading the dedicated capture
metadata; it is not stable capture automation. Deployment still requires final
validation, never `--draft`, and runs only on main.

Actual checks:

- `node --test scripts/site/release-metadata.test.mjs`:4 tests passed, including
  synthetic stable complete-set acceptance, policy/asset/pin refusal, and historical
  RC caption refusal when incorrectly labelled as current stable. Synthetic data
  is test-only and is not public stable evidence.
- `node scripts/site/validate.mjs --draft`:passed five pages at360/768/1440px,
  links/assets, keyboard/semantics/clipboard/no-JavaScript navigation. Draft mode
  explicitly does not claim public-release/capture validation.
- `node scripts/site/validate.mjs`:passed the same full browser checks with actual
  existing RC release metadata and all existing fixture/image/capture hash checks.
  Repeated after final caption helper changes. This consumes existing recorded
  public identities, not a new network download or native executable run.
- `/tmp/navzeroth-actionlint/actionlint .github/workflows/pages.yml`:passed.
- `git diff --check`:passed. Existing locked desktop Playwright dependency tree
  reused through a temporary task-local symlink, removed afterward; no install,
  dependency/lock update, world acquisition or quota expansion occurred.

Logs: `/tmp/navzeroth-site-publication-{metadata,draft,final}.log`.
No application/pack/version/fingerprint-policy changes, no deployment, signing,
private-key access or release publication. Existing artwork provenance limitations
remain in README; moving files does not grant new rights. Original checkout and
unrelated trees remain untouched.

Next: independent review and separate draft PR against the version branch; parent
integrates only after review, renews exact production bindings and checks before
final tag. After verified stable publication update only `docs/site/**` with actual
public hashes/source, supported scope/exclusions/origins/estimates, measured Windows
limits and honest historical capture captions, then verify gated main deployment
and public pages/downloads. Tooling/workflow changes after that would require a
new production identity and cannot be passed off as website-only documentation.
