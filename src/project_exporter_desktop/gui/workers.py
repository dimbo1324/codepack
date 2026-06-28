from __future__ import annotations

import logging
import threading
import traceback
from dataclasses import replace
from pathlib import Path

from PySide6.QtCore import QThread, Signal

_log = logging.getLogger(__name__)

from ..config import Config
from ..constants import TEXT_EXTENSIONS, TEXT_FILENAMES_WITHOUT_EXTENSION
from ..services.analytics_service import analyze_project
from ..services.diff_service import resolve_diff_selection
from ..services.export_ignore import ExportIgnoreRules
from ..services.export_plan import build_export_plan, format_export_plan_for_user
from ..services.exporter import ProjectExporter
from ..services.incremental import IncrementalSelection
from ..services.stack_detector import merged_extra_ignored_dirs


class QtLogQueue:

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
    finished_plan = Signal(object)
    failed = Signal(str)

    def __init__(self, source_root: Path, config: Config, parent=None) -> None:
        super().__init__(parent)
        self.source_root = source_root
        self.config = replace(config)

    def run(self) -> None:
        try:
            ignored = self.config.effective_ignored_dirs() | merged_extra_ignored_dirs(
                self.source_root
            )
            diff_selection = resolve_diff_selection(
                self.source_root,
                self.config.normalized_diff_export_mode(),
                self.config.diff_base_ref,
                self.config.diff_target_ref,
                ignored,
            )
            incremental_selection = IncrementalSelection(enabled=False)
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
                ignored,
                diff_selection,
                incremental_selection,
                export_rules,
            )
            self.finished_plan.emit(plan)
            self.finished_preview.emit(
                format_export_plan_for_user(plan, self.config.effective_zip_part_bytes())
            )
        except Exception:
            tb = traceback.format_exc()
            _log.error("PlanPreviewWorker failed:\n%s", tb)
            self.failed.emit(tb)


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
        parent=None,
        file_overrides: dict[str, bool] | None = None,
    ) -> None:
        super().__init__(parent)
        self.source_root = source_root
        self.config = replace(config)
        self.cancel_event = cancel_event
        self.file_overrides: dict[str, bool] = file_overrides or {}
        self.exporter: ProjectExporter | None = None

    def run(self) -> None:
        try:
            self.exporter = ProjectExporter(
                source_root=self.source_root,
                config=self.config,
                log_queue=QtLogQueue(self),
                cancel_event=self.cancel_event,
                file_overrides=self.file_overrides,
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
            tb = traceback.format_exc()
            _log.error("ExportWorker failed:\n%s", tb)
            self.failed.emit(tb)

    def cancel(self) -> None:
        self.cancel_event.set()


class ClipboardExportWorker(QThread):

    finished = Signal(str, int, str)
    failed = Signal(str)

    _MAX_CLIPBOARD_BYTES = 20 * 1024 * 1024

    def __init__(self, source_root: Path, config: Config, parent=None) -> None:
        super().__init__(parent)
        self.source_root = source_root
        self.config = replace(config)

    def run(self) -> None:
        try:
            from ..utils.token_counter import context_summary_line

            ignored = self.config.effective_ignored_dirs() | merged_extra_ignored_dirs(
                self.source_root
            )
            diff_selection = resolve_diff_selection(
                self.source_root,
                self.config.normalized_diff_export_mode(),
                self.config.diff_base_ref,
                self.config.diff_target_ref,
                ignored,
            )
            incremental_selection = IncrementalSelection(enabled=False)
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
                ignored,
                diff_selection,
                incremental_selection,
                export_rules,
            )

            parts: list[str] = []
            total_bytes = 0

            ctx = (self.config.developer_context or "").strip()
            if ctx:
                header = (
                    "# ══════════════════════════════════════════════════\n"
                    "# ЗАДАЧА / КОНТЕКСТ РАЗРАБОТЧИКА\n"
                    "# ══════════════════════════════════════════════════\n\n"
                    + ctx
                    + "\n\n# ══════════════════════════════════════════════════\n\n"
                )
                parts.append(header)
                total_bytes += len(header.encode("utf-8"))

            for pf in plan.included_files:
                if total_bytes >= self._MAX_CLIPBOARD_BYTES:
                    parts.append(
                        "\n\n[ОБРЕЗАНО: достигнут лимит 20 МБ для буфера обмена]\n"
                    )
                    break
                file_path = self.source_root / pf.relative_path
                if not file_path.is_file():
                    continue
                suffix = file_path.suffix.lstrip(".").casefold()
                name_lower = file_path.name.casefold()
                is_text = (
                    suffix in TEXT_EXTENSIONS
                    or name_lower in TEXT_FILENAMES_WITHOUT_EXTENSION
                )
                if not is_text:
                    continue
                try:
                    content = file_path.read_text(encoding="utf-8", errors="replace")
                except Exception:
                    continue
                block = f"\n\n{'='*60}\nФАЙЛ: {pf.relative_path}\n{'='*60}\n{content}"
                block_bytes = block.encode("utf-8")
                if total_bytes + len(block_bytes) > self._MAX_CLIPBOARD_BYTES:
                    parts.append(
                        "\n\n[ОБРЕЗАНО: достигнут лимит 20 МБ для буфера обмена]\n"
                    )
                    break
                parts.append(block)
                total_bytes += len(block_bytes)

            full_text = "".join(parts)
            summary = context_summary_line(total_bytes)
            self.finished.emit(full_text, total_bytes, summary)
        except Exception:
            tb = traceback.format_exc()
            _log.error("ClipboardExportWorker failed:\n%s", tb)
            self.failed.emit(tb)


class AnalyticsWorker(QThread):
    finished_report = Signal(object)
    failed = Signal(str)

    def __init__(self, source_root: Path, config: Config, parent=None) -> None:
        super().__init__(parent)
        self.source_root = source_root
        self.config = replace(config)

    def run(self) -> None:
        try:
            ignored = self.config.effective_ignored_dirs() | merged_extra_ignored_dirs(
                self.source_root
            )
            self.finished_report.emit(analyze_project(self.source_root, ignored))
        except Exception:
            tb = traceback.format_exc()
            _log.error("AnalyticsWorker failed:\n%s", tb)
            self.failed.emit(tb)