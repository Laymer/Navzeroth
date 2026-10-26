# V20 prepared bundle identities before key use

Coordinator root, existing release-integration claim. Final independent public
review PR282 head912760b7 was normally merged at5d6d2652 after root read its
scope and independently verified all238 unique referenced paths/566421080 bytes
against explicit metadata baseline076a799c, including Git blobs or LFS identities.
The review remains preparation-only; nine current-installer observations are pending.

Before renewal, each old manifest/identity/receipt was verified equal to current
bytes, original9cc774e4 Git blobs and the committed signed-v19 archive under
release/history/2026-10-23-v19-attempts. Only those task-owned canonical outputs
were removed. The historical signed receipt and both failed engineering probes
remain preserved unchanged. No signing key was read during preparation.

The documented scripts/release/prepare-bundle.py command completed successfully
with explicit tested sourceb040778acfdf8ba621df193dc5f914ee8b9b6bda, production
d2cebaecebb068b1c27fb2a8d6d5f4955627d54301a20ee1c75c54049f5a9f6a and review
release/v1/public-content-review-v20.json,124512B SHA256
a718402099c6fcb13dbafa9538e5a0d1e811895c3267ff7bebd74c62a0322adf.
It invoked the cached normal dev CLI with CARGO_PROFILE_DEV_DEBUG=0 and target
/tmp/navzeroth-v20-preparation-target; actual CLI material assessment passed.
Output prepared:true/publication_allowed:false,84 resources/two packs.

Prepared manifest17187B SHA256
1513726d97a3af806e16f875fc1346a2a019fdcab57a8ab1f305d52ccde3f597;
build identity528B SHA256
fd85f5c60cab2aee2e3c4ec2686ab1ccef05e6fc43bc059a2b7860a80461dc7d.
The actual84 resources total395847866 bytes. Preparation canonical JSON has no
trailing newline; identity field order follows the Rust receipt payload contract.
This commit precedes all V20 private-key use. Receipt is intentionally absent.

Next: independent byte reproduction/closure and expected-pin review, then explicit
operator-owned external-key issuance and verification against compiled public trust.
Signing never supplies missing native acceptance or publication authority. Fresh
protected checks, source parity, coverage, resulting-main stability, all eight
installed cases and independently reviewed exact downloaded-draft UI/native smoke
remain required. No candidate push, main merge, tag or publication has occurred.
