#!/usr/bin/env python3
"""Refresh original heuristic content from a bounded admitted class/level inventory.

This imports only inert JSON identities/ranges, never game coefficients or spells.
Canonicalize catalog.json with build-content-v2 after running this helper.
"""
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INPUT = "data-generators/output/quests/quests/vanilla_1_12/1.12.1/part-0000.json"
CLASSES = {1: "warrior", 2: "paladin", 4: "hunter", 8: "rogue", 16: "priest", 64: "shaman", 128: "mage", 256: "warlock", 1024: "druid"}
BANDS = [(1, 20), (21, 40), (41, 60)]
# Original judgements, thousandths per attribute; these are not game formulas.
WEIGHTS = {
    ("warrior", "melee_damage"): {"strength": 1000, "agility": 600, "stamina": 400},
    ("warrior", "tank"): {"stamina": 1000, "strength": 500, "agility": 500, "armor": 20},
    ("paladin", "melee_damage"): {"strength": 1000, "stamina": 500, "intellect": 300, "agility": 300},
    ("paladin", "healer"): {"intellect": 1000, "spirit": 500, "stamina": 400},
    ("paladin", "tank"): {"stamina": 1000, "strength": 500, "intellect": 300, "armor": 20},
    ("hunter", "ranged_damage"): {"agility": 1000, "stamina": 400, "intellect": 300},
    ("rogue", "melee_damage"): {"agility": 1000, "strength": 500, "stamina": 400},
    ("priest", "healer"): {"intellect": 1000, "spirit": 800, "stamina": 300},
    ("priest", "ranged_damage"): {"intellect": 1000, "spirit": 700, "stamina": 400},
    ("shaman", "melee_damage"): {"strength": 1000, "agility": 500, "stamina": 400, "intellect": 300},
    ("shaman", "ranged_damage"): {"intellect": 1000, "spirit": 500, "stamina": 400},
    ("shaman", "healer"): {"intellect": 1000, "spirit": 700, "stamina": 400},
    ("mage", "ranged_damage"): {"intellect": 1000, "spirit": 600, "stamina": 400},
    ("warlock", "ranged_damage"): {"intellect": 1000, "stamina": 800, "spirit": 500},
    ("druid", "melee_damage"): {"agility": 1000, "strength": 500, "stamina": 400},
    ("druid", "ranged_damage"): {"intellect": 1000, "spirit": 500, "stamina": 400},
    ("druid", "healer"): {"intellect": 1000, "spirit": 700, "stamina": 400},
    ("druid", "tank"): {"stamina": 1000, "agility": 600, "armor": 20},
}
ROLE_TEXT = {
    "tank": "Heuristic preparation: agree who leads pulls, review unknown equipment eligibility, and discuss a fallback with your group before departure.",
    "healer": "Heuristic preparation: review your declared healing role, supplies and expected session duration with the group before departure.",
    "melee_damage": "Heuristic preparation: compare the attributes your selected model actually scores and inspect unmodelled weapon damage, speed and effects manually.",
    "ranged_damage": "Heuristic preparation: compare the attributes your selected model actually scores and review supplies, unmodelled effects and expected session duration.",
}

def read(path, limit=16 * 1024 * 1024):
    with path.open("rb") as stream:
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise ValueError("authored coverage input exceeds byte limit")
    return raw, json.loads(raw)

