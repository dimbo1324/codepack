from __future__ import annotations

from collections import defaultdict
from pathlib import Path
from typing import Any

from ...utils.inventory import iter_project_dirs, iter_project_files
from ...utils.path_utils import rel_display
from ...utils.time_utils import human_now

_LAYER_HINTS: dict[str, tuple[str, ...]] = {
    "UI / presentation": ("ui", "views", "pages", "components", "widgets", "layouts", "screens"),
    "Application / orchestration": (
        "services",
        "application",
        "usecases",
        "use_cases",
        "handlers",
        "controllers",
    ),
    "Domain / business logic": ("domain", "core", "models", "entities", "business"),
    "Data access / persistence": (
        "repositories",
        "repository",
        "dao",
        "db",
        "database",
        "migrations",
        "storage",
    ),
    "API / transport": ("api", "routes", "routers", "endpoints", "http", "server"),
    "Configuration": ("config", "settings", "infra", "infrastructure"),
    "Tests": ("tests", "test", "spec", "__tests__"),
    "Utilities": ("utils", "helpers", "lib", "shared"),
}

_ENTRYPOINT_NAMES = {
    "main.py",
    "__main__.py",
    "app.py",
    "server.py",
    "manage.py",
    "index.ts",
    "index.tsx",
    "main.ts",
    "main.tsx",
    "main.go",
}


def _group_dirs_by_layer(root: Path) -> dict[str, list[Path]]:
    groups: dict[str, list[Path]] = defaultdict(list)
    for directory in iter_project_dirs(root):
        parts = {part.lower() for part in directory.relative_to(root).parts}
        for layer, hints in _LAYER_HINTS.items():
            if parts.intersection(hints):
                groups[layer].append(directory)
                break
    return {key: sorted(set(value), key=lambda p: str(p).lower()) for key, value in groups.items()}


def _find_entrypoints(root: Path) -> list[Path]:
    result: list[Path] = []
    for path in iter_project_files(root):
        name = path.name.lower()
        if (
            name in _ENTRYPOINT_NAMES
            or name.startswith("vite.config")
            or name.startswith("next.config")
        ):
            result.append(path)
    return sorted(result, key=lambda p: str(p).lower())


def _top_level_dirs(root: Path) -> list[Path]:
    return sorted([p for p in root.iterdir() if p.is_dir()], key=lambda p: p.name.lower())


def write_architecture_report(
    copied_root: Path, output_file: Path, inventory: dict[str, Any]
) -> None:
    layer_dirs = _group_dirs_by_layer(copied_root)
    entrypoints = _find_entrypoints(copied_root)
    stack: dict[str, list[str]] = inventory["stack"]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# Architecture Report: {copied_root.name}\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write(
            "This report is a static architecture map based on file/folder conventions and detected tooling.\n\n"
        )

        out.write("## Detected technology groups\n\n")
        for group, values in stack.items():
            out.write(f"- **{group}**: {', '.join(values) if values else 'not detected'}\n")

        out.write("\n## Top-level structure\n\n")
        top_dirs = _top_level_dirs(copied_root)
        if top_dirs:
            for directory in top_dirs[:80]:
                out.write(f"- `{rel_display(directory, copied_root)}`\n")
        else:
            out.write("- No top-level folders detected.\n")

        out.write("\n## Layer map\n\n")
        for layer in _LAYER_HINTS:
            out.write(f"### {layer}\n\n")
            dirs = layer_dirs.get(layer, [])
            if dirs:
                for directory in dirs[:80]:
                    out.write(f"- `{rel_display(directory, copied_root)}`\n")
                if len(dirs) > 80:
                    out.write(f"- ... and {len(dirs) - 80:,} more\n")
            else:
                out.write("- not detected\n")
            out.write("\n")

        out.write("## Entrypoints and bootstrap/config files\n\n")
        if entrypoints:
            for path in entrypoints[:100]:
                out.write(f"- `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- No obvious entrypoint detected.\n")

        out.write("\n## Extension points\n\n")
        out.write(
            "- Add new report modules under `reports/insights/` and register them in `orchestrator.py`.\n"
        )
        out.write(
            "- Add new copy/exclusion rules in `config.py`, `constants.py`, and `copy_service.py`.\n"
        )
        out.write(
            "- Add UI options in `ui/app_window.py`; keep long-running work in the exporter thread.\n"
        )
        out.write("- Keep generated-report writers pure: input paths in, report file out.\n")

        out.write("\n## Potential architectural risks\n\n")
        if not layer_dirs:
            out.write(
                "- No conventional layers detected; project may be very small or unconventionally organised.\n"
            )
        if any(p.name == "app_window.py" for p in inventory["files"]):
            out.write(
                "- Desktop UI code may grow quickly; consider splitting layout, state sync, and worker orchestration when it becomes large.\n"
            )
        out.write(
            "- Validate this report manually before making structural refactors; it is heuristic.\n"
        )
