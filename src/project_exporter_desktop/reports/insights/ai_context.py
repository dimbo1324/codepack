from __future__ import annotations

from collections import Counter
from pathlib import Path
from typing import Any

from ...utils.path_utils import rel_display
from ...utils.text_utils import format_bytes, safe_read_json
from ...utils.time_utils import human_now
from .config_report import find_config_files


def write_ai_context_pack(
    copied_root: Path, source_root: Path, output_file: Path, inventory: dict[str, Any]
) -> None:
    files: list[Path] = inventory["files"]
    dirs: list[Path] = inventory["dirs"]
    stack: dict[str, list[str]] = inventory["stack"]
    largest = sorted(inventory["sizes"], key=lambda item: item[1], reverse=True)[:10]
    language_count: Counter[str] = inventory["language_count"]
    configs = find_config_files(copied_root)
    package_json = safe_read_json(copied_root / "package.json")
    scripts = package_json.get("scripts") if package_json else None

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# AI Context Pack: {copied_root.name}\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write(
            "This file is intended to be pasted into ChatGPT/Codex together with the exported project when quick project understanding is needed.\n\n"
        )

        out.write("## Project summary\n\n")
        out.write(f"- Source root: `{source_root}`\n")
        out.write(f"- Copied root: `{copied_root}`\n")
        out.write(f"- Files: {len(files):,}\n")
        out.write(f"- Folders: {len(dirs):,}\n")
        out.write(f"- Copied size: {format_bytes(int(inventory['total_size']))}\n")

        out.write("\n## Detected stack\n\n")
        for group, values in stack.items():
            out.write(f"- **{group}**: {', '.join(values) if values else 'not detected'}\n")

        out.write("\n## Main languages\n\n")
        if language_count:
            for language, count in language_count.most_common(15):
                out.write(f"- {language}: {count:,} files\n")
        else:
            out.write("- No known language extensions detected.\n")

        out.write("\n## Scripts / commands\n\n")
        if isinstance(scripts, dict) and scripts:
            manager = "pnpm" if (copied_root / "pnpm-lock.yaml").exists() else "npm"
            for name, command in sorted(scripts.items(), key=lambda item: item[0].lower()):
                out.write(f"- `{manager} run {name}` — `{command}`\n")
        else:
            out.write("- No package.json scripts detected.\n")

        out.write("\n## Important configuration files\n\n")
        if configs:
            for path in configs[:80]:
                out.write(f"- `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- No common configuration files detected.\n")

        out.write("\n## Largest files\n\n")
        for path, size in largest:
            out.write(f"- `{rel_display(path, copied_root)}` — {format_bytes(size)}\n")

        out.write("\n## Suggested review order\n\n")
        suggestions = [
            "Read `00_project_profile.json` and `01_summary.txt` first.",
            "Use `13_runbook.md` to understand setup, run, and test commands.",
            "Use `15_architecture_report.md` and `16_key_files_report.md` before editing code.",
            "Use `14_dependency_graph.md` / `.mmd` to understand internal imports.",
            "Review `06_security_scan.txt` before sharing the export.",
            "Use `17_code_quality_report.md` and `23_refactoring_opportunities.md` to plan refactors.",
            "Use the `AI_CONTEXT/` folder for a multi-file ChatGPT/Codex handoff.",
        ]
        for suggestion in suggestions:
            out.write(f"- {suggestion}\n")
