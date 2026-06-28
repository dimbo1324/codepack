from __future__ import annotations

from collections import Counter
from pathlib import Path
from typing import Any

from ...utils.inventory import is_non_ascii, path_depth
from ...utils.path_utils import rel_display
from ...utils.text_utils import format_bytes
from ...utils.time_utils import human_now


def write_file_statistics_report(
    copied_root: Path, output_file: Path, inventory: dict[str, Any]
) -> None:
    files: list[Path] = inventory["files"]
    sizes: list[tuple[Path, int]] = inventory["sizes"]
    by_ext_count: Counter[str] = inventory["by_ext_count"]
    by_ext_size: Counter[str] = inventory["by_ext_size"]

    empty_files = [(p, s) for p, s in sizes if s == 0]
    spaced = [p for p in files if " " in p.name]
    non_ascii = [p for p in files if is_non_ascii(str(p.relative_to(copied_root)))]
    deepest = sorted(files, key=lambda p: path_depth(p, copied_root), reverse=True)[:25]
    largest = sorted(sizes, key=lambda item: item[1], reverse=True)[:25]
    long_paths = [p for p in files if len(str(p)) >= 240]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== File Statistics ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("=" * 100 + "\n\n")

        out.write("--- Files by extension ---\n")
        out.write(f"{'Extension':<20} {'Files':>10} {'Total size':>14}\n")
        out.write(f"{'-' * 20} {'-' * 10} {'-' * 14}\n")
        for ext, count in by_ext_count.most_common():
            out.write(f"{ext:<20} {count:>10,} {format_bytes(by_ext_size[ext]):>14}\n")

        out.write("\n--- Top 25 largest files ---\n")
        for path, size in largest:
            out.write(f"{format_bytes(size):>12}  {rel_display(path, copied_root)}\n")

        out.write("\n--- Top 25 deepest files ---\n")
        for path in deepest:
            out.write(
                f"depth={path_depth(path, copied_root):>2}  {rel_display(path, copied_root)}\n"
            )

        out.write("\n--- Empty files ---\n")
        if empty_files:
            for path, _size in empty_files[:100]:
                out.write(f"{rel_display(path, copied_root)}\n")
            if len(empty_files) > 100:
                out.write(f"... and {len(empty_files) - 100:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Files with spaces in names ---\n")
        if spaced:
            for path in spaced[:100]:
                out.write(f"{rel_display(path, copied_root)}\n")
            if len(spaced) > 100:
                out.write(f"... and {len(spaced) - 100:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Files with non-ASCII paths ---\n")
        if non_ascii:
            for path in non_ascii[:100]:
                out.write(f"{rel_display(path, copied_root)}\n")
            if len(non_ascii) > 100:
                out.write(f"... and {len(non_ascii) - 100:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Potential Windows long-path risks >= 240 characters ---\n")
        if long_paths:
            for path in long_paths[:100]:
                out.write(f"{len(str(path)):>4} chars  {rel_display(path, copied_root)}\n")
            if len(long_paths) > 100:
                out.write(f"... and {len(long_paths) - 100:,} more\n")
        else:
            out.write("None detected.\n")