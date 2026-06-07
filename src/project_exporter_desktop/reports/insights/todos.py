from __future__ import annotations

from collections import Counter
from pathlib import Path

from ...constants import TODO_PATTERN
from ...utils.inventory import iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, should_consider_text_file
from ...utils.time_utils import human_now

def write_todo_fixme_report(
    copied_root: Path, output_file: Path, max_bytes_per_file: int
) -> None:
    findings: list[tuple[Path, int, str, str]] = []
    for path in iter_project_files(copied_root):
        if not should_consider_text_file(path):
            continue
        try:
            if path.stat().st_size > max_bytes_per_file:
                continue
        except Exception:
            continue
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue
        for line_number, line in enumerate(text.splitlines(), start=1):
            match = TODO_PATTERN.search(line)
            if match:
                findings.append(
                    (path, line_number, match.group(1).upper(), line.strip())
                )

    by_kind: Counter[str] = Counter(kind for _path, _line, kind, _text in findings)

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== TODO / FIXME / Technical Debt Report ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("=" * 100 + "\n\n")

        out.write("--- Summary ---\n")
        if by_kind:
            for kind, count in by_kind.most_common():
                out.write(f"{kind:<14} {count:>8,}\n")
        else:
            out.write("No TODO/FIXME-like markers detected.\n")

        out.write("\n--- Findings ---\n")
        if findings:
            for path, line_number, kind, line in findings[:1000]:
                out.write(
                    f"{rel_display(path, copied_root)}:{line_number}: [{kind}] {line}\n"
                )
            if len(findings) > 1000:
                out.write(f"... and {len(findings) - 1000:,} more\n")
        else:
            out.write("None.\n")
