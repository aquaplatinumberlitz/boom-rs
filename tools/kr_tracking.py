"""Validate completion records and render progress; never execute claimed test commands.

Checks prove record consistency/reference existence, not acceptance truth. Reviewers
remain responsible for verifying the implementation and evidence at the recorded revision.
"""
import datetime
import hashlib
import json
import re
import subprocess
from collections import Counter
from functools import lru_cache
from pathlib import Path


STATES = ("not_started", "in_progress", "blocked", "in_review", "done")
LABELS = dict(not_started="Chưa làm", in_progress="Đang làm", blocked="Bị chặn",
              in_review="Chờ review", done="Đã nghiệm thu")


@lru_cache(maxsize=256)
def check_commit(root, commit):
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("integration commit must be an immutable full SHA-1")
    result = subprocess.run(["git", "cat-file", "-t", commit], cwd=root,
                            capture_output=True, text=True)
    if result.returncode or result.stdout.strip() != "commit":
        raise ValueError(f"commit does not exist in this checkout: {commit}")


def check_integration(root, commit):
    check_commit(str(root), commit)
    result = subprocess.run(["git", "merge-base", "--is-ancestor", commit, "HEAD"],
                            cwd=root, capture_output=True, text=True)
    if result.returncode:
        raise ValueError(f"integration commit is not integrated into this checkout HEAD: {commit}")


def check_evidence(root, refs):
    if not isinstance(refs, list) or not refs:
        raise ValueError("completion requires evidence references")
    for ref in refs:
        if not isinstance(ref, str) or not ref:
            raise ValueError("invalid evidence reference")
        if ref.startswith("git:"):
            check_commit(str(root), ref[4:])
        else:
            path = Path(ref.split("#", 1)[0])
            resolved = (root / path).resolve()
            if path.is_absolute() or not resolved.is_relative_to(root.resolve()) or not resolved.is_file():
                raise ValueError(f"evidence must be an existing repository file or git SHA: {ref}")


def validate_features(features, baseline, root):
    by_id = {f["id"]: f for f in features}
    for f in features:
        status = f["implementation"]
        if status not in STATES:
            raise ValueError(f"{f['id']}: unknown implementation state {status}")
        t = f.get("tracking")
        if not isinstance(t, dict):
            raise ValueError(f"{f['id']}: missing tracking record")
        datetime.date.fromisoformat(t["updated_at"])
        if t["verification"]["status"] not in ("not_run", "passed", "failed"):
            raise ValueError(f"{f['id']}: invalid verification state")
        if t["review"]["status"] not in ("not_requested", "pending", "changes_requested", "approved"):
            raise ValueError(f"{f['id']}: invalid review state")
        if f["release"] == "drop_legacy" and status != "not_started":
            raise ValueError(f"{f['id']}: excluded legacy work must not earn completion credit")
        if status != "not_started" and not t["assignee"].strip():
            raise ValueError(f"{f['id']}: an active task needs a named assignee")
        if status == "blocked" and not t["blocker"].strip():
            raise ValueError(f"{f['id']}: blocked state needs a reason")
        if status in ("in_progress", "blocked", "in_review") and not t["next_action"].strip():
            raise ValueError(f"{f['id']}: active task needs next_action")
        if status in ("in_progress", "in_review", "done") and (
                t["accepted_baseline"] != baseline or not t["accepted_spec"].strip()):
            raise ValueError(f"{f['id']}: implementation needs accepted scope/spec revisions")
        if status in ("in_review", "done"):
            if not t["reviewer"].strip():
                raise ValueError(f"{f['id']}: review requires a named reviewer")
            if t["verification"]["status"] != "passed" or not all(
                    t["verification"][key].strip() for key in ("command", "result")):
                raise ValueError(f"{f['id']}: review/completion requires passed verification with command/result")
            check_evidence(root, t["evidence"])
        if status == "done":
            if t["assignee"].casefold().strip() == t["reviewer"].casefold().strip():
                raise ValueError(f"{f['id']}: completion requires independent review")
            if t["review"]["status"] != "approved" or not t["review"]["result"].strip():
                raise ValueError(f"{f['id']}: completion requires an approved review")
            if t["accepted_baseline"] != baseline or not t["accepted_spec"].strip():
                raise ValueError(f"{f['id']}: completion must bind accepted scope/spec revisions")
            if t["blocker"].strip():
                raise ValueError(f"{f['id']}: completion cannot retain an acceptance blocker")
            check_integration(root, t["integration_commit"])
            if any(by_id[dep]["implementation"] != "done" for dep in f["depends_on"]):
                raise ValueError(f"{f['id']}: dependency is not done; review dependent work again")


