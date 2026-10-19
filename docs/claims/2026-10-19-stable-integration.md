# STABLE-1.0.0 — integration and publication claim

Owner and integration writer: Codex `/root`.
Date: 2026-10-19.
Baseline: `826aa21862f7ef15d290c453d5bab3caa7806474` (`origin/main`).
Branch: `release/stable-1.0.0-integration`; worktree `/tmp/navzeroth-stable-integration`.
Original checkout and all existing worktrees remain preserved.

Scope: implement the owner's supplied stable-release plan; review and integrate
#214, #215, #216, #217 sequentially, then #212 with required repairs; record a new
ADR and version release contracts; complete content/desktop/release work in
separate reviewed PRs. Defer unrelated new PRs. The three explicitly requested
specialists review read-only; one writer per repair will be assigned separately.

Acceptance: final-head independent reviews, ten protected-main checks, source
platform parity and stability; truthful inventory/origin/receipt closure; hosted
Windows exact-installer verification and owner manual smoke before publication.
Missing checks remain unverified. No tag or publication before applicable gates.
Private exports and signing keys stay outside Git. Published history is preserved.
