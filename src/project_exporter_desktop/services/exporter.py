from __future__ import annotations

import shutil
import threading
from pathlib import Path
from queue import Queue

from ..config import Config
from ..constants import IGNORED_DIR_NAMES
from ..models import ArchiveBuildResult, ExportPaths, TextDumpStats
from ..reports.git_report import write_git_report
from ..reports.insights.dashboard import write_html_dashboard
from ..reports.insights.orchestrator import write_project_insight_reports
from ..reports.metadata import write_index_md, write_manifest
from ..reports.structure_report import write_structure_report
from ..reports.text_dump_report import write_text_dump
from ..utils.path_utils import build_export_paths
from ..utils.text_utils import format_bytes
from ..utils.time_utils import human_now
from ..utils.token_counter import estimate_tokens
from .archive_service import build_final_archives
from .copy_service import copy_project
from .diff_service import (
    history_snapshot_payload,
    resolve_diff_selection,
    snapshot_stats,
    write_diff_report,
)
from .export_history import append_export_history
from .export_ignore import ExportIgnoreRules
from .export_plan import build_export_plan, write_export_plan_files
from .incremental import IncrementalSelection
from .progress import ProgressReporter
from .prompt_builder import write_custom_prompt
from .stack_detector import merged_extra_ignored_dirs


