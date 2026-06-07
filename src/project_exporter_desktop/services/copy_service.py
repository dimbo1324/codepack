from __future__ import annotations

import os
import shutil
import threading
from collections.abc import Callable
from pathlib import Path

from ..models import CopyStats
from ..utils.path_utils import rel_display, should_ignore_dir

def copy_project(
    source_root: Path,
    destination_root: Path,
    extra_ignored_dirs: frozenset[str] | set[str],
    log: Callable[[str], None],
    cancel: threading.Event,
) -> CopyStats:
    stats = CopyStats()
    destination_root.parent.mkdir(parents=True, exist_ok=True)

    log(f"Создаю копию проекта: {destination_root}")

    for current_dir, dirnames, filenames in os.walk(
        source_root, topdown=True, followlinks=False
    ):
        if cancel.is_set():
            log("Копирование остановлено пользователем.")
            break

        current = Path(current_dir)

        safe_dirnames: list[str] = []
        for dirname in dirnames:
            if should_ignore_dir(dirname, extra_ignored_dirs):
                stats.dirs_skipped += 1
                log(f"Пропущена папка: {rel_display(current / dirname, source_root)}")
                continue

            child = current / dirname
            if child.is_symlink():
                stats.symlinks_skipped += 1
                log(
                    f"Пропущена символическая ссылка на папку: "
                    f"{rel_display(child, source_root)}"
                )
                continue

            safe_dirnames.append(dirname)

        dirnames[:] = safe_dirnames

        relative_dir = current.relative_to(source_root)
        target_dir = destination_root / relative_dir
        try:
            target_dir.mkdir(parents=True, exist_ok=True)
            stats.dirs_created += 1
        except Exception as exc:
            stats.errors += 1
            log(f"Ошибка создания папки {target_dir}: {exc}")
            continue

        for filename in filenames:
            if cancel.is_set():
                break

            src_file = current / filename
            if src_file.is_symlink():
                stats.symlinks_skipped += 1
                log(
                    f"Пропущена символическая ссылка на файл: "
                    f"{rel_display(src_file, source_root)}"
                )
                continue

            dst_file = target_dir / filename

            try:
                shutil.copy2(src_file, dst_file)
                stats.files_copied += 1
                if stats.files_copied % 250 == 0:
                    log(f"Скопировано файлов: {stats.files_copied:,}")
            except Exception as exc:
                stats.errors += 1
                log(f"Ошибка копирования {rel_display(src_file, source_root)}: {exc}")

    return stats
