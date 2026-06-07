from __future__ import annotations

import threading
import traceback
from collections.abc import Callable
from pathlib import Path

from ...utils.inventory import collect_basic_inventory
from .ai_context import write_ai_context_pack
from .config_report import write_config_report
from .dependencies import write_dependency_report
from .docker_report import write_docker_report
from .file_statistics import write_file_statistics_report
from .git_deep import write_git_deep_report
from .metrics import write_code_metrics_report
from .routes_report import write_routes_and_pages_report
from .scripts import write_scripts_report
from .security import write_security_scan_report
from .summary import write_project_summary_report
from .todos import write_todo_fixme_report

def write_project_insight_reports(
    copied_root: Path,
    source_root: Path,
    reports_dir: Path,
    max_bytes_per_file: int,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> None:
    reports_dir.mkdir(parents=True, exist_ok=True)
    log(f"Создаю расширенные отчёты: {reports_dir}")

    inventory = collect_basic_inventory(copied_root)

    report_jobs = [
        (
            "01_summary.txt",
            lambda output: write_project_summary_report(
                copied_root, source_root, output, inventory
            ),
        ),
        (
            "02_file_statistics.txt",
            lambda output: write_file_statistics_report(copied_root, output, inventory),
        ),
        (
            "03_dependencies.txt",
            lambda output: write_dependency_report(copied_root, output),
        ),
        (
            "04_scripts.txt",
            lambda output: write_scripts_report(copied_root, output),
        ),
        (
            "05_git_deep.txt",
            lambda output: write_git_deep_report(source_root, output, log, cancel),
        ),
        (
            "06_security_scan.txt",
            lambda output: write_security_scan_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "07_todo_fixme.txt",
            lambda output: write_todo_fixme_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "08_code_metrics.txt",
            lambda output: write_code_metrics_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "09_config.txt",
            lambda output: write_config_report(copied_root, output),
        ),
        (
            "10_docker.txt",
            lambda output: write_docker_report(copied_root, output),
        ),
        (
            "11_routes_and_pages.txt",
            lambda output: write_routes_and_pages_report(copied_root, output),
        ),
        (
            "12_ai_context_pack.md",
            lambda output: write_ai_context_pack(
                copied_root, source_root, output, inventory
            ),
        ),
    ]

    for filename, writer in report_jobs:
        if cancel.is_set():
            log("Создание расширенных отчётов остановлено пользователем.")
            break
        output_file = reports_dir / filename
        log(f"Пишу отчёт: {filename}")
        try:
            writer(output_file)
        except Exception as exc:
            error_file = reports_dir / f"ERROR_{filename}.txt"
            error_file.write_text(
                f"Failed to create {filename}\n\n{type(exc).__name__}: {exc}\n\n{traceback.format_exc()}",
                encoding="utf-8",
                errors="replace",
            )
            log(f"Ошибка создания {filename}: {exc}")

    log("Расширенные аналитические отчёты готовы")
