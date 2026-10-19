"""The world-data pipeline (ADR 0007): one command from download to draft PR.

    python3 data-generators/pipeline.py                 # every stage
    python3 data-generators/pipeline.py --offline       # from the cache only
    python3 data-generators/pipeline.py --pages-dir ~/Downloads/wowhead
    python3 data-generators/pipeline.py --only content,gate --no-publish

Stages, in order (each one's outputs are the next one's inputs):

  detect    client build from .build.info -> release/v1/world-data-target.json
  generate  run_all.py (+ import_wowhead_pages.py with --pages-dir) -> data-generators/output/
  verify    azeroth-cli verify-world-data on the output
  register  to_facts.py, then register_facts.py: fact documents into
            content/source-materials/objects (LFS), the admission registry and
            content/source-build-v1.json; then source-sync and source-build
  content   world_to_content.py -> content/world/<build>/ (catalog.json, sources.json,
            conflicts.json), then build-content-v2 and verify-pack
  gate      release readiness with release/v1/accepted-origins.json
  publish   branch data/world-<build>-<date>, commit (LFS), push, draft PR

Interfaces between stages (owned by the V2.8 sub-tasks):

  register_facts.py --facts DIR --registry FILE --plan FILE --objects DIR
                    --updates FILE --notice FILE
  world_to_content.py --input DIR --target FILE --policy FILE --authored DIR --out DIR

This file is the V2.8 W0 skeleton; task W1 implements the stages.
"""
import argparse
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent
STAGES = ["detect", "generate", "verify", "register", "content", "gate", "publish"]
TARGET = ROOT / "release" / "v1" / "world-data-target.json"
POLICY = ROOT / "release" / "v1" / "accepted-origins.json"


def parse_args(argv=None):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--only", help="comma-separated stages (default: all, in order)")
    mode = p.add_mutually_exclusive_group()
    mode.add_argument("--offline", action="store_true", help="use only cached downloads")
    mode.add_argument("--refresh", action="store_true", help="ignore the download cache")
    p.add_argument("--pages-dir", help="folder of Wowhead list pages saved by the owner")
    p.add_argument("--client", help="WoW client folder holding .build.info (else probed / local.json)")
    p.add_argument("--no-network-sources", action="store_true",
                   help="skip the Wowhead generators (the weekly CI run)")
    p.add_argument("--no-publish", action="store_true", help="stop before the branch and draft PR")
    args = p.parse_args(argv)
    args.stages = [s.strip() for s in args.only.split(",")] if args.only else list(STAGES)
    unknown = [s for s in args.stages if s not in STAGES]
    if unknown:
        p.error(f"unknown stage(s): {', '.join(unknown)}; stages: {', '.join(STAGES)}")
    return args


def main(argv=None):
    args = parse_args(argv)
    for stage in STAGES:
        if stage in args.stages:
            print(f"stage {stage}: not implemented yet (V2.8 W1)")
    return 1


if __name__ == "__main__":
    sys.exit(main())
