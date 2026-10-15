# ForeverDB source replacement review

Owner/sole writer: Codex `/root`; baseline `32c5e2d`; claim `325e74d`.
Branch `feat/foreverdb-source-review`, isolated worktree.

The user selected ForeverDB instead of Wowhead. Updated current STATUS and source
admission research to name ForeverDB as the preferred candidate; marked the
unsent Wowhead request superseded. Historical source reviews remain unchanged.

Reviewed official public metadata, the advertised GPLv3 listing and the
publisher-linked Wago release. The normal CurseForge download returned HTTP 403;
the normal Wago Beta 0.34.0 download succeeded. No access-control workaround or
excluded ForeverDB `/api/` request was made. The first Wago browser attempts had
ambiguous tab selectors; exact link/button roles resolved them. No upstream Lua
or uploader was executed or installed.

The retrieved archive is 3,680,905 bytes, SHA-256
`197af97c02444077e8edd7883eeeb6328828cf9b2f06c540d5708d89894a17d0`;
176 members total 11,215,661 uncompressed bytes. Inspection enforced an 8 MiB
archive/member limit, 4,096-member and 128 MiB aggregate limits and refused
absolute/traversing member names; files were read through ZipFile, not extracted.
Committed only audit metadata and file hashes, not upstream data/code/media.

Meta.lua reports `1.60.1.70338`, data `2026-10-12.1bcd12db`, cap 30. The TOC reports
interface16001. Public website metadata separately reports 70338/extractedOct14.
Those are original source identities, not validation for our target70374.
The review maps candidate files to all nine domains without treating them as
complete coverage. Classic fallback, observed Forever facts and client facts must
stay separate; estimated times/rates and unknown coordinates remain explicit.

GPLv3 is an eligible advertised licence family. Some Classic-derived data files
have explicit GPL notices; no root GPL text or referenced generator scripts occur
in this archive. Verify corresponding-source availability and file-level scope,
then deliver the required licence/attribution/source materials. Wago uses signed
expiring links outside the existing automated admission URL subset. Do not strip
queries; use a reviewed manual acquisition declaration or supported static source.
Admission remains empty. No content, acceptance or publication gate was weakened.

Validation: JSON audit parsing, exact archive/member hash verification, referenced
research snapshot hashes and changed Markdown links checked locally; whitespace
check passes. This is documentation/research only, so no product test suite was
rerun. Actual 70374 data validation and Windows/live checks remain unverified.

Next: resolve the narrow source/export questions in the
[unsent inquiry](../sources/permission-requests/foreverdb-source-request-unsent.md),
admit specific files/fields/transforms, and implement bounded inert staging.
No publisher contact was sent or authorized by this review.

Independent continuation review: the source-admission worker verified the archive
and every member hash and found no important audit/staging blocker. Clarified
that covered GPL material needs no fresh permission; absent generators alone do
not establish the complete corresponding-source requirement. Pinned the canonical
35,149-byte GPL text, SHA-256
`3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986`,
and recorded modified-work/date, attribution, same-licence and preferred-source
obligations. PR143 initially failed the Windows 20-second first-start store check;
the rerun of the same commit passed. No product change or gate relaxation was
made to obtain that result. Follow-up docs require their own hosted checks.
