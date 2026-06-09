from __future__ import annotations

import os
import shutil
import threading
from collections.abc import Callable
from pathlib import Path

from ..models import CopyStats
from ..utils.path_utils import rel_display, should_ignore_dir
from .export_ignore import ExportIgnoreRules
from .export_policy import should_skip_file_for_safety


def copy_project(
    source_root: Path,
    destination_root: Path,
    extra_ignored_dirs: frozenset[str] | set[str],
    log: Callable[[str], None],
    cancel: threading.Event,
    safe_export_mode: str = "safe",
    include_relative_paths: frozenset[str] | None = None,
    export_rules: ExportIgnoreRules | None = None,
    progress: Callable[[int, str, str], None] | None = None,
) -> CopyStats:
    stats = CopyStats()
    destination_root.parent.mkdir(parents=True, exist_ok=True)

    log(f"Создаю копию проекта: {destination_root}")
    if include_relative_paths is not None:
        log(f"Diff export: выбрано Git-путей: {len(include_relative_paths):,}")

    for current_dir, dirnames, filenames in os.walk(source_root, topdown=True, followlinks=False):
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
                log(f"Пропущена символическая ссылка на папку: {rel_display(child, source_root)}")
                continue

            if include_relative_paths is not None:
                try:
                    rel_dir = str(child.relative_to(source_root)).replace("/", "\\")
                except ValueError:
                    rel_dir = ""
                prefix = rel_dir + "\\"
                if not any(
                    path == rel_dir or path.startswith(prefix) for path in include_relative_paths
                ):
                    continue

            if export_rules is not None:
                try:
                    relative_dir = child.relative_to(source_root)
                except ValueError:
                    relative_dir = Path(dirname)
                skip_by_rule, reason = export_rules.should_skip_dir(relative_dir)
                if skip_by_rule:
                    stats.dirs_skipped += 1
                    log(
                        f"Export rules: пропущена папка {rel_display(child, source_root)} ({reason})"
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
                log(f"Пропущена символическая ссылка на файл: {rel_display(src_file, source_root)}")
                continue

            try:
                relative_file = src_file.relative_to(source_root)
            except ValueError:
                stats.files_skipped += 1
                continue
            rel_key = str(relative_file).replace("/", "\\")

            if include_relative_paths is not None and rel_key not in include_relative_paths:
                stats.files_skipped_by_diff += 1
                continue

            if export_rules is not None:
                skip_by_rule, reason = export_rules.should_skip_file(relative_file)
                if skip_by_rule:
                    stats.files_skipped += 1
                    log(
                        f"Export rules: пропущен файл {rel_display(src_file, source_root)} ({reason})"
                    )
                    continue

            safety = should_skip_file_for_safety(relative_file, safe_export_mode)
            if safety.skip:
                stats.files_skipped += 1
                stats.files_skipped_by_safety += 1
                log(
                    f"Safe Export: пропущен файл {rel_display(src_file, source_root)} "
                    f"({safety.reason})"
                )
                continue

            dst_file = target_dir / filename

            try:
                shutil.copy2(src_file, dst_file)
                stats.files_copied += 1
                if stats.files_copied % 250 == 0:
                    log(f"Скопировано файлов: {stats.files_copied:,}")
                    if progress is not None:
                        progress(20, "Copying files", rel_display(src_file, source_root))
            except Exception as exc:
                stats.errors += 1
                log(f"Ошибка копирования {rel_display(src_file, source_root)}: {exc}")

    return stats
