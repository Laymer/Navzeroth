# Source independence without publisher replies

Owner: Codex `/root`; research2026-10-15, baseline `b1d00b9`.
This supersedes waiting for a ForeverDB reply as the next required action.
The inquiry is useful additional evidence, not a release prerequisite by itself.
A compatible published licence is the permission for the material it covers.
Unclear material can be omitted or replaced; it must not hold unrelated work.

## Findings

No researched source establishes a complete, redistributable nine-domain
Forever70374 catalog. There are useful open-source components requiring no
personal approval. Their published licences and upstream source are sufficient
starting points for scoped reuse; target validation remains a separate question.
The [audit manifest](independent-source-review.json) pins the files actually read.
These entries describe the research baseline. The subsequent
[offline pipeline](../data/SOURCE_PIPELINE.md) admits three narrowly reviewed
scopes, retains their original/licence material and automates convergence. It
does not admit the other candidates or establish complete target coverage.

| Source | Published licence inspected | Useful scope | Limitation / decision |
| --- | --- | --- | --- |
| [WoWSims Classic](https://github.com/wowsims/classic/tree/7779ebbf79dc7f1341e6ab939b28a3402c9a730a) | MIT | Authored class/mechanics models, including paladin implementation | Best permissive model-code lead. Adapt and document assumptions; Classic models are not verified Forever models. Do not infer rights to every generated item asset from the code licence. |
| [AtlasLootClassic](https://github.com/Hoizame/AtlasLootClassic/tree/8e99341e4e779328460bf7684c0d5b22ce50ddf1) | GPLv2 text | Dungeon/reputation/crafting and item-source relationships | Best scoped acquisition/dungeon baseline lead. Preserve licence/source and confirm selected-file notices; historical Classic facts require Forever validation. Its data file contains calls and branches: an adapter must explicitly transform supported constructs without running Lua. |
| [VMaNGOS](https://github.com/vmangos/core/tree/53d24d5c748bef5e0cde7b1155bcb3d979af614c) | GPLv2 text for repository | Broad historical world/schema reference; public development database snapshots exist | Candidate for a broad comparison baseline. Core licence alone does not settle every separate database asset's scope. Inspect a pinned dump/source/notices before admission. Supported builds are old1.5–1.12, not70374. Never run its server, sniffers or SQL scripts for import. |
| [Forever GuideMate](https://github.com/TylerAkins/forever-guide-mate/tree/c9bb2efc3fca96dd8b476bb8e5363b366d9ea8f8) | GPLv3 text | Authored Forever guide logic, prerequisites, class/dungeon routes | Useful Forever-specific lead. README identifies incomplete guides, unvalidated coordinates and Wowhead references. Select original licensed work only after provenance assessment; no complete inventory claim. |
| [Mapzeroth](https://github.com/tr0tsky0/Mapzeroth/tree/676241e234cbeab5e2066b869b52c235d675a9e0) | MIT | Routing implementation ideas / potentially original authored graph structure | Inspected TOC interface120100 is Retail. No Forever branch in the retrieved public branch list. Do not import its graph as Forever geography or assume all bundled libraries/media share MIT. |
| [pfQuest](https://github.com/shagu/pfQuest/tree/104f35678ca39ab1fb78b655f815cc7016f5e0c8) | MIT at root | Historical quest/NPC/item adapter reference | README explicitly derives Vanilla data from VMaNGOS and TBC data from CMaNGOS. Root MIT does not erase inherited data obligations or validate Forever. Not a cleaner shortcut around those sources. |
| [Lodestar](https://github.com/danielcosta42/lodestar/tree/5c5fa2346981eefe3f368f325b5ef80e2270d9de) | MIT | Authored guide/routing implementation ideas | QuestieDB ancestry; travel file names client70245 via Wago and QuestieDB. Do not treat all generated data as newly MIT-licensed or70374 validated. |
| [QuestieDB](https://github.com/Questie/QuestieDB/tree/49e7dbca1f18eb3b8e57ee5d9562ac3e5c9adcc4) | No root licence in complete inspected tree | Already supported inert research adapter; Forever candidate facts | Provenance documents ancestry, not a blanket new grant. Separate GitHub tree remains unadmitted; a licence on some other package must be tied to exact covered files before reuse. |
| [Forever Guides](https://github.com/forever-guides-com/forever-guides/tree/c986c13d8e6018294bea7e9535be741bbac91fcf) | Custom source-available, restricted | None for an autonomous redistribution path | Explicitly forbids redistribution and database extraction without written consent. Exclude; do not send another permission request to create a new dependency. |

The previously reviewed [CMaNGOS exclusions](v1-public-source-audit.md#cmangos-classic-db-historical-comparison)
still apply. The MIT-labelled
[WoW Classic Items](https://github.com/nexus-devs/wow-classic-items) README explicitly
says it scrapes Wowhead and Blizzard APIs; it is not a provenance-clean replacement
for an unavailable Wowhead grant. Warcraft Wiki's copyright page returned403 during
this research; no new wiki content was retrieved or admitted.

## Independent operating model

Maintain Navzeroth's own versioned, documented catalog and an extensible set of
source adapters. Prefer sources with published compatible terms, checked original
scope and retained source. No provider is required to stay online, answer an email
or offer a private API for an already accepted revision to remain buildable.
ForeverDB remains an optional candidate under its published GPL terms.

1. **Choose narrow source scopes.** Start model-code assessment with WoWSims and
   acquisition/dungeon assessment with AtlasLootClassic. Review VMaNGOS as a
   historical comparator. Accept exact files, fields and transformations only;
   retain their true origin. Finish the ForeverDB literal-data source-form review
   from available evidence; do not equate an absent generator with a need for a
   personal permission email. Omit any unresolved portion.
2. **Preserve accepted inputs.** Retain immutable upstream snapshots, file hashes,
   licences/notices, allowed transformations and the source needed for compliant
   distribution in project-controlled storage with a local/offline import path.
   A commit URL identifies a revision but is not a durable local backup. Use
   content hashes even for release assets and never build from a moving `latest`
   tag. Do not mirror material until its selected scope is accepted.
3. **Build reproducibly offline.** Fetch is a separate, bounded opt-in update
   step. Normalization reads only the verified local material closure, uses our
   inert parsers/explicit transforms, preserves conflicts and unknowns, validates
   the full reference graph, and builds one immutable pack. Tests and packaging
   must not query a website. A second run with the same inputs must match hashes.
   Upstream outages freeze updates rather than invalidate a retained accepted pack.
4. **Own the Forever differences.** Record numeric gameplay facts through the
   supported addon environment during ordinary operator-driven play. Import
   sanitized exports manually; add build/evidence provenance and original
   descriptions. No memory/packet access, exhaustive hidden queries, input
   automation or copying proprietary quest prose/artwork. Author schematic maps,
   documented role assumptions, party workflows and reminder rules locally.
   New contributors must explicitly grant a compatible licence for their own
   material; licence selection cannot relicense someone else's text or images.
5. **Keep verification independent.** Candidate inventories are investigation
   queues. Imported counts, a second derivative of the same source, or repeated
   observations do not prove the complete70374 denominator. Compare exact required
   identities against independently evidenced target coverage. Missing domains,
   unknown inventories and unobserved behaviour remain visible release blockers.

Retain GPL attribution, modification notices, applicable source and same-licence
terms for covered derived material. Keep MIT notices and project attribution
where required; WoWSims additionally requests a visible project link. Separate
licence-bearing content and reusable application code for traceability. Resolve
GPL version compatibility before combining GPLv2 and GPLv3 derivatives; a root
GPLv2 text alone is not evidence of an “or later” grant. Packaging
files separately does not by itself settle copyleft scope for a combined work.
No blanket licence change to existing Navzeroth code or third-party data is made.

## What autonomy does and does not deliver

This removes provider replies, private APIs and single-provider uptime from the
required acquisition/build design. Source updates, parsing, validation, reporting,
pack assembly and repeat builds can be automated after specific inputs are accepted.
At this research baseline the accepted-source registry was empty. The
[implementation continuation](../data/SOURCE_PIPELINE.md) now retains three narrow
scopes and builds them offline. A complete target catalog remains unavailable.

Live game actions, visual comparisons and real Windows acceptance remain human
operator tasks under the release plan. A model cannot establish unobserved
server-only facts, complete drop eligibility or client compatibility by inference.
The target remains all nine domains on70374; no requirement is silently lowered.

## Next implementation order

First, assess the narrow WoWSims model-code and AtlasLootClassic relationship
scopes, then implement explicit inert transformations for accepted files. Retain
source/notices and test deterministic offline normalization, missing-material
refusal and cross-build isolation. In parallel with ordinary play, collect original
Forever observations for the inventory queue. The existing ForeverDB staging and
QuestieDB adapters remain optional comparison inputs. None of these steps waits
for the inquiry reply. Full-content publication still waits for actual coverage
and operator evidence, not correspondence.
