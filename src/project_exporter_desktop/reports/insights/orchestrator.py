from __future__ import annotations

import shutil
import threading
import traceback
from collections.abc import Callable
from pathlib import Path

from ...constants import DEFAULT_EXPORT_PROFILE, EXPORT_PROFILES
from ...utils.inventory import collect_basic_inventory
from .ai_context import write_ai_context_pack
from .ai_context_folder import write_ai_context_folder
from .ai_prompts import write_ai_prompt_files
from .api_surface import write_api_surface_report
from .architecture import write_architecture_report
from .architecture_map import write_architecture_map_report
from .code_quality import write_code_quality_report
from .config_report import write_config_report
from .dependencies import write_dependency_report
from .dependency_intelligence import write_dependency_intelligence_report
from .dependency_graph import write_dependency_graph_reports
from .docker_report import write_docker_report
from .file_statistics import write_file_statistics_report
from .frontend_backend import write_backend_report, write_frontend_report
from .git_deep import write_git_deep_report
from .git_timeline import write_git_timeline_report
from .health_score import write_project_health_report
from .key_files import write_key_files_report
from .large_files import write_large_files_report
from .metrics import write_code_metrics_report
from .project_profile import write_project_profile_json
from .refactoring import write_refactoring_opportunities_report
from .routes_report import write_routes_and_pages_report
from .runbook import write_runbook_report
from .scripts import write_scripts_report
from .security import write_security_scan_report
from .summary import write_project_summary_report
from .todos import write_todo_fixme_report

ReportJob = tuple[str, set[str], Callable[[Path], None]]


def _normalise_profile(profile: str) -> str:
    return profile if profile in EXPORT_PROFILES else DEFAULT_EXPORT_PROFILE


def _should_run(profile: str, job_profiles: set[str]) -> bool:
    if profile == "full":
        return True
    return profile in job_profiles