def validate_execution(execution, registry, root):
    if execution["baseline"] != registry["baseline"]:
        raise ValueError("plan execution/catalog baseline mismatch")
    if execution["baseline_scope_sha256"] != scope_signature(registry["features"]):
        raise ValueError("catalog membership/scope changed: review and record a new scope baseline before recalculating progress")
    if execution["plan"] != "docs/planning/REWRITE_PLAN.md":
        raise ValueError("execution must point to the canonical rewrite plan")
    phases = {p["id"]: p for p in execution["phases"]}
    if len(phases) != len(execution["phases"]) or set(phases) != {f"P{i}" for i in range(10)}:
        raise ValueError("plan must track unique phases P0–P9")
    if execution["readiness_phases"] != ["P0", "P1", "P2", "P3"]:
        raise ValueError("readiness denominator is P0–P3, independent of feature completion")
    for p in phases.values():
        if p["status"] not in STATES or not p["owner"].strip():
            raise ValueError(f"invalid phase: {p['id']}")
        if p["status"] == "blocked" and not p["blocker"].strip():
            raise ValueError(f"phase {p['id']}: missing blocker")
        if any(dep not in phases or dep == p["id"] for dep in p["depends_on"]):
            raise ValueError(f"phase {p['id']}: invalid dependency")
        if p["status"] == "done":
            if any(phases[d]["status"] != "done" for d in p["depends_on"]):
                raise ValueError(f"phase {p['id']}: predecessor not done")
            check_evidence(root, p["evidence"])
            check_integration(root, p["integration_commit"])
            if not p["reviewer"].strip() or not p["verification"].strip():
                raise ValueError(f"phase {p['id']}: completion requires review/verification")
    seen, active = set(), set()

    def visit(pid):
        if pid in active:
            raise ValueError("phase dependency cycle")
        if pid in seen:
            return
        active.add(pid)
        for dep in phases[pid]["depends_on"]:
            visit(dep)
        active.remove(pid)
        seen.add(pid)

    for pid in phases:
        visit(pid)
    gates = execution["release_gates"]
    if len(gates) != 6 or {g["id"] for g in gates} != {f"G{i}" for i in range(1, 7)}:
        raise ValueError("release must track exactly gates G1–G6")
    for gate in gates:
        if gate["status"] not in ("open", "blocked", "in_review", "passed"):
            raise ValueError(f"invalid release gate: {gate['id']}")
        if gate["status"] == "blocked" and not gate["blocker"].strip():
            raise ValueError(f"gate {gate['id']}: missing blocker")
        if gate["status"] == "passed":
            check_evidence(root, gate["evidence"])
            check_integration(root, gate["integration_commit"])
            if not gate["reviewer"].strip() or not gate["verification"].strip():
                raise ValueError(f"gate {gate['id']}: missing review/verification")
    v1 = [f for f in registry["features"] if f["release"] == "V1"]
    if phases["P8"]["status"] == "done" and (
            not all(f["implementation"] == "done" for f in v1) or
            not all(g["status"] == "passed" for g in gates)):
        raise ValueError("P8 completion requires all V1 features and all six release gates")
    if phases["P9"]["status"] == "done" and any(
            f["implementation"] != "done" for f in registry["features"] if f["release"] != "drop_legacy"):
        raise ValueError("P9 completion requires all in-scope catalog features")


def scope_signature(features):
    membership = sorted((f["id"], f["release"]) for f in features)
    return hashlib.sha256(json.dumps(membership, separators=(",", ":")).encode()).hexdigest()


def metrics(features):
    result = {}
    for name, subset in [("V1", [f for f in features if f["release"] == "V1"]),
                         ("catalog", [f for f in features if f["release"] != "drop_legacy"])]:
        counts = Counter(f["implementation"] for f in subset)
        result[name] = dict(total=len(subset), done=counts["done"],
                            percent=round(100 * counts["done"] / len(subset), 2) if subset else 0,
                            states={s: counts[s] for s in STATES})
    result["excluded"] = sum(f["release"] == "drop_legacy" for f in features)
    return result


def cell(value):
    return str(value or "—").replace("|", "\\|").replace("\n", " ")


def evidence_links(refs):
    result = []
    for ref in refs:
        if ref.startswith("git:"):
            result.append("`" + ref[4:16] + "`")
        else:
            result.append(f"[{Path(ref.split('#', 1)[0]).name}](<../../{ref}>)")
    return "; ".join(result) or "—"


def phase_table(execution):
    lines = ["| Pha | Trạng thái | Owner | Đầu ra | Gate hoàn thành |",
             "|---|---|---|---|---|"]
    for p in execution["phases"]:
        lines.append("| " + " | ".join(map(cell, [p["id"] + " " + p["title"], LABELS[p["status"]],
                                                    p["owner"], p["deliverable"], p["exit_gate"]])) + " |")
    return "\n".join(lines)


