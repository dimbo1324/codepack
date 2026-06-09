from __future__ import annotations

from pathlib import Path
from typing import Any

from ...constants import CONFIG_FILES, SOURCE_CODE_EXTENSIONS
from ...utils.inventory import extension_key
from ...utils.text_utils import format_bytes
from ...utils.time_utils import human_now


def _clamp(value: int) -> int:
    return max(0, min(100, value))


def _bar(score: int) -> str:
    filled = round(score / 10)
    return "█" * filled + "░" * (10 - filled)


def write_project_health_report(
    copied_root: Path, output_file: Path, inventory: dict[str, Any]
) -> None:
    files: list[Path] = list(inventory.get("files", []))
    sizes: list[tuple[Path, int]] = list(inventory.get("sizes", []))
    stack: dict[str, list[str]] = inventory.get("stack", {})

    source_files = [p for p in files if extension_key(p) in SOURCE_CODE_EXTENSIONS]
    test_files = [p for p in files if "test" in str(p).casefold() or "tests" in p.parts]
    docs = [
        p
        for p in files
        if p.name.casefold() in {"readme.md", "license", "changelog.md"}
        or extension_key(p) in {"md", "rst", "adoc"}
    ]
    configs = [p for p in files if p.name in CONFIG_FILES]
    large_files = [(p, s) for p, s in sizes if s >= 500_000]
    suspicious_names = [
        p
        for p in files
        if p.name.casefold().startswith(".env")
        or "secret" in p.name.casefold()
        or "credential" in p.name.casefold()
    ]
    lockfiles = [
        p
        for p in files
        if p.name
        in {
            "pnpm-lock.yaml",
            "package-lock.json",
            "yarn.lock",
            "poetry.lock",
            "uv.lock",
            "go.sum",
            "Cargo.lock",
        }
    ]

    reasons: dict[str, list[str]] = {}

    architecture = 62
    reasons["Architecture"] = []
    if any("src" in p.parts for p in files):
        architecture += 12
        reasons["Architecture"].append("src/ style source layout detected")
    if any(
        part in {"services", "domain", "core", "utils", "reports"}
        for p in files
        for part in p.parts
    ):
        architecture += 10
        reasons["Architecture"].append("separated service/core utility modules detected")
    if len(source_files) > 0 and len(source_files) < max(1, len(files)):
        architecture += 6
        reasons["Architecture"].append("source files are not mixed with every exported file")
    if len([p for p, _ in large_files if extension_key(p) in SOURCE_CODE_EXTENSIONS]) > 3:
        architecture -= 8
        reasons["Architecture"].append("several large source files need decomposition")

    security = 82
    reasons["Security"] = []
    if suspicious_names:
        penalty = min(45, 12 * len(suspicious_names))
        security -= penalty
        reasons["Security"].append(f"{len(suspicious_names)} sensitive-looking filenames detected")
    else:
        security += 5
        reasons["Security"].append("no obvious sensitive filenames in exported copy")
    if lockfiles:
        security += 3
        reasons["Security"].append("dependency lockfile(s) detected")

    maintainability = 64
    reasons["Maintainability"] = []
    if docs:
        maintainability += 10
        reasons["Maintainability"].append("documentation files are present")
    if configs:
        maintainability += 8
        reasons["Maintainability"].append("standard config files are present")
    maintainability -= min(18, len(large_files) * 2)
    if large_files:
        reasons["Maintainability"].append(f"{len(large_files)} files are relatively large")

    tests = 30 + min(42, len(test_files) * 6)
    reasons["Testing"] = []
    if stack.get("testing"):
        tests += 18
        reasons["Testing"].append("testing framework detected")
    if test_files:
        reasons["Testing"].append(f"{len(test_files)} test-like files detected")
    else:
        reasons["Testing"].append("no test-like files detected")

    documentation = 30 + min(48, len(docs) * 8)
    reasons["Documentation"] = []
    if (copied_root / "README.md").exists():
        documentation += 15
        reasons["Documentation"].append("README.md exists")
    if (copied_root / "docs").exists():
        documentation += 7
        reasons["Documentation"].append("docs/ folder exists")
    if not docs:
        reasons["Documentation"].append("documentation is minimal or absent")

    dep_hygiene = 62
    reasons["Dependency Hygiene"] = []
    if lockfiles:
        dep_hygiene += 18
        reasons["Dependency Hygiene"].append("lockfiles improve reproducibility")
    if stack.get("package_managers"):
        dep_hygiene += 8
        reasons["Dependency Hygiene"].append("package manager detected")
    if len(stack.get("package_managers", [])) > 2:
        dep_hygiene -= 8
        reasons["Dependency Hygiene"].append(
            "multiple package managers may increase maintenance cost"
        )

    ai_readiness = 78
    reasons["AI Readiness"] = []
    if (copied_root.parent / "PROJECT_PROFILE.json").exists():
        ai_readiness += 8
        reasons["AI Readiness"].append("PROJECT_PROFILE.json is available")
    if source_files:
        ai_readiness += 5
        reasons["AI Readiness"].append("source files are included")
    if suspicious_names:
        ai_readiness -= 12
        reasons["AI Readiness"].append("sensitive-looking files reduce safe sharing readiness")

    export_safety = 92
    reasons["Export Safety"] = []
    if suspicious_names:
        export_safety -= min(30, 10 * len(suspicious_names))
        reasons["Export Safety"].append("review sensitive-looking files before sharing")
    else:
        reasons["Export Safety"].append("exported copy appears safe by filename heuristics")

    scores = {
        "Architecture": _clamp(architecture),
        "Security": _clamp(security),
        "Maintainability": _clamp(maintainability),
        "Testing": _clamp(tests),
        "Documentation": _clamp(documentation),
        "Dependency Hygiene": _clamp(dep_hygiene),
        "AI Readiness": _clamp(ai_readiness),
        "Export Safety": _clamp(export_safety),
    }
    overall = round(sum(scores.values()) / len(scores))

    with output_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write("# Project Health Report\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write(f"Overall score: **{overall}/100**\n\n")
        out.write("| Area | Score | Signal |\n")
        out.write("|---|---:|---|\n")
        for name, score in scores.items():
            out.write(f"| {name} | {score}/100 | `{_bar(score)}` |\n")
        out.write("\n## Why these scores\n\n")
        for name, score in scores.items():
            out.write(f"### {name}: {score}/100\n\n")
            for reason in reasons.get(name, []) or ["No specific signal."]:
                out.write(f"- {reason}\n")
            out.write("\n")
        out.write("## Raw signals\n\n")
        out.write(f"- Files: {len(files):,}\n")
        out.write(f"- Source files: {len(source_files):,}\n")
        out.write(f"- Test-like files: {len(test_files):,}\n")
        out.write(f"- Documentation files: {len(docs):,}\n")
        out.write(f"- Config files: {len(configs):,}\n")
        out.write(f"- Lockfiles: {len(lockfiles):,}\n")
        out.write(f"- Total copied size: {format_bytes(int(inventory.get('total_size', 0)))}\n")
        out.write("\nThis is a heuristic triage score, not a formal audit.\n")
