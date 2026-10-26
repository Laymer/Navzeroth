# Copy-paste continuation prompt for Claude Code

Continue the authorized Navzeroth unsigned stable1.0.0 release through public
verification and existing GitHub Pages deployment. Do not restart earlier repairs.
Read AGENTS.md, docs/STATUS.md, the latest relevant handoff and release-plan sections.
Preserve all unrelated work. Claim the continuation owner/baseline before edits.

Current checkpoint (refresh GitHub before acting):
- PR239 merged normally at aaa73b94794f5a32162b430b2314f53bca4e1a7b.
- All ten protected main checks, full source-platform parity, per-file90% coverage,
  explicit repeated stability and eight installed Windows cases passed exact main.
- Verified annotated v1.0.0 tag points to that exact source; tag object
  7d811e0f912de9bf39a1c27b096fb73688deeae0. Never move/force-push it.
- Tag package40302406210 is running. Installer build passed; remaining acceptance
  and draft creation are pending. No publication or final draft smoke yet.
- Production SHA a947d2f092f9ed4921676c008745c87c837325102db04b106460121871295537.
- Existing signed V22 content closure is complete. Do not regenerate signed inputs.

Authoritative durable progress: branch docs/v22-passed-candidate-evidence at
/tmp/navzeroth-v22-passed-evidence, especially
 docs/handoffs/2026-10-26-main-release-verification.md
 docs/handoffs/2026-10-23-schema5-draft-public-recipe.md
 release/history/2026-10-26-exact-main-gates/
Older broader context: docs/handoffs/2026-10-23-claude-release-continuation.md.
Its older main/PR/tag status is superseded by the current checkpoint above.
External checkpoint: /tmp/navzeroth-release-current-checkpoint.json.

Fixed tagged source: /tmp/navzeroth-stable-main-1.0.0 (detached aaa73b94).
WARNING: ignored dist/acceptance-evidence currently contains MAIN-installer results,
not tag results. Preserve it separately before installing actual tag overlay.
Main installer SHA774282ec42d01ba55bb9c63c4420de06aaa82ca941bdc729aa7b725cdc6148a4
must NOT be treated as final tag identity unless independently observed equal.
Original checkout /home/laymer/Devel/repos/Navzeroth retains dragon branch/unrelated
untracked data-generators/. Old release worktree retains untracked V19 claim.

Continue in order:
1. Wait for actual completed tag package/eight-case/final readiness SUCCESS. Failures
   hold release. Independently download original tag assets, cases and overlay;
   check provider ZIP digests, exact source/installer/production, every case/ref,
   source material archive/resources, notices, signed content receipts and exclusions.
2. Dispatch verify-release-downloads.yml on EXACT ref v1.0.0, version1.0.0,
   phase=draft, exact source and independently obtained tag installer SHA,
   evidence_run_id=40302406210 (only if successful and correct). No guessed IDs.
3. Wait for completed workflow/job success. Independently download real UI/native
   screenshots/reports and capture ORIGINAL completed run/job/draft-release API
   bytes. Follow the schema5 recipe exactly, including all12 identity pins.
   Obtain independent review of actual screenshots, driver, details and API bindings.
4. Construct provenance/review and ONLY ninth row in external overlay, preserving
   eight original rows and all other metadata. External helper available at
   /tmp/navzeroth-complete-draft-overlay.py; review before use. It is NOT the gate.
   Run full release-readiness from exact source. Require publication_allowed=true,
   no blockers/deferred; draft-readiness is insufficient.
5. Archive exact required bytes in separate immutable reviewed evidence commit under
   release/evidence/1.0.0/{acceptance-evidence,draft-evidence}/ with original dist/...
   references, raw bytes preserved and no private DB/profile/state exports. Verify
   transitive closure and Git blob bytes/modes. Push normally, keep tag untouched.
6. Populate /tmp/navzeroth-stable-release-body.template.md with actual final hashes,
   measurements, limitations and proof. Public verification is still pending before
   publication; do not claim future success. Publish without another approval ONLY
   after full applicable gates pass.
7. Dispatch public verifier on exact tag with exact source/hash and independently
   reviewed_evidence_commit; evidence_run_id must be EMPTY. Verify public assets
   and actual completed result; retain original draft-time API/proof unchanged.
8. Update/deploy existing website through reviewed PR/protected checks. Prepared
   copy is /tmp/navzeroth-site-v20-copy at911419e14580fdb3bb4daa741a0a1252007bc8ca.
   Use actual final tag/public metrics and asset metadata, not main observations.
   Preserve historical RC screenshots, assets/capture-release.json and captions.
   Keep scripts/site and workflows unchanged unless a real defect requires repair.
   Run site metadata tests and full validator; --draft is not deployment acceptance.
9. Final handoff records actual merges/reviews/run URLs/hashes/measurements,
   public downloads and deployment, exclusions and remaining unverified scope.

User authority/decisions persist:
- Related normal merges, release publication after gates and website deployment
  are authorized. No further approval needed. No purchases/quota expansion.
- ADR0009/schema5 replaces mandatory owner manual smoke with independent review
  of automated EXACT downloaded-draft real UI/native smoke. Supplemental personal
  Windows11/0.8.1 smoke is UNPERFORMED, not required or passed. Historical schema4
  owner semantics are unchanged. Relevant product changes invalidate smoke.
- Historical Forever70245 inputs admitted with TRUE build/warning; target70450.
  1.12 historical admitted; Classic Era permitted but not shipped.70532 unadmitted.
- Issue284 Postgres is unspecified FUTURE idea.283 newer build notification deferred.
- Keep unknown/unsupported/restricted/stale/false/zero distinct. No live-game bridge,
  memory/injection/packet decoding/input automation. No live-client acceptance claim.
- Installer/application deliberately unsigned; content signed by two operator-owned
  keys, with separately recorded independent review, not independent key custody.
- Keys /home/laymer/.navzeroth-release-keys; never print/commit them. No key use is
  expected for documentary/evidence/website work.
- Preserve 90% per-file,20-minute workloads,60-minute suite limits. Continue-on-error
  step conclusions never replace actual producer outcomes. Missing checks unverified.
- Hosted resource figures exclude documented processes; no ordinary-PC claim.
  ADR0010 firewall isolates app/WebView/driver external access, keeps runner/VM agents,
  DNS/loopback. Never call it whole-VM airgap or all-adapter disablement.