class ProjectExporter:
    def __init__(
        self,
        source_root: Path,
        config: Config,
        log_queue: Queue[str],
        cancel_event: threading.Event,
        file_overrides: dict[str, bool] | None = None,
    ):
        self.source_root = source_root
        self.config = config
        self.log_queue = log_queue
        self.cancel_event = cancel_event
        self.file_overrides: dict[str, bool] = file_overrides or {}
        self.archive_result: ArchiveBuildResult | None = None

    def log(self, message: str) -> None:
        self.log_queue.put(message)

    def run(self) -> ExportPaths:
        paths = build_export_paths(self.source_root)
        progress = ProgressReporter(self.log, total_steps=8)
        stack_dirs = merged_extra_ignored_dirs(self.source_root)
        extra_ignored = (self.config.effective_ignored_dirs() | stack_dirs) - {
            name.casefold() for name in IGNORED_DIR_NAMES
        }
        ignored_for_walk = self.config.effective_ignored_dirs() | stack_dirs
        max_bytes = self.config.effective_max_text_file_bytes()
        cancelled = False

        export_rules = ExportIgnoreRules.from_project_and_config(
            paths.source_root,
            excluded_files=self.config.custom_excluded_files,
            excluded_extensions=self.config.custom_excluded_extensions,
            always_include_files=self.config.always_include_files,
            always_include_dirs=self.config.always_include_dirs,
        )

        for rel_path, include in self.file_overrides.items():
            norm = rel_path.replace("\\", "/")
            if include:
                export_rules.add_always_include_file(norm)
            else:
                export_rules.add_file_rule(norm)
        if self.file_overrides:
            self.log(f"Применено пользовательских переопределений: {len(self.file_overrides)}")

        diff_selection = resolve_diff_selection(
            paths.source_root,
            self.config.normalized_diff_export_mode(),
            self.config.diff_base_ref,
            self.config.diff_target_ref,
            ignored_for_walk,
        )
        if diff_selection.warning:
            self.log(diff_selection.warning)

        incremental_selection = IncrementalSelection(enabled=False)

        def combined_selected_paths() -> frozenset[str] | None:
            selected_sets: list[frozenset[str]] = []
            if diff_selection.paths is not None:
                selected_sets.append(diff_selection.paths)
            if incremental_selection.paths is not None:
                selected_sets.append(incremental_selection.paths)
            if not selected_sets:
                return None
            result = set(selected_sets[0])
            for selected in selected_sets[1:]:
                result &= set(selected)
            return frozenset(result)

        include_relative_paths = combined_selected_paths()

        self.log(f"Итоговая папка-stage: {paths.staging_dir}")
        self.log(f"Итоговый ZIP: {paths.final_zip}")
        self.log(f"Safe Export mode: {self.config.normalized_safe_export_mode()}")
        self.log(
            "Лимит текстового файла: "
            + (format_bytes(max_bytes) if max_bytes is not None else "не ограничен")
        )
        if export_rules.source_file:
            self.log(f"Загружен .exportignore: {export_rules.source_file}")
        if extra_ignored:
            self.log("Дополнительные исключаемые папки: " + ", ".join(sorted(extra_ignored)))

        progress.step("Шаг 1/8: построение плана экспорта")
        paths.staging_dir.mkdir(parents=True, exist_ok=True)
        paths.reports_dir.mkdir(parents=True, exist_ok=True)
        paths.insights_dir.mkdir(parents=True, exist_ok=True)
        export_plan = build_export_plan(
            source_root=paths.source_root,
            config=self.config,
            ignored_dirs=ignored_for_walk,
            diff_selection=diff_selection,
            incremental_selection=incremental_selection,
            export_rules=export_rules,
        )
        write_export_plan_files(
            export_plan,
            paths.insights_dir / "28_export_plan.json",
            paths.insights_dir / "28_export_plan.md",
        )
        write_diff_report(paths.insights_dir / "29_export_comparison_report.md", diff_selection)
        self.log(
            "План экспорта: "
            f"included={export_plan.included_count:,}, excluded={export_plan.excluded_count:,}, "
            f"estimated={format_bytes(export_plan.estimated_included_bytes)}"
        )

        progress.step("Шаг 2/8: копирование проекта")
        copy_stats = copy_project(
            source_root=paths.source_root,
            destination_root=paths.project_dir,
            extra_ignored_dirs=ignored_for_walk,
            log=self.log,
            cancel=self.cancel_event,
            safe_export_mode=self.config.normalized_safe_export_mode(),
            include_relative_paths=include_relative_paths,
            export_rules=export_rules,
            progress=progress.update,
        )

        self.log(
            "Копирование завершено: "
            f"files={copy_stats.files_copied:,}, dirs={copy_stats.dirs_created:,}, "
            f"skipped_dirs={copy_stats.dirs_skipped:,}, "
            f"skipped_files={copy_stats.files_skipped:,}, "
            f"safe_skipped={copy_stats.files_skipped_by_safety:,}, "
            f"diff_skipped={copy_stats.files_skipped_by_diff:,}, "
            f"symlinks_skipped={copy_stats.symlinks_skipped:,}, "
            f"errors={copy_stats.errors:,}"
        )

        text_stats = TextDumpStats()
        if self.cancel_event.is_set():
            cancelled = True
        else:
            progress.step("Шаг 3/8: запись структуры проекта")
            write_structure_report(
                paths.project_dir,
                paths.structure_report,
                ignored_for_walk,
                self.log,
                self.cancel_event,
            )

        if not cancelled and not self.cancel_event.is_set():
            progress.step("Шаг 4/8: выполнение Git-команд")
            write_git_report(
                paths.source_root,
                paths.git_report,
                self.log,
                self.cancel_event,
                include_patch=self.config.include_git_patch,
                redact=self.config.redact_secrets,
            )

        if not cancelled and not self.cancel_event.is_set():
            progress.step("Шаг 5/8: сбор текстового содержимого")
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
            ctx = (self.config.developer_context or "").strip()
            if ctx and paths.text_dump.exists():
                header = (
                    "# ══════════════════════════════════════════════════\n"
                    "# ЗАДАЧА / КОНТЕКСТ РАЗРАБОТЧИКА\n"
                    "# ══════════════════════════════════════════════════\n\n"
                    + ctx
                    + "\n\n# ══════════════════════════════════════════════════\n\n"
                ).encode("utf-8")
                try:
                    existing = paths.text_dump.read_bytes()
                    paths.text_dump.write_bytes(header + existing)
                    self.log("Контекст разработчика добавлен в начало текстового дампа.")
                except Exception:
                    pass

        if not cancelled and not self.cancel_event.is_set():
            progress.step("Шаг 6/8: расширенная аналитика проекта")
            write_project_insight_reports(
                copied_root=paths.project_dir,
                source_root=paths.source_root,
                reports_dir=paths.insights_dir,
                max_bytes_per_file=max_bytes,
                log=self.log,
                cancel=self.cancel_event,
                project_profile_file=paths.project_profile_file,
                export_profile=self.config.normalized_export_profile(),
            )
            try:
                write_custom_prompt(
                    paths.insights_dir / "AI_PROMPTS" / "CUSTOM_PROMPT.md",
                    paths.project_name,
                    self.config.prompt_goals,
                )
            except Exception as exc:
                self.log(f"Не удалось создать CUSTOM_PROMPT.md: {exc}")

        cancelled = cancelled or self.cancel_event.is_set()
        progress.step("Шаг 7/8: запись manifest.json и INDEX.md")
        write_manifest(
            paths=paths,
            config=self.config,
            copy_stats=copy_stats,
            text_stats=text_stats,
            extra_ignored_dirs=ignored_for_walk,
            cancelled=cancelled,
            archive_result=None,
            diff_selection=diff_selection,
        )
        write_index_md(paths=paths, config=self.config, extra_ignored_dirs=ignored_for_walk)

        def refresh_bundle_metadata(archive_result: ArchiveBuildResult | None) -> None:
            try:
                write_manifest(
                    paths=paths,
                    config=self.config,
                    copy_stats=copy_stats,
                    text_stats=text_stats,
                    extra_ignored_dirs=ignored_for_walk,
                    cancelled=cancelled,
                    archive_result=archive_result,
                    diff_selection=diff_selection,
                )
            except Exception as exc:
                self.log(f"Не удалось обновить manifest: {exc}")
            try:
                write_html_dashboard(
                    paths.insights_dir, paths.insights_dir / "REPORT_DASHBOARD.html"
                )
            except Exception as exc:
                self.log(f"Не удалось обновить REPORT_DASHBOARD.html: {exc}")

        progress.step("Шаг 8/8: упаковка итогового ZIP / набора ZIP")
        self.archive_result = build_final_archives(
            paths=paths,
            include_project=self.config.include_project_in_zip,
            log=self.log,
            cancel=self.cancel_event,
            part_limit_bytes=self.config.effective_zip_part_bytes(),
            progress=progress.update,
            pre_archive_hook=refresh_bundle_metadata,
        )

        refresh_bundle_metadata(self.archive_result)

        result_path = self.archive_result.primary_result if self.archive_result else paths.final_zip
        successful = not cancelled and not self.cancel_event.is_set() and copy_stats.errors == 0
        snapshot = history_snapshot_payload(paths.source_root, ignored_for_walk) if successful else {}
        token_count = estimate_tokens(paths.text_dump.stat().st_size) if paths.text_dump.exists() else 0
        append_export_history(
            {
                "generated_at": human_now(),
                "project_name": paths.project_name,
                "source_root": str(paths.source_root),
                "profile": self.config.normalized_export_profile(),
                "safe_export_mode": self.config.normalized_safe_export_mode(),
                "diff_export_mode": self.config.normalized_diff_export_mode(),
                "incremental_export_enabled": self.config.incremental_export_enabled,
                "result": str(result_path) if result_path else "",
                "split_archives": bool(self.archive_result and self.archive_result.split),
                "archives": [
                    str(path)
                    for path in (self.archive_result.archives if self.archive_result else [])
                ],
                "copy_stats": {
                    "files_copied": copy_stats.files_copied,
                    "files_skipped_by_safety": copy_stats.files_skipped_by_safety,
                    "errors": copy_stats.errors,
                },
                "tokens": token_count,
                "snapshot": snapshot,
                "snapshot_stats": snapshot_stats(snapshot),
                "cancelled": cancelled,
            }
        )

        if not self.config.keep_staging_folder:
            self.log("Удаляю промежуточную папку (staging)")
            try:
                shutil.rmtree(paths.staging_dir, ignore_errors=False)
            except Exception as exc:
                self.log(f"Не удалось удалить staging-папку: {exc}")

        if cancelled:
            self.log(
                "Готово (с прерыванием). Итоговый результат содержит то, что успело собраться."
            )
        else:
            progress.done("Готово")
            self.log(f"Готово. Итоговый результат: {result_path}")

        return paths