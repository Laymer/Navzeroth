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

Implemented selected v1/v2 Atlas image queries with max200 revision/hash/profile
bound pages. Batch projection uses ProjectionIndex. Imported marker projections
use selected content, never the embedded demo catalog; marker retrieval validates
the displayed catalog token. Missing selected profile/catalog is explicit error.
Stored UI hides obsolete imagery on failed refresh while demo refresh behavior is
preserved. Existing demo golden payloads omit optional page metadata.

Installed v2 image layers advertise bounded inert token URLs. Native URI resolver
rechecks selected profile, exact active catalog and hash/PNG-verified stored bytes;
no filesystem path or imported Lua executes. Missing material remains unavailable.
Declared geometry is still displayable as a schematic without raster material.

Checks so far:15 existing Atlas Rust golden/behavior checks and three focused
stored Atlas checks pass (205images/200+5pages, marker/projection active data,
verified bytes, malformed tokens/stale digest). All198 desktop tests, typecheck
and lint pass. Native cargo check attempted; unavailable DBus development package
blocked native compilation before app code. Full app suite and strict Clippy are
rerunning after assets dependency; its lint finding notified sole storage writer.
No Windows/native/live70374 verification or complete corpus claim.

Full app suite passed after storage assets and shared pack verifier integration.
Formatting and diff checks pass. Next task: verified bundle directory importer and
bounded Content inspection transport; separate claim follows this Atlas commit.
