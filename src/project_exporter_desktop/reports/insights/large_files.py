from __future__ import annotations

from collections import Counter
from pathlib import Path
from typing import Any

from ...utils.text_utils import format_bytes
from ...utils.time_utils import human_now


def write_large_files_report(copied_root: Path, output_file: Path, inventory: dict[str, Any]) -> None:
    sizes: list[tuple[Path, int]] = list(inventory.get("sizes", []))
    folder_sizes: Counter[Path] = Counter()
    for path, size in sizes:
        try:
            rel = path.relative_to(copied_root)
        except ValueError:
            continue
        parts = rel.parts[:-1]
        current = Path()
        for part in parts:
            current = current / part
            folder_sizes[current] += size

    with output_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write("# Large File Inspector\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write("## Top 30 largest files\n\n")
        if not sizes:
            out.write("No files detected.\n")
        for path, size in sorted(sizes, key=lambda item: item[1], reverse=True)[:30]:
            out.write(f"- `{path.relative_to(copied_root)}` — {format_bytes(size)}\n")

        out.write("\n## Top 20 largest folders\n\n")
        if not folder_sizes:
            out.write("No folders detected.\n")
        for folder, size in folder_sizes.most_common(20):
            out.write(f"- `{folder}` — {format_bytes(size)}\n")

        out.write("\n## Archive-size guidance\n\n")
        out.write("- Safe Export mode removes common local databases, archives and credentials before packaging.\n")
        out.write("- If the single ZIP exceeds the configured limit, archives are split by logical groups.\n")
        out.write("- Very large individual files should normally be excluded from AI/code-review exports.\n")
