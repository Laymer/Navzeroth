# Active catalog Atlas

Owner Codex /root/acceptance; claimed before code on 2026-10-15.
Branch feat/active-catalog-atlas baseline557d86b; worktree
/tmp/navzeroth-content-consumers. Read shared instructions/status and Atlas plan,
existing Atlas implementation and previous consumer handoff. Own app Atlas query,
projection/marker methods, native Atlas commands/protocol and Atlas UI. Source
worker owns separate get_catalog_page block/registration and list pagination;
inventory owns verified asset persistence. Coordinator owns reusable pack verifier.
No duplicate verifier and no arbitrary filesystem asset lookup.

Current Atlas always queries embedded demo, even when a stored profile is active.
Implement active v1/v2 catalog query and projection with explicit scope/revision
bindings; no stale demo fallback. Preserve standalone demo APIs/fixtures. Stored
asset URLs require exact verified persisted material and remain unavailable until
that closure is installed. Verification and remaining limits follow below.
