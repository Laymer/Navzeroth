# V20 signed closure and candidate continuation

Owner root, existing release-integration claim. Prepared identity commit
b25064b6ed48a5ba7e760b397adc8446aee174bc precedes all V20 private-key use.
Independent tooling review recreated both prepared files byte-for-byte and checked
all 84 resource identities against source before constructing its own expected
receipt payload. Frozen public review remains preparation-only and unchanged.

The first explicit signing attempt selected a different external key location and
correctly refused its mismatch against committed r2 public policy; it produced no
signature or receipt. The matching existing keys were located outside Git with
owner-only directory/file permissions (0700/0600). The user subsequently confirmed
that location. No key rotation or public-policy/source change was required. Private
seeds were not printed or committed. A path-only owner-local locator was recorded
outside Git. Both signing identities remain operator-controlled; independent review
is separately recorded, not inferred from two signatures.

Successful explicit signing, assembly and public verification bind receipt
1080 bytes, SHA256
7944bb65f0a197f5e86de2a979916f4e7e7ca91f7ca1524f73388147f1ccf35c
to prepared manifest 17187 bytes, SHA256
1513726d97a3af806e16f875fc1346a2a019fdcab57a8ab1f305d52ccde3f597,
tested source b040778acfdf8ba621df193dc5f914ee8b9b6bda, production
 d2cebaecebb068b1c27fb2a8d6d5f4955627d54301a20ee1c75c54049f5a9f6a,
and review public-content-review-v20.json, 124512 bytes, SHA256
 a718402099c6fcb13dbafa9538e5a0d1e811895c3267ff7bebd74c62a0322adf.
Root verified against tooling's independently constructed pre-receipt expected
payload; tooling separately verified the issued receipt without private-key access.

Only bundled_content was added to canonical schema5 acceptance evidence. All nine
rows remain pristine unverified; the exact prebundle file remains unchanged under
verification-v20. New canonical bytes: 27970550, SHA256
5e5178040fb4fbb8458d437da6a01ca48cc2faf607ea6622e89b27c454af1d9c.
Historical signed V19 material, failures and actual V20 engineering probe remain
immutable. Probe 39925184911 succeeded at b040 with actual full recovery 915.984s,
strict whole-tree completion and all 14 phases, using the explicitly historical
V19 fixture. It is not current-installer acceptance or measured Windows memory.

Next: independently assemble/check signed resource closure with compiled public
trust, execute prebuild source/readiness gates, review the frozen candidate and
push PR239. Fresh protected checks, source parity, coverage, exact resulting-main
stability, all eight hosted installed cases and reviewed exact downloaded-draft
UI/native smoke remain required. No main merge, tag or publication yet.
