# Foundation role audit CI input availability

Owner/sole investigation writer `/root/world_review`; baseline04ddbc2b;
claime5b03243 on fix/stable-foundation-source-parity. Continuation of the necessary
world identity repair, before push. No audit, source, authored or generator bytes
changed in this continuation.

CI39386917359 Rust Linux and Windows failed the authored role audit because the
observed hash285c609bffa5af9616ad4568c934f8ea51131260d1e5d9151c1f0ab1de469fea
differed from reportf5a0f00625964c0a5c215b90c282473c9441d5f117e9538bdc7bdee0647031f5.
The initial stale-header hypothesis was disproved:285c is exactly the SHA256 of
the133byte checked-in Git LFS pointer. Actual current hydrated quest bytes, the
run manifest and the genuine audit all agree onf5a0. The run manifest itself is
also LFS-managed. Existing CI deliberately leaves LFS objects unhydrated.

Ran the documented update-authored-role-coverage.py in an isolated temporary
copy with exact current registered quest/run-manifest inputs and authoredr2.
The generated role audit is BYTEIDENTICAL. Entire authored catalog JSON is
identical, preserving every class mask/count/range,54models and18reminders,
weights, restrictions, unknowns and gameplay records. Actual build-content-v2
canonicalization of that temporary authored output is byte-identical to checked-in
r2 SHA7336b1f92b49d886a596bb40e056f09dd5264651bc35532c2639dfd270bfe87b.
No metadata rewrite or second world identity regeneration is justified.

All4CLI authored_slice tests and11source_admission tests passed against hydrated
actual inputs. Actual bound mapper parity ran twice and matched all committed
products in23.03seconds at e5b03243. The existing39d1516d world identity repair
remains correct; both original CI and hosted source failures retain their outcomes.
[Exact hashes and reproduction](../data/stable-foundation-role-audit-binding-v1.json).
Logs: /tmp/navzeroth-foundation-role-audit-{rust,parity}.log.

Root chose a bounded selective CI hydration repair, preserving all existing raw
record/count/origin assertions. Tooling is sole writer in a separate isolated
branch for helper/tests/workflow steps; this source branch does not modify those
files. Hydrating only the exact quest shard and run manifest avoids unrelated
imagery/media downloads. No passing pointer-only substitute or weakened expected
hash is introduced. Final combined repair must be independently reviewed, then
all ten exact-head checks and actual Linux/Windows parity rerun before main.
No push, signing, network acquisition or acceptance promotion occurred here.
