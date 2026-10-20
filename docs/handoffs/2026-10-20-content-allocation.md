# Strict owned V2 content allocation

Owner desktop_review; baseline d271bba67e8fa274e44953c9f78198b1d8fc5f6d;
claim a0b9c492, frozen code b3dfe191. Isolated /tmp/navzeroth-stable-content-allocation.
No recovery.rs, hash algorithm, runtime trust policy, schemas, limits, source packs,
canonical release bindings or key changes. World owns separate recovery validation.

Strict V2 decoding previously held the strict input Value, cloned it for consuming
from_value, then retained typed content and a comparison Value. Deserialize from
&Value removes the temporary full Value clone while preserving strict_json input,
known_fields traversal and every explicit-empty/dependency exception. It still
creates the original and comparison trees; no zero-copy/streaming decoder claim.

Storage load owned a decoded ContentRevisionV2 yet canonical(&input) cloned the
whole typed graph. A private consuming entry now validates the same compact
capacity/domain invariants and moves that owned graph into unchanged pretty-bound,
source registry and conflict canonicalization. Borrowed staging validates BEFORE
cloning, retains caller ownership and exact output. Stored raw document is dropped
after strict borrowed-wrapper decoding, before canonicalization/hash. All SQL
version/length, hash/coverage, source/conflict row and count comparisons remain.
The canonical hash still allocates its Value and byte buffer; streaming is deferred.

Actual checks at b3dfe191:
- Core content_v2:21passed/1ignored, including nested unknown/duplicate refusal,
 explicit empty optional cases and new parity against historical consuming serde.
- Storage catalog10passed, assets7passed; owned/capacity units2passed. Pointer test
 proves the owned node-vector allocation moves intact; borrowed caller bytes stay
 unchanged and stored/catalog digests match.
- Full affected LLVM core+storage663passed/0failed/2ignored across76test/doc binaries.
 Final PRODUCTION counts use existing lcovLines+rustTestRanges, not raw JSONsummary:
 core/content_v2/json.rs43/43=100%; storage/content_catalog.rs350/372=94.0860%.
 Only existing inline cfg(test)490–557 is excluded. Exact current source paths
 selected; unrelated stale old-worktree rows are not current production evidence.
- all-target Clippy-Dwarnings, fmtcheck and gitdiffcheck passed.

Independent prior-output oracle: /tmp/navzeroth-old-content-oracle.rs was linked
against retained PRE-CHANGE optimized storage rlib4a3c24e0b5fc7882 (SHA256
acef23ae36b063ddd9c75ce0de866499f04d6fd35466977b63f6ce1a35a1bec7), matched
serde_json47e2d607. The old root decoder/catalog/hash source bytes were checked
identical to d271 before use; no new-source compiler provenance is claimed for
that old library. Actual staging generated
/tmp/navzeroth-pre-allocation-content-oracle.sqlite3. PythonSQLite independently
read document13677B SHA223f8539f445e4edb759e87dcd4a17f0e3cf6a26f1b59f0e0fbb7ff29451acf0,
canonical digest10d068a2aea2fdad6c31a49e62cf1edd2928fd717e1d572f52b5f760b92db95e,
coverage SHA8181ce0eddfc80940a44598c13ed713dee1ec7ccb0dc1835065607d52a43647f.
The new fixed golden stages unsorted source rows plus a retained conflict, asserts
these exact prior outputs, then reopens/loads and checks provenance and bytes.
It does not derive its expected digest from the new shared canonical helper.
Root independently checked this old SQLite and ran the matched new core21/catalog10/
owned2 binaries; separate root production LCOV derivation also passed43/43+350/372.

Reports /tmp/navzeroth-content-allocation-{core,owned,storage,golden,coverage,clippy}.log,
coverage.json, coverage.lcov, production-coverage.json. Initial attempted nonexistent
catalog_generation test target did not run tests; corrected targets passed. A test
fixture attempted mutation of Validated<SourceRecord> and failed compilation; it
now constructs a separately validated JSON fixture. Initial old-oracle link used
a mismatched serde artifact and failed compilation; matched old dependency ran.

LLVM waited for the independent full-pack baseline benchmark to finish. No duplicate
full-pack or optimized candidate execution was run here. Allocation removal alone
is not a measured speedup, lower-RSS claim, Windows fix or recovery completion.
Actual V13timeout/partial successes retain their old source identity. Next: tooling
independent review, draft repair PR and parent integration; then rebuild and actual
whole-pack recovery/functional measurements, fresh protected/source/Windows gates.