def write(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")

def main():
    raw, source = read(ROOT / INPUT)
    digest = hashlib.sha256(raw).hexdigest()
    _, manifest = read(ROOT / "data-generators/output/run-manifest.json")
    entries = [f for g in manifest["generators"] for f in g["files"] if f["path"] == INPUT.removeprefix("data-generators/output/")]
    if len(entries) != 1 or entries[0]["sha256"] != digest or entries[0]["bytes"] != len(raw):
        raise ValueError("quest shard is not the exact registered run input")
    if source["origin"]["edition"] != "vanilla_1_12" or source["origin"]["build"] != {"status": "known", "value": "1.12.1"}:
        raise ValueError("historical class inventory origin changed; review required")
    if source.get("format") != "navzeroth-world-data" or source.get("schema_version") != 1 or source.get("dataset") != "quests.quests" or source.get("record_kind") != "quest":
        raise ValueError("unsupported class inventory source contract")
    records = source["records"]
    observed_classes = {r["required_classes"] for r in records if r.get("required_classes") in CLASSES}
    if observed_classes != set(CLASSES):
        raise ValueError("admitted single-class inventory changed; review required")
    levels = [r["min_level"] for r in records if isinstance(r.get("min_level"), int) and r["min_level"] >= 1]
    if not levels or min(levels) != 1 or max(levels) != 60:
        raise ValueError("admitted level inventory changed; review supported heuristic range")
    coverage = {"schema_version": 1, "kind": "authored_heuristic_coverage", "source": {"path": INPUT, "sha256": digest, "bytes": len(raw), "records": len(records), "origin": source["origin"], "confidence": source["confidence"], "source_ids": [s["id"] for s in source["sources"]]},
                "observed_quest_min_level_range": [min(r["min_level"] for r in records), max(r["min_level"] for r in records)], "supported_heuristic_levels": [1, 60], "bands": BANDS,
                "classes": [{"id": "class:" + name, "single_class_mask": mask, "single_class_quest_records": sum(r.get("required_classes") == mask for r in records), "single_class_quest_min_level_range": [min(r["min_level"] for r in records if r.get("required_classes") == mask), max(r["min_level"] for r in records if r.get("required_classes") == mask)], "roles": sorted(role for cls, role in WEIGHTS if cls == name)} for mask, name in sorted(CLASSES.items())],
                "limitations": ["Historical quest restrictions establish an input inventory, not a verified Forever class roster or level cap.", "Source min_level zero stays recorded as zero; authored player-level models start at one.", "Item requirement levels above 60 do not establish supported player-level coverage.", "All weights are original authored heuristics; bands reuse ratios as explicit extrapolation.", "Class proficiency, slot layouts, objective details and gameplay availability remain unresolved."]}
    write(ROOT / "docs/data/role-coverage-v1.json", coverage)
    path = ROOT / "content/authored/slice-v1/catalog.json"
    _, catalog = read(path)
    template = copy.deepcopy(catalog["models"][0])
    models = []
    for (cls, role), weights in sorted(WEIGHTS.items()):
        for lo, hi in BANDS:
            model = copy.deepcopy(template)
            model.update(id=f"model:{cls}-{role.replace('_', '-')}-{lo}-{hi}", **{"class": "class:" + cls}, minimum_level=lo, maximum_level=hi)
            model["evidence"] = ["ev-authored-roles-r2"]
            model["applicability"]["value"] = {"op": "role_is", "role": role}
            model["profile"].update(role=role, weights={"stat:"+k: v for k,v in weights.items()}, known_unmodeled=[f"Original authored heuristic for levels {lo}-{hi}; not measured, simulated or verified game mechanics.", "Ratios are reused across level bands as an explicit uncalibrated extrapolation.", "Weapon damage, speed, procs, pet damage, class abilities, talents, forms, set bonuses and on-use/on-equip effects are not modelled.", "Class proficiency and slot layout remain unknown; a model score does not establish equipment eligibility."])
            models.append(model)
    catalog["models"] = sorted(models, key=lambda m:m["id"])
    payload = catalog["reference"]["payload"]
    payload["revision"] = "navzeroth-authored-slice-r4-heuristic-reminders"
    original_evidence = next(e for e in payload["evidence"] if e["id"] == "ev-authored-v1")
    evidence = copy.deepcopy(original_evidence)
    evidence["id"] = "ev-authored-roles-r2"
    evidence["notes"] = "Original authored role/reminder coverage revision r2, 2026-10-19. Historical quest masks identify inventory only; coefficients and extrapolations are original heuristics, not verified game mechanics. See evidence/role-model-assumptions.md."
    evidence["observed_at_utc_ms"] = {"clock_trust":"uncertain","source":"unrecorded","status":"unknown"}
    reminder_evidence=copy.deepcopy(original_evidence)
    reminder_evidence["id"]="ev-authored-reminders-r4"
    reminder_evidence["notes"]="Original project-authored heuristic reminder statements, separately attributed from dungeon/map declarations. Publisher documentation records authorship only, not verified game mechanics, prerequisites or availability."
    reminder_evidence["conflicts"]=[]
    payload["evidence"] = sorted([original_evidence,evidence,reminder_evidence],key=lambda e:e["id"])

    rules = [r for r in payload["rules"] if not r["id"].startswith("reminder:role-")]
    for rule in rules:
        if rule["id"] == "reminder:slice-level-boundary":
            rule["id"] = rule["deduplication_key"] = "reminder:heuristic-level-boundary"
            rule["evidence"] = ["ev-authored-roles-r2"]
            rule["predicate"] = {"op":"level_at_least","level":61}
            rule["result_template"] = "Your level is outside the authored 1-60 heuristic range. This range is an advice boundary, not a verified game level cap."
    for rule in rules:
        rule["evidence"]=["ev-authored-reminders-r4"]
    role_template = copy.deepcopy(next(r for r in rules if r["id"] == "reminder:train-after-level-up"))
    for role, text in sorted(ROLE_TEXT.items()):
        for lo, hi in BANDS:
            rule = copy.deepcopy(role_template)
            rule.update(id=f"reminder:role-{role}-{lo}-{hi}", deduplication_key=f"reminder:role-{role}-{lo}-{hi}", trigger="snapshot_changed", result_template=f"Levels {lo}-{hi}: {text}", cooldown_ms=600000, priority=10, dismissal="session", expires_at_utc_ms=None)
            rule["predicate"] = {"op":"all","predicates":[{"op":"role_is","role":role},{"op":"level_at_least","level":lo},{"op":"not","predicate":{"op":"level_at_least","level":hi+1}}]}
            rule["evidence"] = ["ev-authored-roles-r2"]
            rules.append(rule)
    payload["rules"] = sorted(rules,key=lambda r:r["id"])
    write(path,catalog)
    matrix_path = ROOT / "content/authored/slice-v1/party-support.json"
    _, matrix = read(matrix_path)
    for row in matrix["rows"]:
        for reference in row["supporting_evidence"]:
            if reference["path"] in {"evidence/authoring-statement.md", "evidence/role-model-assumptions.md"}:
                evidence_bytes = (ROOT / "content/authored/slice-v1" / reference["path"]).read_bytes()
                reference["bytes"] = len(evidence_bytes)
                reference["sha256"] = hashlib.sha256(evidence_bytes).hexdigest()
        for dependency in row["dependencies"]:
            identity = json.loads(dependency["record_id"])
            identity[0] = payload["revision"]
            if identity[1] == "rules" and identity[2] == "reminder:slice-level-boundary":
                identity[2] = "reminder:heuristic-level-boundary"
            dependency["record_id"] = json.dumps(identity,separators=(",",":"))
    matrix["catalog_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    write(matrix_path,matrix)

if __name__ == "__main__":
    main()
