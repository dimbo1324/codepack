# Insight report generator: reads the copied project and writes one focused analysis artifact into reports/insights.

from __future__ import annotations

from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from ...constants import SOURCE_CODE_EXTENSIONS
from ...utils.inventory import extension_key, iter_project_files, write_key_value_lines
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely
from ...utils.time_utils import human_now


def comment_like_line(stripped: str, suffix: str) -> bool:
    if not stripped:
        return False
    common = ("//", "/*", "*", "#", "<!--", "--")
    if stripped.startswith(common):
        return True
    if suffix in {"sql"} and stripped.startswith("--"):
        return True
    return False


def write_code_metrics_report(
    copied_root: Path, output_file: Path, max_bytes_per_file: int | None
) -> None:
    per_file: list[dict[str, Any]] = []
    totals = Counter()

    for path in iter_project_files(copied_root):
        suffix = extension_key(path)
        if suffix not in SOURCE_CODE_EXTENSIONS:
            continue
        try:
            if max_bytes_per_file is not None and path.stat().st_size > max_bytes_per_file:
                continue
        except Exception:
            continue
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue

        lines = text.splitlines()
        blank = 0
        comments = 0
        for line in lines:
            stripped = line.strip()
            if not stripped:
                blank += 1
            elif comment_like_line(stripped, suffix):
                comments += 1
        code = max(0, len(lines) - blank - comments)
        item = {
            "path": path,
            "lines": len(lines),
            "blank": blank,
            "comments": comments,
            "code": code,
            "suffix": suffix,
        }
        per_file.append(item)
        totals["lines"] += len(lines)
        totals["blank"] += blank
        totals["comments"] += comments
        totals["code"] += code

    largest_by_lines = sorted(per_file, key=lambda item: item["lines"], reverse=True)[:50]
    over_500 = [item for item in per_file if item["lines"] >= 500]
    over_1000 = [item for item in per_file if item["lines"] >= 1000]
    by_ext: dict[str, Counter[str]] = defaultdict(Counter)
    for item in per_file:
        ext = item["suffix"]
        by_ext[ext]["files"] += 1
        by_ext[ext]["lines"] += item["lines"]
        by_ext[ext]["code"] += item["code"]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Code Metrics ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("Line classification is heuristic.\n")
        out.write("=" * 100 + "\n\n")
        write_key_value_lines(
            out,
            {
                "Source files analysed": f"{len(per_file):,}",
                "Total lines": f"{totals['lines']:,}",
                "Code lines": f"{totals['code']:,}",
                "Comment-like lines": f"{totals['comments']:,}",
                "Blank lines": f"{totals['blank']:,}",
                "Files >= 500 lines": f"{len(over_500):,}",
                "Files >= 1000 lines": f"{len(over_1000):,}",
            },
        )

        out.write("\n--- Metrics by extension ---\n")
        out.write(f"{'Ext':<16} {'Files':>8} {'Lines':>12} {'Code':>12}\n")
        for ext, counter in sorted(by_ext.items(), key=lambda item: item[1]["lines"], reverse=True):
            out.write(
                f"{ext:<16} {counter['files']:>8,} {counter['lines']:>12,} {counter['code']:>12,}\n"
            )

        out.write("\n--- Largest files by line count ---\n")
        for item in largest_by_lines:
            out.write(
                f"{item['lines']:>8,} lines  code={item['code']:>8,}  {rel_display(item['path'], copied_root)}\n"
            )

        out.write("\n--- Files >= 500 lines ---\n")
        if over_500:
            for item in sorted(over_500, key=lambda x: x["lines"], reverse=True):
                out.write(f"{item['lines']:>8,} lines  {rel_display(item['path'], copied_root)}\n")
        else:
            out.write("None detected.\n")
