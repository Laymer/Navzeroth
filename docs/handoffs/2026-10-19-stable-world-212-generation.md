# Isolated PR212 canonical world regeneration

Owner/sole writer: world_review. Baseline38710cd; claim98003a7.
Canonical LF code86756f74b30b66bac9c0104ccdfe872d15e68a0b, generatedc339e990dd901f8ef3a641989b1bb88223d468ac.

This regeneration deliberately uses original authored r1, SHA256
8bac8b20ee75bb1c6d73b1b6eb57285ef9101a22d360e5ae671d1a2511d48d86,
unchanged from baseline. Later quest/role/party/authored inventory work remains
in its separate feature branch; none was copied here. Mapper remains v2; dungeon
v3 planar entrance estimates are not included. Accepted-origin policy and source
normalization policy are byte-identical to baseline; all upstream and unrelated
admission records are identical. Only project mapper code files/proofs and their
immutable URL/revision bindings changed.

The retainer read the existing cache at
/home/laymer/Devel/worktrees/datagen/data-generators/.cache without modifying it.
It retained the same19 upstream originals and22 corresponding-source files,
with two new small content-addressed project code objects; immutable older source
objects remain. True origins remain CMaNGOS vanilla1.12.1 and historical Forever
1.60.1.70245. Admission does not claim target-build verification. New world revision:
navzeroth-world-1.60.1.70450-df08efbdf71b8a6cb3cebbf7.

Actual checks:

- Own exact-candidate CLI build passed;157 generator tests passed.
- Git LFS object/pointer fsck passed on committedc339e99.
- Offline source-sync passed using existing exact local retained objects:10.10s,
  33024KiB peak child RSS on shared Linux host. No network rights or budget change.
- Actual bound mapper regenerated twice against committedc339e99; all four outputs
  exactly match committed bytes, with no newline normalization. Selected90 files,
  253629231 bytes,35 raw shards,19 originals. Combined two-run time27.55s.
  [Receipt](../data/stable-world-212-parity-linux-v1.json) records every input and
  product pin. Catalog152853638 bytes, SHA256
  7aa5cc1ee39b7671f1d7a8daf19f95c1705fb35939eef3f41cb6a249d4e3c7d3.

Pack build and independent verify-pack both passed:3files367731829 bytes,
393597 declared coverage gaps,distribution_eligible=true. Debug build115.36s /
6696800KiB peak child RSS; verify52.72s /5363156KiB.
[Pack hashes, reports and measurements](../data/stable-world-212-generation-v1.json)
retain exact resources and commands. None of these
checks establish hosted Windows parity, installed application performance, live
client evidence, stable release readiness, signed receipts or installer inclusion.
Exact-head protected checks and independent review remain required. Publish the
original86756f7 code history before relying on public corresponding-source URLs;
cherry-picking alone onto different parents need not publish that original commit.

Next: root review/integrate canonical world plus parity workflow; dispatch actual
source parity on the refreshed candidate, then rehearse inventory/material closure
with these exact PR212 world outputs. Keep later feature inventory separate.
