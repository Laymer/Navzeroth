# ForeverDB inert quest staging

Owner and sole writer: Codex `/root`.
Baseline: `93c361db12ad80f8c82e7a39a57960268e77022c` (PR143).
Branch/worktree: `feat/foreverdb-inert-staging` / isolated staging worktree.

Implement bounded local diagnostic staging of Meta.lua, QuestForever.lua and
QuestClassic.lua using the existing literal parser. Preserve source identity,
origin strata and literal distinctions. Emit audit metadata/IDs only, never
activate or redistribute gameplay data. Keep the unreviewed admission and release
gates intact. Verify actual pinned files, adversarial literals, input hashes,
path safety, unknown versions and mixed wrappers; document remaining work.
