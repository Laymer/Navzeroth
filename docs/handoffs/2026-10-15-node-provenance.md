# Additive node provenance and strict source claims

Owner: source_admission worker, claim 39c859b, baseline e6816f1 in `/tmp/navzeroth-node-provenance`. Root transferred v2/schema/record mapping and CLI release node completeness/source checks; inventory was notified.

Added optional empty-omitted `ReferenceContent.node_facts` with unique existing node IDs and nonempty, unique, resolved, uncontested evidence. Existing draft fixture bytes remain unchanged and explicit empty lists normalize to the prior canonical serialization. Facts count toward total capacity. Exact node evidence supplies both the existing node identity and new node_facts travel identity; absent facts leave sources empty rather than borrowing adjacent records' sources. No new desktop metadata list/API is needed.

Release binding requires one node fact per base node. Claimed source-less records now block even inside a unit with other attributed records. The isolated positive CLI mapping test now explicitly attributes its synthetic quest/objective; shared v1 fixture bytes and source gates remain unchanged.

Checks: all 344 core tests pass; focused v2/provenance tests 14 pass / 1 existing ignored. CLI library 10 pass; release_readiness 14 pass; release_materials 9 pass / 2 existing environment ignored. Strict core+CLI all-target Clippy and Rustfmt pass. Generated v2 schema agrees, Ajv accepts legacy/additive fixtures and rejects 9 negative cases. No Windows/live/content validation claimed. Next: independent review and merge into coordinator pack branch, then admission-registry release gate binding. All these source/content fixtures remain synthetic.
