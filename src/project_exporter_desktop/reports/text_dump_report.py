from __future__ import annotations

import threading
from collections.abc import Callable
from datetime import datetime
from pathlib import Path

from ..models import TextDumpStats
from ..utils.path_utils import rel_display
from ..utils.text_utils import read_text_safely, redact_secrets, should_consider_text_file
from ..utils.time_utils import human_now


def write_text_dump(
    root: Path,
    output_file: Path,
    max_bytes_per_file: int | None,
    redact: bool,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> TextDumpStats:
    log(f"Собираю текстовое содержимое файлов: {output_file.name}")

    stats = TextDumpStats()
    files = sorted((p for p in root.rglob("*") if p.is_file()), key=lambda p: str(p).lower())

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Text Files Dump ===\n")
        out.write(f"Project copy root name: {root.name}\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(
            f"Max bytes per file: {max_bytes_per_file:,}\n"
            if max_bytes_per_file is not None
            else "Max bytes per file: unlimited\n"
        )
        out.write(f"Secrets redaction: {'enabled' if redact else 'disabled'}\n")
        out.write("Only readable text-like files are included.\n")
        out.write("=" * 100 + "\n\n")

        for path in files:
            if cancel.is_set():
                log("Сбор текстовых файлов остановлен пользователем.")
                break

            stats.scanned += 1

            if not should_consider_text_file(path):
                stats.skipped_not_text += 1
                continue

            try:
                size = path.stat().st_size
            except Exception:
                stats.skipped_decode += 1
                continue

            if max_bytes_per_file is not None and size > max_bytes_per_file:
                stats.skipped_large += 1
                continue

            text, info = read_text_safely(path, max_bytes=max_bytes_per_file)
            if text is None:
                if info == "binary-detected":
                    stats.skipped_binary += 1
                elif info.startswith("too-large"):
                    stats.skipped_large += 1
                else:
                    stats.skipped_decode += 1
                continue

            if redact:
                text = redact_secrets(text)

            stat = path.stat()
            out.write("\n" + "=" * 120 + "\n")
            out.write(f"File: {rel_display(path, root)}\n")
            out.write(f"Name: {path.name}\n")
            out.write(f"Size: {stat.st_size:,} bytes\n")
            out.write(
                f"Modified: "
                f"{datetime.fromtimestamp(stat.st_mtime).isoformat(sep=' ', timespec='seconds')}\n"
            )
            out.write(f"Encoding: {info}\n")
            out.write("=" * 120 + "\n\n")
            out.write(text)
            if not text.endswith("\n"):
                out.write("\n")

            stats.written += 1
            if stats.written % 50 == 0:
                log(f"Записано текстовых файлов: {stats.written:,}")

    log(f"Текстовый дамп готов: {stats.written:,} файлов")
    return stats