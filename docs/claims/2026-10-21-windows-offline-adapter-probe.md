# Hosted Windows offline adapter probe

Owner tooling_review. Baseline764fcac347f0c75ee628df619e9bd224ebf00b6f.
Isolated branch fix/windows-offline-adapter-probe; one writer for the NEW probe
workflow/wrapper/tests only. Desktop owns Stable-Acceptance-Helpers.ps1 and its
existing tests; integrate its independently reviewed final helper normally.

Read STATUS, V16 storage/runtime handoff and implementation plan M9 hardening.
Scope: bounded disposable GitHub-hosted Windows engineering probe of actual
network disablement and exact restoration, no product build or acceptance pass.
Contents-read only. New distinct nonprotected job. No protected workflow edits.
Opt-in push branch verify/offline-adapters-* plus future workflow_dispatch;
normal repair branch pushes do not invoke the probe. Root explicitly triggers
only a reviewed frozen combined helper/probe commit. No local adapter controls,
keys, signing, purchases, account/quota change or publication.
