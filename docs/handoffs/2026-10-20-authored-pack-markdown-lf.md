# Preserve canonical shipped authored Markdown on Windows

Owner/sole writer `/root/tooling_review`; baseline
500f58116696612827dfbcf70364294bfd5a8ad9; claimf95810b2; immutable repair
1c7d6520ea75212afb4930998f212994e5b1524c.

Actual preliminary stable coverage run39400864456 at40eaafaa2fafeb7da68daa8c191eed226b7f9cfb
failed Windows job118554947383 at09:46:51Z in coverage-bundle-inputs. The retained
trace raises `working non-LFS resource differs from HEAD`; assembly, native build
and native smoke were not reached. The coverage gate then failed the required
collection-status guard, not a measured per-file threshold. Linux independently
passed1605tests/0failed/8ignored across207binaries; TypeScript passed with91source
files all>=90%lines/global95.9%. These do not establish Windows bundle acceptance.

Read-only diagnosis found14 authored Markdown FileRefs without explicit LF:
two evidence files and12 procedures under content/release-packs/authored-v2.
The existing content/authored Markdown rule did not cover their shipped copies.
The first affected FileRef in manifest order was evidence/authoring-statement.md.
An actual Git fixture using40ea's exact blob and attributes reproduced2476-byte
HEAD SHA2cfb951a671fa7c52539558044a532fc6be1aedca519b695b00697cb02708258
becoming2517bytes SHA42687ce4291a39e930f3126b6a5a3f2723f51c3c6358f7602461f76547c79441
under core.autocrlf=true (41CRLFs). V3 public review JSON already had correct LF;
no source grant, LFS pin, Python path or signature defect was identified.

Repair adds only `content/release-packs/authored-v2/**/*.md text eol=lf` and one
real-Git regression. It reproduces CRLF checkout/refusal before any fetch, applies
the actual rule, verifies exact canonical LF plus successful offline hydration,
and proves subsequent altered bytes still fail strict hashing. No runtime read
normalization or validation relaxation was added. Actual Git attribute audit
confirms all14 signed authored Markdown refs now resolve text=set/eol=lf.

Actual13 bundle Git/LFS tests PASS (preceding12 retained), `git diff --check`
PASS. Runtime scripts, Rust, TypeScript, shipped content bytes, raw source/code
pins and receipts are unchanged. The attributes change is nevertheless production
and requires release-owner frozen-production/public-review/receipt renewal after
independent review. No new full functional corpus run was claimed or needed to
validate this checkout-only repair. Actual fresh Windows/full coverage remains
required; the old failed run stays failed.

Evidence paths:
`/tmp/navzeroth-stable-coverage-39400864456-failed.log`;
`/tmp/navzeroth-stable-coverage-39400864456/windows/logs/coverage-bundle-inputs.log`;
Linux/TypeScript raw artifacts under the same parent directory. No retry, network
world acquisition, signing, key access, push or merge occurred in this task.
Next: independent review and normal repair integration, renewed public bindings,
then fresh required stable coverage and hosted acceptance on the final source.
