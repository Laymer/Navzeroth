# Catalog pagination implementation

Owner: source_admission worker. Claimed task in `docs/claims/2026-10-15-catalog-ui-pagination.md`; initial baseline 57f6097. Dependencies merged normally: authored consumers 557d86b and capacity/index cached first-page API 908aef2. One writer for this task. Atlas is explicitly owned by the acceptance worker; asset storage by inventory worker.

Desktop catalog DTOs now carry first-page metadata. Gear item/acquisition lists, solo and joint planner nodes/objectives, and party quest lists return at most 200 rows per collection. `get_catalog_page` binds subsequent requests to the exact immutable revision and catalog SHA-256, with a fixed 200-row limit, strict bounded input, indexed v2 access and compatible bounded legacy paging. The authored model/dungeon picker uses the same cached index rather than materializing the entire choice list. Selection exchanges retain their existing formats.

UI controls replace the visible page without accumulating the catalog. Selected inputs remain bounded by existing 128 candidate/exclusion limits and remain available across pages; off-page selected node/source options remain visible. Gear and route binding caches preserve selected facts across page changes. Same-catalog refresh preserves selections, while scope/content changes revalidate them. Async page responses from old scope/hash bindings are ignored or rejected. Active planning disables page changes while a calculation is pending. The gear catalog golden changed only for additive page metadata.

Validation performed:
- `cargo test --locked -p azeroth-app`: 204 tests pass, including 1,205 authored records, first/middle/final pages, missing/stale bindings, wrong hash, oversized request, and unexpected limit refusal.
- `cargo clippy --locked -p azeroth-app --all-targets -- -D warnings`: passes.
- `cargo fmt --all`; `git diff --check`: pass.
- `pnpm install --frozen-lockfile`, desktop typecheck/lint: pass.
- `pnpm --filter azeroth-desktop test:unit`: 35 files, 201 tests pass. Added page scope/hash rejection, selection retention, and real Gear screen page/refresh tests.
- Native `cargo check --locked --manifest-path apps/desktop/src-tauri/Cargo.toml` blocked by missing system D-Bus development library (`libdbus-sys` build failure). Native Windows interactive/scale measurements remain unverified.

No gameplay dataset, permissions, or operator evidence has been invented. This change makes lists accessible at catalog scale; full-corpus Windows performance and live acceptance still require the real accepted corpus and operator environment. Atlas and verified bundle import continue in separately owned work. Next: independent review, hosted checks, integration with Atlas command blocks, and native acceptance.

Independent review: coordinator and acceptance worker reviewed c808a7c with no important blocker. Core/main fb6303b merged normally at bc30147, resolving only the core module registration union. PR135 targets main for hosted checks; dependency PRs still integrate first. Acceptance's Atlas branch has merged this pagination implementation.
