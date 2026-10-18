# Local-data third-party notices

Navzeroth's code licence does not relicense the following works or grant rights
to extracted game data. Extracted client data is local-only by default.

CascLib: copyright (c) 2014 Ladislav Zezula, MIT, source
https://github.com/ladislav-zezula/CascLib at
38a34665624b8775bb875274b36191b21c38d97b. Exact retained subset and hashes:
vendor/casclib/UPSTREAM.json. MIT text: vendor/casclib/LICENSE. Inline notices
are retained. Bob Jenkins' lookup3 is public domain; Alexander Peslyak's MD5
is public domain with its fallback permissive grant retained in hashes/md5.cpp.
Bundled zlib inflate routines: copyright (C) 1995-2017 Jean-loup Gailly and
Mark Adler, zlib licence; full notice in src/zlib/zlib.h. No deflate, gzip file
I/O or upstream tests/shared libraries are compiled. OpenSSL/Overwatch and unused
SHA1 sources are excluded. Build-time overlays in build.rs force read-only
local flat streams, reject remote/master providers and writes/directory creation,
remove the unused SHA1 helper, and fix upstream's Linux LPDWORD typo. Original
retained files are unmodified; derived copies exist only under Cargo OUT_DIR.

WoWDBDefs: wowdev and contributors, source https://github.com/wowdev/WoWDBDefs
at b9a63096cdd99b26967f17f9515377ae62cadfd7 (2026-10-16). Original selected definitions
(those explicitly listing 1.60.1.70374, 1.60.1.70425 or 1.60.1.70450), the
manifest, attribution/readme and full licences retained under vendor/wowdbdefs/;
UPSTREAM.json binds every file. Data definitions are CC BY-SA 4.0; code has a
separate BSD-3-Clause licence. The CC licence and attribution apply to the
embedded definitions; they do not grant client-data rights. Definitions themselves
are unchanged. Application-owned adapters select exact explicit builds/layouts.

wowdbdefs-rs0.1.0: Gtker, MIT OR Apache-2.0. Source at
https://github.com/gtker/wowdbdefs-rs, retained under root vendor/wowdbdefs-rs/.
Full licences and original changed parser/type files are retained there. This
is an altered version: build numbers widened to u32; final layouts/build-list
elements and mixed list/range syntax are retained; layout lists are parsed in
order. PROVENANCE.json records local changes and hashes. These changes are not
claimed as upstream code. No build numbers are truncated or remapped.

ed25519-dalek2.2.0: The dalek-cryptography developers, BSD-3-Clause,
https://github.com/dalek-cryptography/curve25519-dalek/tree/ed25519-2.2.0/ed25519-dalek.
Used for canonical receipt signature primitives, not automatic trust or signing.
Full notice is distributed under notices/ED25519-LICENSE.txt.

rfd0.17.2: PolyMeilex and contributors, MIT,
https://github.com/PolyMeilex/rfd. Used for the explicit installation folder
picker. Full notice: notices/RFD-LICENSE.txt. Full parser notices are also
distributed with the application. See dependency source for transitive notices.

## Complete desktop dependency notices

`notices/DEPENDENCIES.json` records the resolved Windows desktop normal/build
Rust dependency closure (including build tools and workspace feature unification,
not a runtime-only SBOM), npm production dependencies and retained notice hashes.
Full upstream texts are under `notices/rust/` and `notices/js/`; this includes the
PNG/compression dependencies used by private media decoding. The five MPL-2.0
packages have their exact, unmodified registry source archives under
`notices/sources/`, checksum-bound to Cargo.lock, to provide their corresponding
source with the distribution. They are not linked under the project's BSD terms.
Rust standard-library attribution and license texts are under `notices/rust-stdlib/`.

The WebView2 Rust wrappers' MIT terms do not cover Microsoft's loader binaries.
`notices/webview2-sdk/` retains Microsoft's separate LICENSE and NOTICE from
WebView2 SDK 1.0.3800.47. The manifest records the exact NuGet origin and SHA256,
and byte-for-byte matching hashes for all nine retained loader variants in
webview2-com-sys 0.39.1. The SDK binaries are not redistributed a second time as
source. WebView2 Runtime/bootstrapper installation remains subject to Microsoft's
separate terms. Installer signing and trust are independent of these notices.

Regenerate only after reviewing locked dependency changes:
`node scripts/dependency-notices.mjs --write`. Packaging runs `--check` after
locked dependency acquisition and refuses changed/missing/extra notice files,
altered registry notices or source archives, unknown dependency licenses and
unreviewed SDK/compiler versions. The retained override input file records pinned
upstream URLs and hashes for packages that omit root license files. Notice text
is copied unchanged; no game data, private stores, exports, caches, generated media
or signing keys are part of this dependency closure.
