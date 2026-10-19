"""Validate the curated KR scope registry and export reviewable CSV tables.

docs/specification/features.json is the source of truth. Asset inventory regeneration does
not change scope decisions. Run after kr_feature_inventory.py when packs change.
"""
import argparse
import csv
import json
from collections import Counter
from pathlib import Path

from kr_tracking import metrics, phase_table, render_progress, validate_execution, validate_features


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate without rewriting exports")
    args = parser.parse_args()
    docs = Path(__file__).resolve().parents[1] / "docs"
    registry = json.loads((docs / "specification/features.json").read_text())
    features = registry["features"]
    with (docs / "evidence/inventory/kr_feature_candidates.csv").open(newline="") as stream:
        groups = list(csv.DictReader(stream))
    group_ids = {g["feature_id"] for g in groups}
    ids = {f["id"] for f in features}
    if len(ids) != len(features):
        raise ValueError("duplicate feature ID")
    by_id = {f["id"]: f for f in features}
    for feature in features:
        if feature["group_id"] and feature["group_id"] not in group_ids:
            raise ValueError(f"unknown group: {feature['id']}")
        if feature["release"] not in {"V1", "later", "drop_legacy"}:
            raise ValueError(f"undecided scope: {feature['id']}")
        for field in ("title", "server_responsibility", "acceptance", "open_questions", "owner"):
            if not feature[field].strip():
                raise ValueError(f"missing {field}: {feature['id']}")
        for dep in feature["depends_on"]:
            if dep not in ids or (feature["release"] == "V1" and by_id[dep]["release"] != "V1"):
                raise ValueError(f"invalid dependency: {feature['id']} -> {dep}")
    done, visiting = set(), set()

    def visit(fid):
        if fid in visiting:
            raise ValueError(f"dependency cycle: {fid}")
        if fid in done:
            return
        visiting.add(fid)
        for dep in by_id[fid]["depends_on"]:
            visit(dep)
        visiting.remove(fid)
        done.add(fid)

    for fid in ids:
        visit(fid)
    validate_features(features, registry["baseline"], docs.parent)
    execution = json.loads((docs / "planning/execution.json").read_text())
    validate_execution(execution, registry, docs.parent)
    scopes = []
    for group in groups:
        children = [f for f in features if f["group_id"] == group["feature_id"]]
        if not children:
            raise ValueError(f"group without scope disposition: {group['feature_id']}")
        counts = Counter(f["release"] for f in children)
        scopes.append(dict(group_id=group["feature_id"], title=group["feature_group"],
                           scope="V1_subset" if counts["V1"] else
                           "drop_legacy" if counts["drop_legacy"] == len(children) else "later",
                           feature_ids=" | ".join(f["id"] for f in children),
                           asset_matches=group["match_count"],
                           evidence_status=group["evidence_status"]))
    exported = [{k: " | ".join(v) if k == "depends_on" else
                 json.dumps(v, ensure_ascii=False, sort_keys=True) if isinstance(v, (dict, list)) else v
                 for k, v in f.items()} for f in features]
    for path, rows in [(docs / "specification/features.csv", exported),
                       (docs / "evidence/inventory/kr_scope_groups.csv", scopes)]:
        if args.check:
            with path.open(newline="") as stream:
                actual = list(csv.DictReader(stream))
            expected = [{k: str(v) for k, v in r.items()} for r in rows]
            if actual != expected:
                raise ValueError(f"stale export: {path.name}; run without --check")
        else:
            with path.open("w", newline="", encoding="utf-8") as stream:
                writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator="\n")
                writer.writeheader()
                writer.writerows(rows)
    plan_path = docs / execution["plan"].removeprefix("docs/")
    plan = plan_path.read_text()
    start, end = "<!-- PHASE_STATUS:START -->", "<!-- PHASE_STATUS:END -->"
    if plan.count(start) != 1 or plan.count(end) != 1:
        raise ValueError("canonical plan requires exactly one generated phase-status block")
    active = ", ".join(p["id"] + " " + p["title"] for p in execution["phases"]
                       if p["status"] in ("in_progress", "in_review", "blocked"))
    block = f"\nPha đang hoạt động: **{active or 'không có'}**.\n\n" + phase_table(execution) + "\n"
    before, rest = plan.split(start)
    _, after = rest.split(end)
    outputs = {docs / "planning/PROGRESS.md": render_progress(registry, execution),
               plan_path: before + start + block + end + after}
    for path, expected in outputs.items():
        if args.check:
            if not path.exists() or path.read_text() != expected:
                raise ValueError(f"stale generated progress: {path.name}; run without --check")
        else:
            path.write_text(expected, encoding="utf-8")
    print(json.dumps(dict(baseline=registry["baseline"], groups=len(groups), features=len(features),
                          scope=dict(Counter(f["release"] for f in features)), progress=metrics(features)), ensure_ascii=False))


if __name__ == "__main__":
    main()
