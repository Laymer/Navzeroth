# V18 upgrade-wait repair candidate (in preparation)

Owner root. Previous failed candidate 8b12aec1 (V17). PR 268 (uninstall completion) was merged at 2692631f; its tested production commit is d7b7474d22323a84ca37ad895d26ba8400910b25. Production SHA256 8fd5fa2982164e040bcdf999190fd307ef76ef7adddb94cdf9b8681389b21493 over 3132 entries (same method as the V17 review; the method reproduced V17's 77e88f41... exactly before it was used). Only three production files differ from V17: Native-Smoke-Helpers.ps1, Stable-Acceptance.ps1 and Test-NativeSmokeHelpers.ps1. Runtime Rust, UI, addon, content and Cargo are unchanged from the V16 functional execution at ac45a4f4, whose actual full-pack proof is kept and not relabelled.

The V17 signed bytes were compared with the committed archive (release/history/2026-10-21-signed-content-v17) before the canonical files were cleared. The nine canonical observation rows stay unverified and bundled_content is empty until fresh preparation and signing.

Checks recorded for the repair itself: Test-NativeSmokeHelpers 17 checks on native Windows PowerShell 5.1 and PowerShell 7.6.7, Test-StableAcceptanceHelpers 36 and Test-StableWorkflowGate 12 on PowerShell 5.1, workflow Node tests 6. The hosted effect is unverified.

Next: independent public content review v18, reproducible unsigned preparation committed before any key use, signing with the owner's external keys, independent closure checks, then one candidate push. No main merge, tag or publication.
