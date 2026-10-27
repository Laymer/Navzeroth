# Windows schema 4 contract bounds

Owner desktop_review; isolated branch fix/stable-schema4-windows-contracts, baseline ab16316bfbc1038976526489a847942527c3c408. Claim: docs/claims/2026-10-20-schema4-windows-contracts.md.

Measured final contracts exceed the generic historical 16 MiB reader: inventory 27,957,511 bytes and evidence 27,969,970 bytes. Added a specialized inventory/evidence reader: explicit integer schema 4 permits at most 64 MiB; schemas 1–3 retain 16 MiB; missing, noninteger and future schemas refuse. The shared internal stream reader requests at most cap+1 bytes and rejects overflow before decoding, without trusting a length precheck. Existing regular-file/reparse traversal checks remain; strict UTF-8 decoding retains UTF-8 BOM support and counts the BOM in the byte budget. Generic artifact/manifest/case readers remain 16 MiB. Only inventory/evidence intake calls opt into the special reader. Hosted evidence output uses compact JSON and checks UTF-8 bytes against the same versioned bound before writing. JSON semantic/source/acceptance checks remain the existing CLI and overlay validators; this helper does not grant acceptance.

Actual Windows PowerShell 5.1.26100.9711 checks:
- Test-ReleaseContractBounds.ps1 -ActualRoot <immutable final-content worktree>: 26 passed, including exact 16/64 MiB boundaries, cap+1, all historical versions, unknown/noninteger versions, invalid UTF-8, BOM byte accounting, Unicode serialization overflow, directory refusal and actual evidence roundtrip preserving source/check records.
- Actual inventory SHA256 f7030627acbc61fbbec32deeacc9bfc4dc1f8adc8fb6a183771559c656ce9eac; evidence SHA256 61c3f4ce220b93042dc039d83553bc0ad5073a7807b4ee33d2dca7ab4c2c91a8. Both read as exact unchanged bytes from final-content a14b8b02643c978bbd34bc83626adf0d160b168b.
- Existing Test-StablePreparation.ps1: 42 passed. Test-StableAcceptanceHelpers.ps1: 10 passed. Native PS5.1 parser accepted all four changed/new PowerShell files. git diff --check passed.
- Test-ReleaseMaterials.ps1 could not start its assertions: native Windows Git is absent from PATH. This archive regression remains unverified locally; no pass inferred. PowerShell 7 is unavailable locally; hosted workflow should run the new portable boundary suite under both powershell and pwsh.

Commands used powershell.exe -NoProfile -ExecutionPolicy Bypass -File with WSL UNC paths; policy override was process-local for synthetic helper tests only. No installation, adapter changes, signing, status promotion, source-pack modification, hosted acceptance or manual owner smoke was performed. Outputs stayed in temporary test directories and were removed. No push.

Next: independent review and normal integration; workflow owner wires Test-ReleaseContractBounds.ps1 under both Windows shells. Separate workflow startup allowance/producer integration and restart preference assertions remain outside this repair.
