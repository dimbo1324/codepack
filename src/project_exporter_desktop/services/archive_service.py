from __future__ import annotations

import threading
import zipfile
from collections.abc import Callable
from pathlib import Path

from ..models import ExportPaths

def build_final_zip(
    paths: ExportPaths,
    include_project: bool,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> int:
    """Pack the entire staging directory into a single ZIP on the Desktop.

    The archive layout matches the staging directory: INDEX.md, manifest.json,
    {project}/ (optional), and reports/. When ``include_project`` is False,
    the {project}/ subtree is skipped — the archive then contains only the
    reports bundle, which is useful when only an overview is needed.
    """
    log(f"Создаю итоговый ZIP-архив: {paths.final_zip.name}")
    file_count = 0
    skipped_project_files = 0

    project_dir_resolved = paths.project_dir.resolve()
    staging_resolved = paths.staging_dir.resolve()

    with zipfile.ZipFile(
        paths.final_zip, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6
    ) as archive:
        for file_path in paths.staging_dir.rglob("*"):
            if cancel.is_set():
                log("Создание архива остановлено пользователем.")
                break
            if not file_path.is_file():
                continue

            resolved = file_path.resolve()
            if not include_project:
                try:
                    resolved.relative_to(project_dir_resolved)
                    skipped_project_files += 1
                    continue
                except ValueError:
                    pass  # Outside the project subtree — keep it.

            try:
                arcname = resolved.relative_to(staging_resolved)
            except ValueError:
                # Should not happen, but stay defensive.
                arcname = Path(file_path.name)

            archive.write(file_path, arcname)
            file_count += 1
            if file_count % 500 == 0:
                log(f"Добавлено в ZIP: {file_count:,} файлов")

    if skipped_project_files:
        log(
            f"Копия проекта исключена из ZIP по настройке "
            f"({skipped_project_files:,} файлов)"
        )
    log(f"ZIP готов: {file_count:,} файлов → {paths.final_zip}")
    return file_count
