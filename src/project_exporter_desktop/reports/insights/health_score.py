from __future__ import annotations

from pathlib import Path
from typing import Any

from ...constants import CONFIG_FILES, SOURCE_CODE_EXTENSIONS
from ...utils.inventory import extension_key
from ...utils.text_utils import format_bytes
from ...utils.time_utils import human_now


def _clamp(value: int) -> int:
    return max(0, min(100, value))


def write_project_health_report(copied_root: Path, output_file: Path, inventory: dict[str, Any]) -> None:
    files: list[Path] = list(inventory.get("files", []))
    sizes: list[tuple[Path, int]] = list(inventory.get("sizes", []))
    stack: dict[str, list[str]] = inventory.get("stack", {})

    source_files = [p for p in files if extension_key(p) in SOURCE_CODE_EXTENSIONS]
    test_files = [p for p in files if "test" in str(p).casefold() or "tests" in p.parts]
    docs = [p for p in files if p.name.casefold() in {"readme.md", "license", "changelog.md"} or extension_key(p) in {"md", "rst", "adoc"}]
    configs = [p for p in files if p.name in CONFIG_FILES]
    large_files = [(p, s) for p, s in sizes if s >= 500_000]

    architecture = 70
    if source_files and len(files) > 1:
        architecture += 8
    if any("src" in p.parts for p in files):
        architecture += 8
    if len(source_files) > 0 and len(source_files) < max(1, len(files)):
        architecture += 4

    security = 80
    suspicious_names = [p for p in files if p.name.casefold().startswith(".env") or "secret" in p.name.casefold()]
    security -= min(40, 10 * len(suspicious_names))
    if stack.get("testing"):
        security += 5

    maintainability = 70
    if docs:
        maintainability += 8
    if configs:
        maintainability += 6
    maintainability -= min(20, len(large_files) * 2)

    tests = 35 + min(45, len(test_files) * 5)
    if stack.get("testing"):
        tests += 15

    documentation = 35 + min(50, len(docs) * 8)
    if (copied_root / "README.md").exists():
        documentation += 15

    ai_readiness = 75
    if (copied_root.parent / "PROJECT_PROFILE.json").exists():
        ai_readiness += 10
    if source_files:
        ai_readiness += 5
    if suspicious_names:
        ai_readiness -= 15

    scores = {
        "Architecture": _clamp(architecture),
        "Security": _clamp(security),
        "Maintainability": _clamp(maintainability),
        "Testing signals": _clamp(tests),
        "Documentation": _clamp(documentation),
        "AI readiness": _clamp(ai_readiness),
    }
    overall = round(sum(scores.values()) / len(scores))

    with output_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write("# Project Health Report\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write(f"Overall score: **{overall}/100**\n\n")
        for name, score in scores.items():
            out.write(f"- {name}: **{score}/100**\n")
        out.write("\n## Signals used\n\n")
        out.write(f"- Files: {len(files):,}\n")
        out.write(f"- Source files: {len(source_files):,}\n")
        out.write(f"- Test-like files: {len(test_files):,}\n")
        out.write(f"- Documentation files: {len(docs):,}\n")
        out.write(f"- Config files: {len(configs):,}\n")
        out.write(f"- Total copied size: {format_bytes(int(inventory.get('total_size', 0)))}\n")
        out.write("\n## Interpretation\n\n")
        out.write("This is a heuristic score. Treat it as a triage signal, not as a formal audit.\n")
        if suspicious_names:
            out.write("\n## Security notes\n\n")
            for path in suspicious_names[:20]:
                out.write(f"- Review sensitive-looking file: `{path.relative_to(copied_root)}`\n")
