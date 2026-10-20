# V2 strict content allocation repair

Owner desktop_review. Baseline d271bba67e8fa274e44953c9f78198b1d8fc5f6d.
Sole writer /tmp/navzeroth-stable-content-allocation branchfix/stable-content-allocation.
Read STATUS/latest workload/native handoffs and plan16.3 correctness requirements.
Scope core/content_v2/json.rs borrowed Value deserialization, storage/content_catalog.rs
consuming canonicalization for owned stored input, focused regression tests/claim/handoff.
Retain strict JSON duplicate/unknown/optional checks, all capacities/validation,
source/conflict canonicalization, historical hashes and borrowed stage API semantics.
No recovery.rs edits (world owns separate recovery validation task), no streaminghash,
no content/source/receipt/key/limit changes. No speedup or Windows completion claim.
