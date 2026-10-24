# Downloaded draft Windows gate implementation

Owner: root. Baseline `9cc774e4f2656f26d544071d6bd174367ccca16a`.
Production implementation: `eee8344f009b4e6fca49854cd3b941dbc0c6321c`.

PowerShell release consumers now accept schema5 alongside historical schema4.
The eight-row hosted recorder cannot create either final smoke observation.
Overlays must preserve the immutable source contract; only schema5 admits the
separately checked downloaded-draft proof. Installed-resource checks cover both
schemas. Preparation report versions must match the actual inventory schema.

The download verifier still checks unsigned installer/application status, all
release assets, exact source, addon contents, installed resources and notices.
For a schema5 draft it then runs the separately owned native UI driver in fresh
and restart modes against that installed executable, with isolated profile data.
Source/installer identities and actual completion are checked at the parent
process boundary. The four final native checks require actual fresh and restart
results. Private profile state is excluded from bounded retained detail files.
The parent removes its one initially absent WebView2 debugging policy value even
when the bounded child is killed. This code refuses ordinary/self-hosted machines.

The workflow must be dispatched on the exact release tag: checking out a selected
commit from a different workflow revision cannot qualify. Raw evidence binds the
draft release ID, installer asset ID/name/size/hash, source/production identity,
workflow bytes, and run/attempt/job. This running job does not attest its own
eventual success or create an independent review. The coordinator must retrieve
completed trusted API snapshots and obtain independent review after completion;
the schema5 CLI verifies their exact byte references and consistent identities.
These observations remain external to the already signed bundle after tagging.

The new job envelope is 90 minutes for dependencies, source/static checks and
native UI work. Individual workloads remain capped at 20 minutes and the native
suite at 60 minutes. No existing workload bound, coverage gate or required check
was relaxed. Partial diagnostics are retained on failure and never count as pass.

Actual focused checks:

- Linux PowerShell7.6.6: 49 stable preparation, 27 contract bound, 37 existing
  producer/helper and 24 new downloaded-draft synthetic checks passed.
- Native Windows PowerShell5.1: 49 stable preparation, 27 contract bound and 18
  new downloaded-draft synthetic checks passed. Six PS7-only process/orchestration
  checks were explicitly unexecuted on PS5.1.
- Native Windows PowerShell7.6.7 with Node24.15.0: all24 new downloaded-draft
  synthetic checks passed. These never installed an app or disabled networking.
- Node installed-tree tests: all3 passed, including exact schema4/schema5 bundles.
- Actionlint passed for both changed workflows; `git diff --check` passed.

An initial new test fixture used multiline build-identity JSON and correctly
failed the existing canonical-format validator. The fixture was corrected to the
actual single-line format; the production validator was preserved. One patch
attempt used an absent workflow label and changed nothing; the exact file label
was read before applying the corrected patch.

No new native application, downloaded installer or public release was exercised
by these synthetic tests. The world reviewer owns the new UI driver; tooling owns
schema5 core/CLI/preparation. Independent review of this frozen implementation and
integration with those branches are still required. Next: resolve findings, normal
merge reviewed repair PRs, regenerate source-bound release evidence/receipts, run
all candidate gates, then exact-tag draft automation and publication gates.
