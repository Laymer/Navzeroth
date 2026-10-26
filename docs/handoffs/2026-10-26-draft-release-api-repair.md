# Draft release API lookup repair

Owner Codex/root; baseline aaa73b94794f5a32162b430b2314f53bca4e1a7b.
Claim docs/claims/2026-10-26-draft-release-api-id.md. One isolated writer.

Tag package40302406210 completed SUCCESS, including eight independently reviewed
actual Windows cases and draft creation. Draft424161437 is unsigned stable1.0.0,
installer26039435bytes/SHA1b32ba37d0527afe211a36d5d1269a9c70bb1295f24cd9e7d6f5c37759d6d68c.
It remains unpublished. Actual by-tag REST lookup returned404 twice; release view
and REST by numeric ID both succeeded. GitHub documents by-tag lookup for published
releases: https://docs.github.com/en/rest/releases/releases#get-a-release-by-tag-name

Draft verification40314686467 was dispatched on exactv1.0.0/AAA/taginstaller/package
40302406210. The coordinator shell continued to the authorized dispatch after its
initial by-tag API precheck failed; this is recorded, not presented as a passed
precheck. Subsequent independent list/by-ID and release-view checks confirmed all
8draft asset IDs/sizes/providerdigests. The hosted attempt is still running at this
checkpoint; no hosted failure or UI acceptance is claimed yet. Preserve its actual
outcome and reports even if it reaches the late by-tag lookup failure.

Repair adds databaseId to the successful release view, uses that issued positive
integer in REST by-ID lookup, and rejects malformed repository, wrong ID/tag,
public/prerelease state, nonboolean state, failed API and malformed JSON. Existing
asset/native/source/workflow checks remain. No by-tag fallback, fabricated API,
publication workaround or tag rewrite. Public-phase behavior is unchanged.

Actual validation:
-29 Linux PowerShell downloaded-draft synthetic checks passed (5newlookupchecks).
-Actionlint and git diff --check passed.
-Real read-only repaired lookup returned draft424161437/tagv1.0.0/eightassets.
-Native Windows PowerShell5.1.26100.9711 parsed both changed PSfiles:0syntaxerrors.
-Local nativePS5 test execution was refused because scripts are disabled. No
 execution-policy override/change was made. Runtime tests on nativePS5 remain
 unverified locally; parsing is not an execution pass. Hosted native checks pending.

The workflow/helper change affects production identity and signed release closure.
Existing AAA tag artifacts cannot be relabelled as repaired-source artifacts.
Independent review and renewed source/review/receipt bindings, fresh candidate/main
checks and exact downloaded-installer acceptance are required. Existing published
Git tag v1.0.0 must not be silently moved. Release identity/version decision remains
open; no tag mutation, release publication or main merge in this repair.

Independent desktop and tooling reviews CLEAR on exactcodecommit
c2052f47dd2cb8a3991e66027ef8193d915e616e. Both actually reran29Linuxchecks,
actionlint and diffcheck successfully; neither claimed nativeWindows execution
or repaired-release acceptance. Desktopreport retained in docs/reviews.
