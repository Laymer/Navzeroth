#!/usr/bin/env bash
# Regenerates every Rust-written golden and fails when the working tree changes.
# Each writer was run twice and produced identical bytes (see fixtures/README.md);
# a writer that is not deterministic must not be added here.
set -euo pipefail
cd "$(dirname "$0")/.."

export UPDATE_DESKTOP_FIXTURES=1
export UPDATE_GEAR_GOLDENS=1
export UPDATE_TODAY_GOLDEN=1
export UPDATE_REFERENCE_GOLDEN=1
export UPDATE_ROUTE_ACQUISITION_GOLDENS=1
export UPDATE_CONTENT_DISPOSITION_GOLDENS=1
export NAVZEROTH_UPDATE_PREPARATION_FIXTURE=1
export UPDATE_LUA_GEAR_FIXTURES=1
export UPDATE_WORLD_BROWSER_FIXTURES=1

before="$(git status --porcelain)"
cargo test --locked -p azeroth-core -p azeroth-app
after="$(git status --porcelain)"
if [ "$before" != "$after" ] || ! git diff --quiet; then
  echo "Golden drift: a writer changed committed fixtures." >&2
  git status --short >&2
  git diff --stat >&2
  exit 1
fi
echo "No golden drift."
