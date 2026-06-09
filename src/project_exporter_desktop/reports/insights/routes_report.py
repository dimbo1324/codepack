from __future__ import annotations

import re
from pathlib import Path

from ...utils.inventory import extension_key, iter_project_dirs, iter_project_files
from ...utils.path_utils import rel_display
from ...utils.time_utils import human_now


def write_routes_and_pages_report(copied_root: Path, output_file: Path) -> None:
    interesting_dirs = {
        "pages": [],
        "routes": [],
        "app": [],
        "features": [],
        "components": [],
        "widgets": [],
        "layouts": [],
    }

    for directory in iter_project_dirs(copied_root):
        lower_parts = [part.lower() for part in directory.relative_to(copied_root).parts]
        for key in interesting_dirs:
            if key in lower_parts:
                interesting_dirs[key].append(directory)
                break

    route_like_files: list[Path] = []
    component_like_files: list[Path] = []
    page_like_files: list[Path] = []

    for path in iter_project_files(copied_root):
        rel_parts = [part.lower() for part in path.relative_to(copied_root).parts]
        suffix = extension_key(path)
        if suffix not in {"ts", "tsx", "js", "jsx", "vue", "svelte", "astro"}:
            continue
        if "routes" in rel_parts or "router" in path.name.lower() or "route" in path.name.lower():
            route_like_files.append(path)
        if "components" in rel_parts or re.match(
            r"^[A-Z].*\.(tsx|jsx|vue|svelte|astro)$", path.name
        ):
            component_like_files.append(path)
        if "pages" in rel_parts or "page" in path.name.lower():
            page_like_files.append(path)

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Frontend Routes / Pages / UI Map ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("This is a heuristic map based on folder and file names.\n")
        out.write("=" * 100 + "\n\n")

        out.write("--- Important UI directories ---\n")
        for key, dirs in interesting_dirs.items():
            out.write(f"\n{key}:\n")
            if dirs:
                for directory in sorted(set(dirs), key=lambda p: str(p).lower())[:100]:
                    out.write(f"- {rel_display(directory, copied_root)}\n")
            else:
                out.write("- not detected\n")

        groups = (
            ("Route-like files", route_like_files),
            ("Page-like files", page_like_files),
            ("Component-like files", component_like_files),
        )
        for title, paths in groups:
            out.write(f"\n--- {title} ---\n")
            if paths:
                for path in sorted(set(paths), key=lambda p: str(p).lower())[:300]:
                    out.write(f"{rel_display(path, copied_root)}\n")
                if len(paths) > 300:
                    out.write(f"... and {len(paths) - 300:,} more\n")
            else:
                out.write("None detected.\n")
