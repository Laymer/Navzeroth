# Private media and research tooltip pipeline

Media format 1 binds a verified catalog, explicitly selected locale, deterministic
discovery object, source-closed analyzer/dependency profile and selected asset
results. All assets are local-only, uncalibrated and unverified for gameplay.
There is no distribution-rights or content-acceptance inference.

## Discovery and extraction

Exact retained development layouts are reviewed for Item icon references
`9a2a4834`, SpellMisc icon references `434b3607`, UiMapArtTile file references
`2da5b77b`, and UiMapArt highlights `3ae7d144`. Supported product/version tuples
are wow_classic_beta / 1.60.1.{70374,70425,70450}, with a declared selected locale.
Missing/unknown/unsupported fields, zero sentinels, decoder gaps and duplicate
record identities remain distinct discovery gaps. Each candidate retains table,
row and field provenance; tile identity refers back to the retained decoded row.
No guessed file paths or full archive scans are used for candidate discovery.

`local-data media-discover --destination <store> --catalog <digest> --locale
<locale> [--offset <n> --limit <1-200>]` returns a revision-bound page.
Discovery is bounded to 16,384 candidates; exceeding a quota refuses the operation,
never silently truncates it. Use an explicitly extracted table subset where needed.

`local-data media-extract --installation <directory> --destination <store>
--catalog <digest> --locale <locale> --ids <sorted-unique-file-IDs>
[--preset small|balanced|large|max]` runs in an isolated native worker, limited
before paths are sent over inherited pipes. Select at most 128 discovered IDs.
The installation must match the catalog metadata and full source hashes before
and after reads. File-ID-only reads never fall back to invented archive names.
Encrypted data is not decrypted or reconstructed. Native refusal reasons retain
read-stage provenance, including invalid, truncated and short reads. They remain
unavailable assets rather than invalidating an otherwise usable batch.

Limits: 32 MiB encoded file, 128 MiB aggregate encoded bytes, 4,194,304 pixels,
16 declared nonoverlapping mips with exact payload lengths. Existing private-store
quotas apply to raw objects, PNGs and manifests. Failures cannot activate content.
Unsealed immutable objects may remain for later recovery; existing active state
is untouched. The synthetic staging API always marks fixtures and cannot produce
a production manifest.

## Decoding and serving

The decoder supports BLP2 top-mip palette encoding (alpha 0/1/4/8), BGRA8, and
BC1/BC2/BC3 (DXT1/3/5). Declared lower mips are range/length checked, not rendered.
BLP1/JPEG, DDS, TGA and other formats are explicitly unsupported. Unknown encodings
and alpha combinations remain unsupported; malformed headers/ranges are invalid.
PNG output is deterministic and contains only decoded RGBA pixels.

The decoder is an original implementation. Header semantics were checked against
[an independent BLP2 implementation](https://github.com/corepunch/open-realm/blob/main/renderer/r_blp2.c);
BC color/alpha rules were checked against the
[Khronos data format specification](https://registry.khronos.org/DataFormat/specs/1.4/dataformat.1.4.pdf)
and [Microsoft BC documentation](https://learn.microsoft.com/en-us/windows/uwp/graphics-concepts/opaque-and-1-bit-alpha-textures).
No reference implementation source or proprietary image is copied into fixtures.

`media-verify --destination <store> --media <digest>` verifies catalog/discovery
closure, source hashes, decoded pixels, PNG bytes and current profile.
`media-page` accepts the destination, media digest, offset and limit (1–200).
Application services serve PNG bytes only for an opaque ID bound to a verified
media revision. Neither a file path nor a FileDataID authorizes serving. A previous
revision's ID cannot select an asset in a different revision. Desktop presentation
and delivery-policy bindings follow in later continuations.

Map/floor transformations, tile dimensions/bounds beyond decoded pixel resolution,
phase applicability, coordinate calibration and overlay eligibility remain
unverified. Schematics remain the supported gameplay fallback.

## Tooltip evaluator

`local-data tooltip --destination <store> --projection <digest> --spell <id>`
requires a reproduced current research projection and rechecks the reread hash.
It loads only the requested description's dependency closure. Descriptions remain
plain text: neither Lua, HTML nor WoW links/colors execute.

The core evaluator allowlists literal text, `$$`, explicit reviewed scalar values
for `$s1`–`$s32`, `$m1`–`$m32`, `$M1`–`$M32`, `$d`, and dependencies
`$@spelldesc<ID>`. The projection adapter supplies no guessed scalar values;
missing scalar inputs remain unknown. Arithmetic, conditionals, player modifiers,
other variables and expressions are unsupported and preserved as plain text.
Cycles, missing documents/scalars and capacity outcomes are explicit gaps.
Limits: 4,096 documents, 16 dependency levels, 4,096 evaluation tokens, 16 KiB
per text/output and 4 MiB aggregate input. Scalar values cannot inject tokens.
Every result has `verified_for_gameplay: false`; successful interpolation does
not establish a current spell amount or actionable advice.

Actual native Windows media extraction, client image correctness and interactive
desktop display remain unverified. Synthetic pixel/closure checks are separate
from operator evidence and cannot clear acceptance blockers.
