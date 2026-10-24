# Stable 1.0.0 content signer rotation

Owner: root. Baseline 5d62bfe7ec175ae0e4ba6ac3a36c6e841731f26f. Sole writer in /tmp/navzeroth-stable-signer-rotation, branch fix/stable-signer-rotation. Scope: content/bundled/signers-v1.json (the compiled trusted policy) and docs/releases/1.0.0-content-public-identities.json.

The operator could not locate the original operator and review-evidence seeds that signed V3-V17, so the owner authorized replacing both identities for 1.0.0. New Ed25519 seeds were generated locally, outside Git, with owner-only permissions; only public keys and new key IDs (-r2) are committed. Both keys remain operator-controlled and the receipt requirement is unchanged. Earlier archived signatures were made with the old keys and are historical evidence only. This is a production change: it requires a new tested ancestor, public review, unsigned preparation and signing.
