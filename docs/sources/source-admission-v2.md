# Source admission schema2 — retained original inputs

Schema2 preserves schema1's explicit sources, grants, revisions, exact fields,
transformations, bounded retrieval and offline hash checks. It changes two bounds
needed for retained world-data sources:

- One admitted original may be at most64MiB. The selected CMaNGOS compressed SQL
  is about13MiB; decompression remains a separate bounded inert parser operation.
  Schema1 retains its8MiB per-file limit. Total retrieval remains at most2GiB.
- A permitted-file URL may name exactly a Wago DB2 CSV table with canonical
  `?product=wowf&build=A.B.C.D` or `?product=wow_classic_era&build=A.B.C.D`.
  Host/path/table/product/build are constrained. Tokens, additional query fields,
  fragments and unknown products are rejected. Schema1 still rejects queries.

This is retention/provenance support, not a new source admission or network grant.
Existing rights, hashes, true origin/build and exact reviewed field permissions
remain mandatory. Automatic source-sync fetching remains restricted to reviewed
immutable GitHub raw paths; Wago originals must already be retained and verified.

The cache verifier admits the larger maximum only after the source registry has
validated declared file sizes; historical schema1 cannot declare a larger file.
Unsigned installers do not waive content signatures or material/source closure.
