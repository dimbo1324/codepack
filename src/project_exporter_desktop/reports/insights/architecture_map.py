"""Architecture map report: classifies every file into a coarse architectural layer.

Produces 24_architecture_map.md with a per-layer file listing and a Mermaid flowchart
showing the canonical dependency direction between layers.
"""

from __future__ import annotations

from collections import defaultdict
from pathlib import Path
from typing import Any

from ...utils.inventory import extension_key
from ...utils.time_utils import human_now


def _layer_for_path(path: Path) -> str:
    # Rules are evaluated top-to-bottom; more specific checks (entrypoints) come first.
    parts = {part.casefold() for part in path.parts}
    name = path.name.casefold()
    suffix = extension_key(path)
    if name in {"main.py", "__main__.py", "app.py", "manage.py"} or name.startswith("vite.config"):
        return "entrypoints"
    if (
        "ui" in parts
        or "components" in parts
        or "pages" in parts
        or suffix in {"html", "css", "scss", "tsx", "jsx", "vue", "svelte"}
    ):
        return "interface"
    if "services" in parts or "service" in parts or "usecases" in parts:
        return "business_services"
    if "reports" in parts or "exporters" in parts:
        return "report_generation"
    if "models" in parts or "schemas" in parts or "entities" in parts:
        return "data_models"
    if "utils" in parts or "helpers" in parts or "lib" in parts:
        return "utilities"
    if "tests" in parts or "test" in name:
        return "tests"
    if suffix in {"json", "yaml", "yml", "toml", "ini", "cfg", "conf"}:
        return "configuration"
    if suffix in {"md", "rst", "adoc", "txt"}:
        return "documentation"
    return "other"


def write_architecture_map_report(
    copied_root: Path, output_file: Path, inventory: dict[str, Any]
) -> None:
    layers: dict[str, list[Path]] = defaultdict(list)
    for path in inventory.get("files", []):
        layers[_layer_for_path(path)].append(path)

    with output_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write("# Architecture Map\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write("## Layer summary\n\n")
        for layer in sorted(layers):
            out.write(f"### {layer}\n\n")
            for path in sorted(layers[layer], key=lambda p: str(p).casefold())[:40]:
                out.write(f"- `{path.relative_to(copied_root)}`\n")
            if len(layers[layer]) > 40:
                out.write(f"- ... and {len(layers[layer]) - 40:,} more\n")
            out.write("\n")

        out.write("## Suggested dependency direction\n\n")
        out.write("```mermaid\n")
        out.write("flowchart TD\n")
        out.write("  entrypoints[Entrypoints] --> interface[Interface / CLI / UI]\n")
        out.write("  interface --> business_services[Business services]\n")
        out.write("  business_services --> report_generation[Report generation]\n")
        out.write("  business_services --> data_models[Data models]\n")
        out.write("  report_generation --> utilities[Utilities]\n")
        out.write("  data_models --> utilities\n")
        out.write("  tests[Tests] --> business_services\n")
        out.write("  tests --> report_generation\n")
        out.write("```\n\n")
        out.write("## Review notes\n\n")
        out.write("- Keep UI modules thin; long-running work should stay in services.\n")
        out.write("- Report modules should not depend on Tkinter.\n")
        out.write("- Utility modules should stay dependency-light and deterministic.\n")