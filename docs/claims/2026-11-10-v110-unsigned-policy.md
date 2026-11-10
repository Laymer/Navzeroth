# V110.9c — explicit unsigned1.1.0 artifact policy

Owner/sole writer Codex `/root`; baseline `a410133e`; isolated PR #297 worktree.
Implement ADR0011's already approved unsigned exception in the machine-readable
artifact policy. Preserve historical entries and reject unlisted versions. Check
PowerShell policy state/identity tests and website metadata validation. This does
not sign content, satisfy acceptance, change release versions or permit publication.
