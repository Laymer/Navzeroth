# Authored dungeon travel facts

Owner: source_admission worker, baseline 872a814 (`feat/v2-content-consumers`). Root explicitly transferred dungeon app report/consumer/tests ownership after review found `travel_edges` discarded. Worktree `/tmp/navzeroth-dungeon-travel`, branch `fix/authored-dungeon-travel`. One writer. Acceptance retains Atlas and bundle import work; pagination is frozen separately.

Implement provenance-bearing authored travel observation in preparation report, explicit unknown/restricted gaps, resolved known edge dependencies, and tests. Do not treat dungeon travel edge list as invented route geometry or infer missing facts from the base graph. Preserve existing local-package and exchange schemas.
