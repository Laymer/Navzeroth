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
