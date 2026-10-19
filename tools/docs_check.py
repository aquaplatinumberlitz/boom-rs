"""Check local Markdown targets and the project's documentation entry points.

Offline only: no HTTP requests or gameplay verification. Fragment anchors and
bare paths inside prose/code blocks are outside this check's coverage.
Ignored research caches are skipped; mandatory/migrated documents must remain versionable.
"""
import json
import re
import subprocess
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
REQUIRED = [
    "AGENTS.md", "CLAUDE.md", "docs/README.md",
    "docs/planning/REWRITE_PLAN.md", "docs/planning/SCOPE_V1.md",
    "docs/planning/ARCHITECTURE.md", "docs/research/QUESTIONS.md",
    "docs/specification/features.json",
    "docs/specification/FEATURE_CATALOG.md", "docs/planning/PROGRESS.md",
    "docs/planning/execution.json",
]


def main():
    errors, checked = [], 0
    for name in REQUIRED:
        if not (ROOT / name).is_file():
            errors.append(f"missing mandatory entry point: {name}")
    root_files = [p.name for p in (ROOT / "docs").iterdir() if p.is_file()]
    if root_files != ["README.md"]:
        errors.append(f"docs root must contain only README.md, got {sorted(root_files)}")
    if "@AGENTS.md" not in (ROOT / "CLAUDE.md").read_text():
        errors.append("CLAUDE.md must import the shared AGENTS.md")
    config = json.loads((ROOT / "opencode.json").read_text())
    if "AGENTS.md" not in config.get("instructions", []):
        errors.append("OpenCode instructions must include the shared AGENTS.md")
    files = [ROOT / "README.md", ROOT / "AGENTS.md", ROOT / "CLAUDE.md"]
    files += sorted((ROOT / "docs").rglob("*.md"))
    files += sorted((ROOT / "l10n").glob("*.md"))
    cache_check = subprocess.run(["git", "check-ignore", "--stdin", "-z"], cwd=ROOT,
                                 input="\0".join(str(p.relative_to(ROOT)) for p in files) + "\0",
                                 text=True, capture_output=True)
    if cache_check.returncode not in (0, 1):
        raise RuntimeError(f"git ignore check failed: {cache_check.stderr.strip()}")
    ignored_cache = set(filter(None, cache_check.stdout.split("\0")))
    files = [p for p in files if str(p.relative_to(ROOT)) not in ignored_cache]
    for path in files:
        # Fenced examples are not navigable links; ignore them.
        text = re.sub(r"(?ms)^```[^\n]*\n.*?^```[ \t]*$", "", path.read_text())
        for match in re.finditer(r"\]\((<[^>]+>|[^\s)]+)(?:\s+\"[^\"]*\")?\)", text):
            target = match.group(1).strip("<>")
            if target.startswith("#") or re.match(r"^[a-zA-Z][\w+.-]*:", target):
                continue
            filename = unquote(target.split("#", 1)[0].split("?", 1)[0])
            checked += 1
            if not (path.parent / filename).exists():
                line = text.count("\n", 0, match.start()) + 1
                errors.append(f"{path.relative_to(ROOT)}:{line}: missing target {target}")
    migration = json.loads((ROOT / "docs/evidence/DOCS_MIGRATION.json").read_text())
    for old, new in migration["old_to_new"].items():
        if not (ROOT / new).exists():
            errors.append(f"migration destination missing: {old} -> {new}")
    candidates = sorted(set(REQUIRED + [str(p.relative_to(ROOT)) for p in files] +
                            list(migration["old_to_new"].values())))
    ignored = subprocess.run(["git", "check-ignore", "--stdin", "-z"], cwd=ROOT,
                             input="\0".join(candidates) + "\0", text=True,
                             capture_output=True)
    if ignored.returncode not in (0, 1):
        errors.append(f"git ignore check failed: {ignored.stderr.strip()}")
    else:
        errors.extend(f"documentation artifact is ignored by Git: {name}"
                      for name in ignored.stdout.split("\0") if name)
    if errors:
        print("\n".join(errors))
        raise SystemExit(1)
    print(f"Docs OK: {len(files)} Markdown files, {checked} local links, "
          f"{len(migration['old_to_new'])} migration records; shared rules/entry points present.")


if __name__ == "__main__":
    main()