def write_project_insight_reports(
    copied_root: Path,
    source_root: Path,
    reports_dir: Path,
    max_bytes_per_file: int | None,
    log: Callable[[str], None],
    cancel: threading.Event,
    project_profile_file: Path | None = None,
    export_profile: str = DEFAULT_EXPORT_PROFILE,
) -> None:
    reports_dir.mkdir(parents=True, exist_ok=True)
    profile = _normalise_profile(export_profile)
    log(f"Создаю расширенные отчёты: {reports_dir}")
    log(f"Профиль экспорта: {profile}")

    inventory = collect_basic_inventory(copied_root)

    all_profiles = {"quick", "full", "ai_review", "security", "minimal"}
    ai_security = {"ai_review", "security"}
    ai_quick_minimal = {"quick", "ai_review", "minimal"}
    ai_only = {"ai_review"}
    security_only = {"security"}

    def project_profile_job(output: Path) -> None:
        write_project_profile_json(copied_root, source_root, output, inventory)
        if project_profile_file is not None:
            shutil.copyfile(output, project_profile_file)

    report_jobs: list[ReportJob] = [
        (
            "00_project_profile.json",
            all_profiles,
            project_profile_job,
        ),
        (
            "01_summary.txt",
            all_profiles,
            lambda output: write_project_summary_report(
                copied_root, source_root, output, inventory
            ),
        ),
        (
            "02_file_statistics.txt",
            {"quick", "full", "ai_review", "security"},
            lambda output: write_file_statistics_report(copied_root, output, inventory),
        ),
        (
            "03_dependencies.txt",
            {"quick", "full", "ai_review", "security"},
            lambda output: write_dependency_report(copied_root, output),
        ),
        (
            "04_scripts.txt",
            {"quick", "full", "ai_review"},
            lambda output: write_scripts_report(copied_root, output),
        ),
        (
            "05_git_deep.txt",
            ai_security | {"full"},
            lambda output: write_git_deep_report(source_root, output, log, cancel),
        ),
        (
            "06_security_scan.txt",
            {"quick", "full", "ai_review", "security", "minimal"},
            lambda output: write_security_scan_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "07_todo_fixme.txt",
            {"quick", "full", "ai_review", "security"},
            lambda output: write_todo_fixme_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "08_code_metrics.txt",
            {"quick", "full", "ai_review", "security"},
            lambda output: write_code_metrics_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "09_config.txt",
            {"quick", "full", "ai_review", "security", "minimal"},
            lambda output: write_config_report(copied_root, output),
        ),
        (
            "10_docker.txt",
            {"quick", "full", "ai_review", "security"},
            lambda output: write_docker_report(copied_root, output),
        ),
        (
            "11_routes_and_pages.txt",
            {"quick", "full", "ai_review"},
            lambda output: write_routes_and_pages_report(copied_root, output),
        ),
        (
            "12_ai_context_pack.md",
            {"quick", "full", "ai_review", "minimal"},
            lambda output: write_ai_context_pack(
                copied_root, source_root, output, inventory
            ),
        ),
        (
            "13_runbook.md",
            all_profiles,
            lambda output: write_runbook_report(copied_root, output),
        ),
        (
            "14_dependency_graph.md",
            {"quick", "full", "ai_review"},
            lambda output: write_dependency_graph_reports(
                copied_root,
                output,
                reports_dir / "14_dependency_graph.mmd",
                max_bytes_per_file,
            ),
        ),
        (
            "15_architecture_report.md",
            all_profiles,
            lambda output: write_architecture_report(copied_root, output, inventory),
        ),
        (
            "16_key_files_report.md",
            all_profiles,
            lambda output: write_key_files_report(
                copied_root, output, inventory, max_bytes_per_file
            ),
        ),
        (
            "17_code_quality_report.md",
            {"quick", "full", "ai_review", "security"},
            lambda output: write_code_quality_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "18_api_surface_report.md",
            {"full", "ai_review", "security"},
            lambda output: write_api_surface_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "19_frontend_report.md",
            {"full", "ai_review"},
            lambda output: write_frontend_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "20_backend_report.md",
            {"full", "ai_review"},
            lambda output: write_backend_report(copied_root, output, max_bytes_per_file),
        ),
        (
            "21_git_timeline_report.md",
            {"full", "ai_review", "security"},
            lambda output: write_git_timeline_report(source_root, output, log, cancel),
        ),
        (
            "22_project_health_report.md",
            {"quick", "full", "ai_review", "security", "minimal"},
            lambda output: write_project_health_report(copied_root, output, inventory),
        ),
        (
            "23_refactoring_opportunities.md",
            {"quick", "full", "ai_review", "security"},
            lambda output: write_refactoring_opportunities_report(
                copied_root, output, inventory, max_bytes_per_file
            ),
        ),
        (
            "24_architecture_map.md",
            {"quick", "full", "ai_review", "minimal"},
            lambda output: write_architecture_map_report(copied_root, output, inventory),
        ),
        (
            "25_large_files_report.md",
            {"quick", "full", "ai_review", "security"},
            lambda output: write_large_files_report(copied_root, output, inventory),
        ),
        (
            "26_dependency_intelligence.md",
            {"quick", "full", "ai_review", "security"},
            lambda output: write_dependency_intelligence_report(copied_root, output),
        ),
    ]

    for filename, job_profiles, writer in report_jobs:
        if cancel.is_set():
            log("Создание расширенных отчётов остановлено пользователем.")
            break
        if not _should_run(profile, job_profiles):
            log(f"Пропущен отчёт по профилю {profile}: {filename}")
            continue
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

    if not cancel.is_set() and profile in {"full", "ai_review", "quick", "minimal"}:
        try:
            log("Пишу AI_CONTEXT папку")
            write_ai_context_folder(copied_root, source_root, reports_dir / "AI_CONTEXT", inventory)
        except Exception as exc:
            error_file = reports_dir / "ERROR_AI_CONTEXT.txt"
            error_file.write_text(
                f"Failed to create AI_CONTEXT folder\n\n{type(exc).__name__}: {exc}\n\n{traceback.format_exc()}",
                encoding="utf-8",
                errors="replace",
            )
            log(f"Ошибка создания AI_CONTEXT: {exc}")

    if not cancel.is_set() and profile in {"full", "ai_review", "quick", "minimal", "security"}:
        try:
            log("Пишу AI_PROMPTS папку")
            write_ai_prompt_files(copied_root, reports_dir / "AI_PROMPTS")
        except Exception as exc:
            error_file = reports_dir / "ERROR_AI_PROMPTS.txt"
            error_file.write_text(
                f"Failed to create AI_PROMPTS folder\n\n{type(exc).__name__}: {exc}\n\n{traceback.format_exc()}",
                encoding="utf-8",
                errors="replace",
            )
            log(f"Ошибка создания AI_PROMPTS: {exc}")

    log("Расширенные аналитические отчёты готовы")
