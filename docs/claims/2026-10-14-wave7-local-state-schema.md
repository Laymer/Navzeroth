# Claim: wave7 shared local state schema and recovery foundation

Owner: Codex /root, sole foundation writer. Baseline published/protected main
739a82229a5b7369cab2a6d83c4b2a935bd0ba2e, v0.6.0 published2026-10-14T12:49:07Z.
Branch feat/local-state-schema; worktree /tmp/navzeroth-wave7-storage.
Read STATUS, v0.6 handoffs, wave7 coordination, current migration/content/recovery
modules and plan8.1/8.6/19.3. Shared schema design independently reviewed11SQLite
probes. Scope: migration9/four bounded tables, backfill/stage initial dispositions,
eligibility/activation-order boundary, trusted recovery table/seed copying and
fail-closed feature-document recovery until feature validators join. No feature UI,
moderation transitions or Today state changes. Freeze content.rs before releasing
ownership to the quarantine worker; other feature modules remain worker-owned.
User authorizes granular pushes/separate PR/protected integration and recurring
verified releases. Schema/input identities and evidence confidence stay distinct.
