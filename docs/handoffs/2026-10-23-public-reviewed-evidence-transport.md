# Public verification of independently reviewed draft evidence

Owner world_review; baseline4900876f; claimc58e2415. CODEf7f59176341fa530705de5441ffa3084f735dfe8.
Single writer new Python transport/tests and verify-release-downloads workflow.
No Rust acceptance, signed resources, metadata, key or publication mutation.

## Gap and transport

The old evidence_run_id can provide packaging's eight rows and pending final row,
but cannot supply the later completed-job API snapshots and independent draft UI
review. Reusing that run alone cannot pass full public readiness. A second artifact
producer would add an unnecessary workflow. The new reviewed_evidence_commit is
an explicit40-character lowercase commit in the same configured GitHub repository,
allowed only for public stable1.0.0 and mutually exclusive with evidence_run_id.

The source-tag checkout runs its own trusted Python code. A bounded Git fetch
loads the requested object; only regular non-executable blobs in the fixed
release/evidence/1.0.0/{acceptance-evidence,draft-evidence}/ subtree are copied to
new dist/{acceptance-evidence,draft-evidence}/ directories. No checkout, filters,
archive code execution, imported Python/Lua, or tagged-source overwrite occurs.
The source HEAD/tree and tracked cleanliness are checked before and after import.
All Git outputs are bounded while read, with command deadlines and an overall
600-second transport deadline. Bounds:1024 files/64MiB each/256MiB aggregate,
2MiB tree metadata/64KiB stderr. Actual Rust contracts impose narrower typed roles,
including8MiB details; this transport never grants acceptance from bytes or flags.

Portable ASCII paths refuse absolute/dot/backslash/Windows-device/trailing-dot
names and case collisions; links/submodules/executable modes/LFS pointers are
refused. Existing destinations are never merged. Inert temporary staging is removed
on failure; partial moved evidence directories are removed. This is a trusted CI
workspace operation, not a hostile concurrent-filesystem sandbox. Git object and
FETCH_HEAD updates do not change application production identity.

Public-phase download receipts now write separately to dist/public-download-evidence
and artifact upload chooses that path. They cannot overwrite the consumed draft
proof. Draft receipt path/semantics remain dist/draft-evidence unchanged. The
workflow still calls the existing full readiness validator; wrong installer/source,
failed/unreviewed/incomplete evidence must fail there. No review is auto-created.

## Actual checks

- Python unittest14PASS on Linux using synthetic temporary repositories: inert
  positive data import, unchanged exact source/tree, failed raw flag preserved,
  existing destinations, altered checkout/dirty source, limits, paths/case, LFS,
  link/submodule/executable modes, missing overlay, output symlink, origin mismatch,
  explicit public-only/mutually-exclusive inputs and overall deadline.
- The positive CLI test stubs ONLY network fetch, using an already-present commit;
  it checks same-origin fetch arguments. No network fetch or real acceptance proof
  was imported/executed. Native Windows execution remains unverified.
- Initial positive tests found ls-tree -t includes exact ancestor trees; the
  parser now allows only those known tree ancestors and all14 tests reranPASS.
- actionlint on the changed workflow and git diff --check PASS.

Next: independent source/workflow review before final source freeze and receipt
preparation. After the real exact draft job finishes, independently review its
native/UI/download details, retrieve completed run/job/release API snapshots,
construct the strict final overlay, archive only its inert evidence subtree in
an immutable same-repository commit, and record that exact reviewed commit. Public
verification imports it without changing the tag and still requires full readiness.
No hypothetical future proof is classified as passed here.
