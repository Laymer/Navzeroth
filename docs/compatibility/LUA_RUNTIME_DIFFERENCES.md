# Lua runtime differences: test harness versus the game client

Task M0.3. The crate `azeroth-lua-harness` (`crates/lua-harness`) runs this
repository's own addon sources in a vendored **PUC-Rio Lua 5.1.5** (mlua 0.12.2,
features `lua51` and `vendored`). The game client embeds its own Lua 5.1-family
runtime whose exact build, library set and limits for Forever are **unverified**.
Standalone Lua tests verify pure module logic only, never Blizzard API behavior
(plan section 18.3). Passing these tests is not evidence of client behavior.

The harness is dev-only. It is not a dependency of `azeroth-core`, `azeroth-app`,
`azeroth-cli` or `azeroth-desktop`, and it never loads SavedVariables, imported data
or player-supplied text (plan section 7.2): the desktop must parse SavedVariables as
data without a Lua interpreter.

| Area | Harness | Client | Consequence |
| --- | --- | --- | --- |
| Interpreter | Stock Lua 5.1.5 C source | Blizzard's modified Lua 5.1 (unverified for Forever) | Language semantics are shared; client patches are not modeled. |
| Standard library | `string`, `table`, `math` and a whitelist of base functions (`assert`, `error`, `getmetatable`, `ipairs`, `next`, `pairs`, `pcall`, `rawequal`, `rawget`, `rawset`, `select`, `setmetatable`, `tonumber`, `tostring`, `type`, `unpack`) | A larger set including client helpers (`strsplit`, `wipe`, `tinsert`, `date`, `time`, ...) and possibly partial `os`; unverified | The addon uses only the harness subset. Helpers absent from the harness are not used. |
| Forbidden facilities | No `load*`, `dofile`, `require`, `package`, `io`, `os`, `debug`, `getfenv`, `setfenv`, `collectgarbage`, `coroutine`; `string.dump` removed | Several exist or are restricted differently | A source scan fails if the addon names any of them, so differences cannot matter to the addon. |
| Globals | One environment table per sandbox; strict mode errors on any undeclared read or any write | The shared `_G` with every client global | Strict mode proves `Core/` touches no global; it cannot prove what the client exposes. |
| Client API | Synthetic stubs defined per test | Real, version-dependent functions, secure and protected functions, taint, combat lockdown, restricted ("secret") values | No stub models restrictions. CASE-010 is exercised synthetically only. |
| Frames and events | A stub `CreateFrame` records `RegisterEvent` and `SetScript`; tests call the handler directly | Real event dispatch, ordering, loading screens, reloads | Event names and the timing of SavedVariables loading are assumptions. |
| Reload | A new Lua state that receives the previous table | UI reload inside the client | Models "no error on repeat and identical output" only. |
| Numbers | IEEE doubles; export is integer-only; `%.0f` formatting | Doubles; `%d` can truncate on 32-bit `long` platforms | The encoder avoids `%d` for large values and never emits floats. |
| String ordering | `strcoll` in the C locale, so byte order | Client locale behavior unverified | Export keys are ASCII, so order is stable; `table.sort` on other strings is not relied upon. |
| Iteration order | `pairs` order is unspecified | Same | Output sorts keys and sorts the capability catalogue. |
| Error text | Chunk names such as `@AzerothNavigator/Core/Json.lua` | `Interface\AddOns\...` paths and client traces | Redaction is tested with synthetic path and name patterns only. |
| Resources | 64 MiB memory limit, instruction budget of 100 million | Script-time and memory limits unknown | Tests prevent runaway code; they do not measure client cost. |
| UTF-8 | Lua 5.1 has no `utf8` library; the encoder validates by hand | Same | Invalid UTF-8 is rejected, never emitted. |
| Build | Vendored C compiled by `cc` on Linux (gcc) and Windows (MSVC) | n/a | The Windows MSVC build is verified only by the hosted Windows Rust job. |

Maintenance rule: when a client check (M0.4, M0.5) observes a difference, add a row
here and, if a test depended on the old assumption, change the test with evidence.
