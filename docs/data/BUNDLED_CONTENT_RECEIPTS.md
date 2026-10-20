# Public bundled-content receipts (schema1, STABLE-CONTENT-RECEIPTS)

This is a separate `navzeroth-bundled-content-receipt` format. It does not change
private/local receipt formats1/2 or bound-delivery format3, their domains, trust
rules or launch-target behavior. The public bundle format supports the explicitly
approved release target without reclassifying old receipts.

The payload binds schema_version1, app_version, target (edition/version/build/
interface), bundle_manifest_sha256, production_sha256, tested_source_commit and
review (safe relative path, exact byte count and SHA-256). The caller supplies all
expected pins from independently verified state. Receipt contents must never be
used to choose their own trusted policy or expected release/review identity.

The caller-owned `navzeroth-bundled-content-signers` policy has schema_version1,
operator_controls_both_keys:true and exactly two ordered signers. Each declares
role, key_id, public_key_hex, identity and revoked. Roles are operator then
review_evidence; IDs and public keys must differ. Revoked, weak, malformed,
unknown-role, missing, duplicate or reordered identities/signatures are refused.
Both keys are operator-controlled: the review-evidence signature binds a separately
recorded review artifact. It does **not** prove independent key control, reviewer
authorship, correctness of review or completion of protected independent review.

Every Ed25519 signature covers the literal byte domain
`NAVZEROTH\0BUNDLED-CONTENT-RECEIPT\0V1\0`, followed by the canonical payload byte
length as a little-endian u64 and the canonical payload bytes. Receipt format,
schema and operator-control disclosure are mandatory constants checked separately.
All JSON inputs must be canonical UTF-8 as produced by bundled_receipts::encode:
exact typed field order, no additional whitespace/BOM/trailing newline. Duplicate
and unknown fields, invalid numeric/string values and inputs over64KiB fail closed.
Review artifacts may be up to16MiB; the receipt stores their identity, not their bytes.

The local-data API exposes payload/policy/signature readers, sign, assemble and
verify(bytes,policy,Expected). `Expected` includes every bound field. Pure signature
verification compares caller pins and cryptography; it does not read review files,
calculate the bundle/production fingerprint or prove Git ancestry. The loader/release
boundary must independently establish those facts before supplying Expected.

The explicit CLI commands are:

```
azeroth-cli bundle-receipt-sign --payload payload.json --policy trusted-policy.json --key-id operator --secret-key /external/operator.hex > operator.sig.json
azeroth-cli bundle-receipt-sign --payload payload.json --policy trusted-policy.json --key-id review-evidence --secret-key /external/review-evidence.hex > review.sig.json
azeroth-cli bundle-receipt-assemble --payload payload.json --operator operator.sig.json --review-evidence review.sig.json > receipt.json
azeroth-cli bundle-receipt-verify --receipt receipt.json --policy trusted-policy.json --expected expected-pins.json
```

`expected-pins.json` is a separately verified canonical Payload, not a copy selected
from the receipt. These commands emit canonical bytes without a newline; use byte-
preserving redirection (POSIX shells or cmd.exe). PowerShell5.1 `>` re-encodes output
and must not be used directly for these canonical files. Author files with the typed
encode API. Secret files use the existing format: exactly64 lowercase hex digits
for a32-byte Ed25519 seed, optionally followed by one LF. No key generation or
implicit lookup occurs; errors do not echo secret bytes. Keep private keys outside
Git with restricted permissions. The tests use conspicuous synthetic seeds only.

Assembly orders supplied signature records; it does not establish their validity.
Always verify against the independently supplied policy and expected pins. Actual
trusted public identities must be compiled or source-bound by the native loader.
Release receipt/payload/policy artifacts may live under release (excluded from the
production fingerprint); do not embed the signature into its own signed manifest
hash. This change adds no shipped loader, release signing, gate waiver or actual
review result. Unsigned installer status remains separate from signed content.
