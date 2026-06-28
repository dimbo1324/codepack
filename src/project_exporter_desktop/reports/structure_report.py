from __future__ import annotations

import os
import threading
from collections.abc import Callable
from pathlib import Path

from ..constants import IGNORED_DIR_NAMES
from ..utils.path_utils import ps_date, ps_mode, rel_display, should_ignore_dir
from ..utils.time_utils import human_now


def write_structure_report(
    root: Path,
    output_file: Path,
    extra_ignored_dirs: frozenset[str] | set[str],
    log: Callable[[str], None],
    cancel: threading.Event,
) -> int:
    log(f"Формирую отчёт структуры: {output_file.name}")
    groups_written = 0

    ignored_display = ", ".join(sorted(IGNORED_DIR_NAMES | set(extra_ignored_dirs)))

    with output_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write("=== Relative Project Structure ===\n")
        out.write(f"Project copy root name: {root.name}\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(f"Ignored directories: {ignored_display}\n")
        out.write("=" * 100 + "\n\n")

        for current_dir, dirnames, filenames in os.walk(root, topdown=True, followlinks=False):
            if cancel.is_set():
                break

            current = Path(current_dir)
            dirnames[:] = [d for d in dirnames if not should_ignore_dir(d, extra_ignored_dirs)]

            entries: list[Path] = []
            for dirname in sorted(dirnames, key=str.lower):
                entries.append(current / dirname)
            for filename in sorted(filenames, key=str.lower):
                entries.append(current / filename)

            if not entries:
                continue

            out.write(f"    Directory: {rel_display(current, root)}\n\n")
            out.write(f"{'Mode':<20} {'LastWriteTime':<20} {'Length':>12} Name\n")
            out.write(f"{'----':<20} {'-------------':<20} {'------':>12} ----\n")

            for entry in entries:
                try:
                    stat = entry.stat()
                    length = "" if entry.is_dir() else str(stat.st_size)
                    out.write(
                        f"{ps_mode(entry):<20} {ps_date(stat.st_mtime):<20} "
                        f"{length:>12} {entry.name}\n"
                    )
                except Exception as exc:
                    out.write(f"{'ERROR':<20} {'':<20} {'':>12} {entry.name} ({exc})\n")

            out.write("\n\n")
            groups_written += 1

    log(f"Отчёт структуры готов: {groups_written:,} директорий")
    return groups_written