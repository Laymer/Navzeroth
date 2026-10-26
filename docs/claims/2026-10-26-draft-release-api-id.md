# Draft release REST lookup repair

Owner: Codex /root. Baseline: aaa73b94794f5a32162b430b2314f53bca4e1a7b.
One writer in isolated /tmp/navzeroth-draft-release-api-id.

Actual draft424161437 is available through gh release view and REST by ID; the
REST by-tag endpoint returns404, consistent with its published-release contract.
Tagged verifier currently uses that endpoint after downloaded UI verification.
Repair the lookup using the issued numeric databaseId, with strict identity/state
validation and focused negative tests. Preserve the running attempt's real results.
Do not change gates, publish, move the existing v1.0.0 tag or rewrite history.
Independent review, renewed production/review/receipt bindings and fresh checks
are required before repaired release artifacts can be accepted. Resolve existing
unpublished-tag identity explicitly before any replacement.
