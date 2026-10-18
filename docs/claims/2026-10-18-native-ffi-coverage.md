# Archive FFI boundary coverage

Owner/sole writer Codex `/root`, baseline `3bc9d18`, within the single draft
coverage PR #203 (`test/continuation-coverage`). Read STATUS, current coverage
handoff and implementation plan sections16/18 before implementation.

Create a private explicit native-call table so the same Rust handle/read/partial
assembly boundary can be tested with bounded synthetic native responses. Production
always uses the linked offline CascLib shim; no selectable/runtime test backend,
public fixture API, decryption, archive format expansion or evidence promotion.
Preserve serialization, thread affinity, typed read causes, quotas, RAII ownership
and strict frame verification. Exercise normal/partial/encrypted/corrupt/missing,
null-handle and malformed-frame outcomes and cleanup. Real native decryption guard
and unavailable-storage checks remain separately exercised. Native responses in
unit tests are synthetic contract tests, not real-client/operator evidence.
Independent read-only review and actual checks precede integration; record any
product defect separately if one is discovered. Original checkout and `.claude/`
remain untouched. Do not push while the first corrected native collectors run.
