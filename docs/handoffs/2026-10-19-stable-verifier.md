# STABLE-VERIFIER handoff

Owner/sole writer: Codex `/root`; baseline `826aa21`; isolated branch
`fix/stable-download-verifier`. Claim recorded before implementation.

## Implemented

Versioned artifact policy schema 1 explicitly admits unsigned 1.0.0-rc.1
(prerelease) and unsigned 1.0.0 (stable). Other stable releases retain signing
requirements. Policy validation refuses invalid versions, inconsistent release
flags, duplicates, unsupported signatures and unknown schemas.

Download workflow accepts an explicit version, validates the committed policy,
exact tag/checkout/source identity and requested draft/public state, then retains
existing exact assets/checksums/installer-hash/source/resource/NotSigned checks.
Verification receipt schema 2 records declared signature and prerelease status.
Tag signing and prepared-artifact rehearsal consume the same source policy;
configured signing commands/test-signing cannot contradict unsigned declarations.
No readiness/material gate, version declaration or acceptance result is waived.

## Actual validation

- Native Windows PowerShell 5.1 Test-ReleasePolicy.ps1: 17 checks passed.
- Native Windows PowerShell 5.1 Test-Signing.ps1: 11 checks passed (synthetic PE,
  not real signing or installation).
- node --test scripts/windows/verify-installed-tree.test.mjs: one mutation test
  passed (changed/missing/extra/linked resources).
- git diff --check: passed.

Hosted Windows packaging/download installation and PowerShell 7 remain unverified
until workflows run. No new installer, tag, draft assets or publication created.
Independent review and all refreshed required checks precede merge. Actual stable
publication still requires complete content/receipt closure, versioned acceptance,
exact-commit checks and the owner's exact-draft-installer smoke.
