# STABLE-NATIVE-BUNDLES handoff

Owner/sole writer Codex `/root`; isolated feat/stable-native-bundles at defaults
24ccc4c plus receipt617deec cherry-picked1152754. Claim247f95b. Historical base
is PR212 before its integrated repairs; normal integration must retain those.

Implemented the versioned installed-resource manifest, exact caller-owned build
pins, compiled public signer identities (sameoperatorcontrolexplicit), bounded
streamed no-follow resource integrity, typed source-conflict staging and separate
raw normalization conflict resources. Native startup embeds the validated build
identity from NAVZEROTH_BUNDLE_IDENTITY_JSON and refuses stable missing identity.
Owned verified catalogs/assets stage before one-scope transactional activation;
background initializer runs before ready and after compatible backup restoration.
CLI check-bundled-content reports signed-resource-closure only, not admission or
activation. Full documentation: docs/data/BUNDLED_INSTALLER_CONTENT.md.

Independent tooling review of exact production file hashes (before docs-only
commit): appbundle bdea489362afcbf3809cffbe112b31368d32ca3e8a25c4e9da562ab1bfb1939f,
resource I/O a89146b1dd8f3726de5c3998a963495d6527a97b29f7dba8c13df64afe1aac85,
CLI7575c86a0abe2b775399f60f6dccb1f440da25de94b5b64d6762970f241a5c54,
recovery fd8de75fa0b2e3eb7d2008afc8a6f27a0c550dbf47006e2141662c0caa6af567.
Six independent initial bundle/CLI tests and13recovery tests passed; no binding or
activation blocker found. Initial rejected two-filter cargo invocation was corrected
and is not a pass. Later two new tests cover raw normalization report/revision/
precedence binding and remaining-policy-gap/depth refusal; no product edits after
that independent review. Earlier architecture findings (64KiB reader reuse, unsafe
reopen, re-read staging, dropped conflicts) were repaired before this review.

Actual final portable continuous LLVM run:582passed,0failed,4existingignored.
Appbundle224/233=96.14%, resource I/O74/78=94.87%, CLI41/43=95.35%; recovery94.23%,
CLI main92.63%, app lib100%, local-data lib92.03%. Thresholds unchanged. Initial
coverage appbundle89.27% was not a gate pass; meaningful normalization/policy/depth
tests brought it above90%. App/local-data/CLI all-target Clippy, format, diffcheck
passed. The initial profile-import fixture retarget lacked proper producer/nested
source/context metadata; checks correctly refused it. Corrected synthetic fixture
now exercises activation before imports without weakening production validation.

Native cargo check was attempted and failed in libdbus-sys before checking shell
code: dbus-1.pc is unavailable. Windows native build/installer/resource mapping,
coverage of native shell/buildscript, hosted performance, receipt issuance, manual
owner smoke and final readiness remain unverified. No real secrets read/signature
issued and no publication occurred.

Important inherited reader finding: pack::v2 reopens through shared ingest Budget
plain File::open. A controlled replacement could block Unix despite new wrapper
checks. Tooling reviewer owns a separate descriptor-safe pack-reader repair;
cherry-pick/review it before merge. Do not call the end-to-end loader complete until
that repair and final native verification pass.

Next: integrate repaired212/dungeoncomposition/objectives/roles/materials and
workflow adapters normally; assemble exact source-backed signed bundle only after
independent final review, add complete hosted Windows case producers, then revised
release gates/exact draft download verification and required owner smoke.
