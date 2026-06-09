from __future__ import annotations

import threading
import traceback
from dataclasses import replace
from pathlib import Path

from PySide6.QtCore import QThread, Signal

from ..config import Config
from ..services.export_ignore import ExportIgnoreRules
from ..services.export_plan import build_export_plan, format_export_plan_for_user
from ..services.exporter import ProjectExporter
from ..services.git_diff import resolve_diff_selection
from ..services.incremental import resolve_incremental_selection


class QtLogQueue:
    """Queue-compatible adapter used by core services inside a Qt worker thread."""

    def __init__(self, worker: ExportWorker) -> None:
        self.worker = worker

    def put(self, message: str) -> None:
        if message.startswith("PROGRESS\t"):
            parts = message.split("\t", 3)
            try:
                percent = int(parts[1])
            except Exception:
                percent = 0
            stage = parts[2] if len(parts) > 2 else ""
            current = parts[3] if len(parts) > 3 else ""
            self.worker.progress_changed.emit(percent, stage, current)
            return
        self.worker.log_message.emit(message)


class PlanPreviewWorker(QThread):
    finished_preview = Signal(str)
    failed = Signal(str)

    def __init__(self, source_root: Path, config: Config, parent=None) -> None:  # noqa: ANN001
        super().__init__(parent)
        self.source_root = source_root
        self.config = replace(config)

    def run(self) -> None:
        try:
            diff_selection = resolve_diff_selection(
                self.source_root,
                self.config.normalized_diff_export_mode(),
                self.config.diff_base_ref,
                self.config.diff_target_ref,
            )
            incremental_selection = resolve_incremental_selection(
                self.source_root,
                self.config.effective_ignored_dirs(),
                self.config.incremental_export_enabled,
            )
            export_rules = ExportIgnoreRules.from_project_and_config(
                self.source_root,
                excluded_files=self.config.custom_excluded_files,
                excluded_extensions=self.config.custom_excluded_extensions,
                always_include_files=self.config.always_include_files,
                always_include_dirs=self.config.always_include_dirs,
            )
            plan = build_export_plan(
                self.source_root,
                self.config,
                self.config.effective_ignored_dirs(),
                diff_selection,
                incremental_selection,
                export_rules,
            )
            self.finished_preview.emit(
                format_export_plan_for_user(plan, self.config.effective_zip_part_bytes())
            )
        except Exception:
            self.failed.emit(traceback.format_exc())


class ExportWorker(QThread):
    log_message = Signal(str)
    progress_changed = Signal(int, str, str)
    finished_success = Signal(object)
    failed = Signal(str)

    def __init__(
        self,
        source_root: Path,
        config: Config,
        cancel_event: threading.Event,
        parent=None,  # noqa: ANN001
    ) -> None:
        super().__init__(parent)
        self.source_root = source_root
        self.config = replace(config)
        self.cancel_event = cancel_event
        self.exporter: ProjectExporter | None = None

    def run(self) -> None:
        try:
            self.exporter = ProjectExporter(
                source_root=self.source_root,
                config=self.config,
                log_queue=QtLogQueue(self),
                cancel_event=self.cancel_event,
            )
            paths = self.exporter.run()
            archive_result = self.exporter.archive_result
            result_path = None
            if archive_result and archive_result.primary_result:
                result_path = archive_result.primary_result
            elif paths.final_zip.exists():
                result_path = paths.final_zip
            elif paths.archive_set_dir.exists():
                result_path = paths.archive_set_dir
            elif paths.staging_dir.exists():
                result_path = paths.staging_dir
            self.finished_success.emit(
                {
                    "paths": paths,
                    "archive_result": archive_result,
                    "result_path": result_path,
                    "cancelled": self.cancel_event.is_set(),
                }
            )
        except Exception:
            self.failed.emit(traceback.format_exc())

    def cancel(self) -> None:
        self.cancel_event.set()
