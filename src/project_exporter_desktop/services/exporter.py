from __future__ import annotations

import shutil
import threading
from pathlib import Path
from queue import Queue

from ..config import Config
from ..constants import IGNORED_DIR_NAMES
from ..models import ExportPaths, TextDumpStats
from ..reports.git_report import write_git_report
from ..reports.insights.orchestrator import write_project_insight_reports
from ..reports.metadata import write_index_md, write_manifest
from ..reports.structure_report import write_structure_report
from ..reports.text_dump_report import write_text_dump
from ..utils.path_utils import build_export_paths
from .archive_service import build_final_zip
from .copy_service import copy_project

class ProjectExporter:
    def __init__(
        self,
        source_root: Path,
        config: Config,
        log_queue: Queue[str],
        cancel_event: threading.Event,
    ):
        self.source_root = source_root
        self.config = config
        self.log_queue = log_queue
        self.cancel_event = cancel_event

    def log(self, message: str) -> None:
        self.log_queue.put(message)

    def run(self) -> ExportPaths:
        paths = build_export_paths(self.source_root)
        extra_ignored = self.config.effective_ignored_dirs() - IGNORED_DIR_NAMES
        ignored_for_walk = self.config.effective_ignored_dirs()
        max_bytes = max(1, self.config.max_text_file_mb) * 1024 * 1024
        cancelled = False

        self.log(f"Итоговая папка-stage: {paths.staging_dir}")
        self.log(f"Итоговый ZIP: {paths.final_zip}")
        if extra_ignored:
            self.log(
                "Дополнительные исключаемые папки: " + ", ".join(sorted(extra_ignored))
            )

        # --- Step 1/7: copy ---------------------------------------------------
        self.log("Шаг 1/7: копирование проекта")
        paths.staging_dir.mkdir(parents=True, exist_ok=True)
        paths.reports_dir.mkdir(parents=True, exist_ok=True)
        paths.insights_dir.mkdir(parents=True, exist_ok=True)

        copy_stats = copy_project(
            source_root=paths.source_root,
            destination_root=paths.project_dir,
            extra_ignored_dirs=ignored_for_walk,
            log=self.log,
            cancel=self.cancel_event,
        )

        self.log(
            "Копирование завершено: "
            f"files={copy_stats.files_copied:,}, dirs={copy_stats.dirs_created:,}, "
            f"skipped_dirs={copy_stats.dirs_skipped:,}, "
            f"symlinks_skipped={copy_stats.symlinks_skipped:,}, "
            f"errors={copy_stats.errors:,}"
        )

        text_stats = TextDumpStats()

        if self.cancel_event.is_set():
            cancelled = True
        else:
            # --- Step 2/7: directory structure --------------------------------
            self.log("Шаг 2/7: запись относительной структуры")
            write_structure_report(
                paths.project_dir,
                paths.structure_report,
                ignored_for_walk,
                self.log,
                self.cancel_event,
            )

        if not cancelled and not self.cancel_event.is_set():
            # --- Step 3/7: basic Git report -----------------------------------
            self.log("Шаг 3/7: выполнение Git-команд")
            write_git_report(
                paths.source_root, paths.git_report, self.log, self.cancel_event
            )

        if not cancelled and not self.cancel_event.is_set():
            # --- Step 4/7: text dump ------------------------------------------
            self.log("Шаг 4/7: сбор текстового содержимого")
            text_stats = write_text_dump(
                root=paths.project_dir,
                output_file=paths.text_dump,
                max_bytes_per_file=max_bytes,
                redact=self.config.redact_secrets,
                log=self.log,
                cancel=self.cancel_event,
            )
            self.log(
                "Текстовый отчёт: "
                f"scanned={text_stats.scanned:,}, written={text_stats.written:,}, "
                f"binary={text_stats.skipped_binary:,}, "
                f"large={text_stats.skipped_large:,}, "
                f"not_text={text_stats.skipped_not_text:,}, "
                f"decode_errors={text_stats.skipped_decode:,}"
            )

        if not cancelled and not self.cancel_event.is_set():
            # --- Step 5/7: project insights -----------------------------------
            self.log("Шаг 5/7: расширенная аналитика проекта")
            write_project_insight_reports(
                copied_root=paths.project_dir,
                source_root=paths.source_root,
                reports_dir=paths.insights_dir,
                max_bytes_per_file=max_bytes,
                log=self.log,
                cancel=self.cancel_event,
            )

        # --- Step 6/7: manifest + INDEX (always written) ----------------------
        # Even if cancelled, we still record what happened.
        cancelled = cancelled or self.cancel_event.is_set()
        self.log("Шаг 6/7: запись manifest.json и INDEX.md")
        write_manifest(
            paths=paths,
            config=self.config,
            copy_stats=copy_stats,
            text_stats=text_stats,
            extra_ignored_dirs=ignored_for_walk,
            cancelled=cancelled,
        )
        write_index_md(
            paths=paths,
            config=self.config,
            extra_ignored_dirs=ignored_for_walk,
        )

        # --- Step 7/7: final zip + optional cleanup ---------------------------
        self.log("Шаг 7/7: упаковка итогового ZIP")
        build_final_zip(
            paths=paths,
            include_project=self.config.include_project_in_zip,
            log=self.log,
            cancel=self.cancel_event,
        )

        if not self.config.keep_staging_folder:
            self.log("Удаляю промежуточную папку (staging)")
            try:
                shutil.rmtree(paths.staging_dir, ignore_errors=False)
            except Exception as exc:
                self.log(f"Не удалось удалить staging-папку: {exc}")

        if cancelled:
            self.log(
                "Готово (с прерыванием). Итоговый ZIP содержит то, что успело собраться."
            )
        else:
            self.log("Готово. Итоговый ZIP лежит на Desktop.")

        return paths
