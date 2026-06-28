from __future__ import annotations

from pathlib import Path
from typing import Any

from ...services.prompt_builder import build_custom_prompt
from ...utils.inventory import iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import format_bytes, safe_read_json
from ...utils.time_utils import human_now
from .config_report import find_config_files
from .project_profile import build_project_profile


def _write(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8", newline="\n", errors="replace")


def write_ai_context_folder(
    copied_root: Path, source_root: Path, output_dir: Path, inventory: dict[str, Any]
) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    profile = build_project_profile(copied_root, source_root, inventory)
    configs = find_config_files(copied_root)
    sizes = sorted(inventory["sizes"], key=lambda item: item[1], reverse=True)
    package_json = safe_read_json(copied_root / "package.json")
    scripts = package_json.get("scripts") if package_json else None

    _write(
        output_dir / "00_PROJECT_OVERVIEW.md",
        "\n".join(
            [
                f"# Project Overview: {copied_root.name}",
                "",
                f"Generated: {human_now()}",
                "",
                f"- Project type: **{profile['project_type']}**",
                f"- Risk level: **{profile['risk_level']}**",
                f"- Files: **{profile['counts']['files']:,}**",
                f"- Folders: **{profile['counts']['folders']:,}**",
                f"- Size: **{format_bytes(profile['counts']['total_size_bytes'])}**",
                "",
                "## Detected stack",
                "",
                *(f"- {item}" for item in profile["detected_stack"] or ["not detected"]),
                "",
                "## Risk reasons",
                "",
                *(f"- {item}" for item in profile["risk_reasons"]),
            ]
        )
        + "\n",
    )

    _write(
        output_dir / "01_ARCHITECTURE.md",
        "# Architecture Reading Guide\n\nRead these generated reports first:\n\n"
        "1. `../01_summary.txt`\n"
        "2. `../15_architecture_report.md`\n"
        "3. `../16_key_files_report.md`\n"
        "4. `../14_dependency_graph.md`\n"
        "5. `../23_refactoring_opportunities.md`\n\n",
    )

    tree_lines = ["# File Tree Snapshot", ""]
    for path in sorted(iter_project_files(copied_root), key=lambda p: str(p).lower())[:1000]:
        tree_lines.append(f"- `{rel_display(path, copied_root)}`")
    _write(output_dir / "02_FILE_TREE.md", "\n".join(tree_lines) + "\n")

    entry_lines = ["# Entrypoints", ""]
    for entry in profile["entrypoints"] or ["No obvious entrypoint detected."]:
        entry_lines.append(
            f"- `{entry}`" if entry != "No obvious entrypoint detected." else f"- {entry}"
        )
    _write(output_dir / "03_ENTRYPOINTS.md", "\n".join(entry_lines) + "\n")

    key_lines = ["# Key Files Reading Order", ""]
    for path, size in sizes[:30]:
        key_lines.append(f"- `{rel_display(path, copied_root)}` — {format_bytes(size)}")
    _write(output_dir / "04_KEY_FILES.md", "\n".join(key_lines) + "\n")

    deps_lines = ["# Dependencies / Commands", ""]
    deps_lines.append("## Commands")
    deps_lines.append("")
    for group, commands in profile["commands"].items():
        deps_lines.append(f"### {group}")
        for command in commands:
            deps_lines.append(f"- `{command}`")
        deps_lines.append("")
    deps_lines.append("## Config files")
    deps_lines.append("")
    for path in configs[:100]:
        deps_lines.append(f"- `{rel_display(path, copied_root)}`")
    _write(output_dir / "05_DEPENDENCIES.md", "\n".join(deps_lines) + "\n")

    _write(
        output_dir / "06_SECURITY_NOTES.md",
        "# Security Notes\n\nReview `../06_security_scan.txt` before sharing this export. "
        "Generated scanners are heuristic, so manually check `.env`, credentials, tokens, private keys, and Git history.\n",
    )

    _write(
        output_dir / "07_TODO_FIXME.md",
        "# TODO / FIXME\n\nSee `../07_todo_fixme.txt` for extracted technical-debt markers.\n",
    )

    _write(
        output_dir / "08_REFACTORING_TARGETS.md",
        "# Refactoring Targets\n\nSee `../23_refactoring_opportunities.md` and `../17_code_quality_report.md`.\n",
    )

    _write(output_dir / "09_PROMPT_FOR_CODEX.md", build_custom_prompt(copied_root.name))

    scripts_lines = ["# Scripts", ""]
    if isinstance(scripts, dict) and scripts:
        for name, command in sorted(scripts.items(), key=lambda item: item[0].lower()):
            scripts_lines.append(f"- `{name}` → `{command}`")
    else:
        scripts_lines.append("- No package.json scripts detected.")
    _write(output_dir / "10_SCRIPTS.md", "\n".join(scripts_lines) + "\n")
