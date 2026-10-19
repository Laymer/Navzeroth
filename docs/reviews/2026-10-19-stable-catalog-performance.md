# Independent storage performance repair review

Reviewer Codex `/root`, read-only review of world specialist implementation
9bb2096c113e88cd525653c31df41c47b202701b and evidence95f04e6.
Root independently traced content_v2::decode to strict typed decoding followed by
ReferenceContent::Validate. Already typed catalogs cannot retain discarded JSON
fields; all format/schema, envelope, graph, domain and capacity validation remains.
The two replaced digest/serialization calls were capacity-only consumers. Counting
compact/pretty byte lengths preserves those limits, including the canonical
trailing newline; real stored digests, document bytes and coverage comparison stay.
RawValue at persisted input retains original nested fields for strict decoding.
Historical v1 decoding is untouched. No blocking finding.

Actual independent command: cargo test --locked -p azeroth-storage --test catalog_v2,
8passed0failed. Includes exact false/zero/unknown/stored-byte persistence, malformed
typed refusals and original unknown/duplicate JSON rejection. Log
/tmp/navzeroth-root-perf-review.log. Full-data byte compatibility, portablecoverage
and Linux performance measurements are writer-recorded evidence, not additional
independent runs. The writer's exact three stored documents match baseline; all
measurements remain shared Linux observations. Windows resources/performance and
ordinary-PC claims remain unverified.

Integrated ascc8ad87+c5471d0, followed by normal main merge39c5170. Current protected
checks on this repaired branch remain required before PR212 merges. No installer,
manual smoke or stable publication result follows from this review.
