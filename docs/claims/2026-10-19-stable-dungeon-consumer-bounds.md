# Stable dungeon consumer bounds

Owner: /root/desktop_review. Single writer.
Baseline: 14c091826aef1a6e00da1e2bf5698fdefb015f6e.
Branch: fix/stable-dungeon-consumer-bounds; isolated /tmp/navzeroth-stable-dungeon-consumer.

Authorized repair: the actual r4 full-pack scenario failed after 166.44 seconds
with “Preparation section exceeds 32 records”. All 26 supported generated dungeon
packages have more than 32 reward references; maximum 1024 rewards and 91 quests.
Preserve every source reference and unknown condition. Version larger package
bounds, retain bounded request/report output and historical schema1 behavior,
and page desktop lists without dropping report records. No storage/cache edits,
source changes, signing, publication or target-client claims.

Validation and exact affected IDs will be recorded in the handoff.