def render_progress(registry, execution):
    m = metrics(registry["features"])
    phases = {p["id"]: p for p in execution["phases"]}
    prep = sum(phases[p]["status"] == "done" for p in execution["readiness_phases"])
    passed = sum(g["status"] == "passed" for g in execution["release_gates"])
    active = ", ".join(p["id"] for p in execution["phases"] if p["status"] in ("in_progress", "in_review", "blocked"))
    lines = ["# Tiến độ so với catalog KR", "", "> GENERATED — không sửa tay. Chạy `python3 tools/kr_catalog.py`.",
             "", f"Baseline mẫu số: **`{registry['baseline']}`**.",
             "Nguồn: [features.json](../specification/features.json) và [execution.json](execution.json).",
             "Kế hoạch thực hiện: [REWRITE_PLAN](REWRITE_PLAN.md); danh mục: [FEATURE_CATALOG](../specification/FEATURE_CATALOG.md).", "",
             "## Số mục đã nghiệm thu", "", "| Phạm vi | Done / tổng | Tỷ lệ số mục |", "|---|---:|---:|"]
    for key, label in [("V1", "Release 1"), ("catalog", "Toàn catalog cần triển khai")]:
        v = m[key]
        lines.append(f"| {label} | {v['done']} / {v['total']} | {v['percent']:.2f}% |")
    lines += ["", f"Có **{m['excluded']} mục drop_legacy** hiển thị riêng trong CSV, không cộng vào done hoặc mẫu số.",
              "Đây là tỷ lệ **số mục nghiệm thu**, không phải % công sức, thời gian hay fidelity KR.",
              "Không chia điểm cho in_progress/in_review/blocked. Scope đổi phải đổi revision và giải thích mẫu số.",
              "", "| Trạng thái | V1 | Toàn catalog cần triển khai |", "|---|---:|---:|"]
    for status in STATES:
        lines.append(f"| {LABELS[status]} | {m['V1']['states'][status]} | {m['catalog']['states'][status]} |")
    lines += ["", "## Planning và gate phát hành — không cộng vào feature %", "",
              f"Pha đang hoạt động: **{active or 'không có'}**. Chuẩn bị P0–P3: **{prep}/4 pha hoàn tất**; các pha không có độ lớn bằng nhau.",
              f"Cổng phát hành: **{passed}/6 passed**. Milestone P8: **{LABELS[phases['P8']['status']]}**.",
              "Chỉ release khi feature V1 đã nghiệm thu, tất cả gate passed và P8 done.", "", phase_table(execution), "",
              "| Gate | Trạng thái | Owner | Bằng chứng |", "|---|---|---|---|"]
    for gate in execution["release_gates"]:
        lines.append("| " + " | ".join(map(cell, [gate["id"] + " " + gate["title"], gate["status"], gate["owner"], evidence_links(gate["evidence"])])) + " |")
    lines += ["", "## Từng feature V1", "", "Owner là luồng; assignee/reviewer là người hoặc agent nhận task cụ thể.",
              "Chi tiết mọi mục, proof/review/dependency nằm trong [CSV](../specification/features.csv) và JSON nguồn.",
              "", "| ID | Feature | Trạng thái | Owner / assignee | Reviewer | Bước tiếp / blocker | Bằng chứng |", "|---|---|---|---|---|---|---|"]
    for f in registry["features"]:
        if f["release"] != "V1":
            continue
        t = f["tracking"]
        lines.append("| " + " | ".join(map(cell, [f["id"], f["title"], LABELS[f["implementation"]],
                     f["owner"] + " / " + (t["assignee"] or "chưa giao"), t["reviewer"], t["blocker"] or t["next_action"], evidence_links(t["evidence"])])) + " |")
    lines += ["", "## Các mục catalog ngoài V1", "", "Các mục này vẫn tính vào mẫu số toàn catalog; chưa tự mở scope triển khai.",
              "", "| ID | Feature | Trạng thái | Assignee | Bằng chứng |", "|---|---|---|---|---|"]
    for f in registry["features"]:
        if f["release"] != "later":
            continue
        t = f["tracking"]
        lines.append("| " + " | ".join(map(cell, [f["id"], f["title"], LABELS[f["implementation"]],
                                                  t["assignee"], evidence_links(t["evidence"])])) + " |")
    lines += ["", "## Không triển khai legacy — không tính done", ""]
    lines.extend(f"- `{f['id']}`: {f['title']}" for f in registry["features"] if f["release"] == "drop_legacy")
    lines += ["", "## Agent phải cập nhật gì", "",
              "Nhận task theo plan, ghi ID và assignee; sửa trạng thái/proof/review trong JSON nguồn rồi sinh lại report.",
              "Done cần commit đã tích hợp vào HEAD của checkout tích hợp, baseline/spec được chấp nhận, verification passed, reviewer độc lập approved,",
              "bằng chứng tồn tại và dependency done. Validator chỉ kiểm tính nhất quán/tham chiếu; không tự xác minh sự thật của proof.",
              "Planning/research cập nhật execution.json và QUESTIONS tương ứng; không đánh dấu feature done vì đã viết spec hoặc tìm thấy asset.", ""]
    return "\n".join(lines)
