from __future__ import annotations

from datetime import datetime
from pathlib import Path

from ..constants import IGNORED_DIR_NAMES, MONTHS
from ..models import ExportPaths
from .time_utils import now_stamp


def desktop_path() -> Path:
    desktop = Path.home() / "Desktop"
    return desktop if desktop.exists() else Path.home()


def sanitize_name(name: str) -> str:
    cleaned = "".join(ch for ch in name if ch not in '<>:"/\\|?*').strip()
    return cleaned or "project"


def is_relative_to(child: Path, parent: Path) -> bool:
    try:
        child.resolve().relative_to(parent.resolve())
        return True
    except ValueError:
        return False


def validate_source_root(path_text: str) -> Path:
    if not path_text.strip():
        raise ValueError("Укажите корневую папку проекта.")

    root = Path(path_text).expanduser().resolve()

    if not root.exists():
        raise ValueError("Указанная папка не существует.")
    if not root.is_dir():
        raise ValueError("Указанный путь не является папкой.")
    if root.parent == root:
        raise ValueError("Нельзя выбирать корень диска.")
    if root == Path.home().resolve():
        raise ValueError("Не выбирайте всю домашнюю папку целиком. Укажите конкретный проект.")

    return root


def build_export_paths(source_root: Path) -> ExportPaths:
    """Allocate a unique bundle path. If a collision is detected, suffix it."""
    desktop = desktop_path()
    project_name = sanitize_name(source_root.name)

    base = f"{project_name}_export_{now_stamp()}"
    bundle_name = base
    staging = desktop / bundle_name
    final_zip = desktop / f"{bundle_name}.zip"
    archive_set_dir = desktop / f"{bundle_name}_archives"

    counter = 1
    while staging.exists() or final_zip.exists() or archive_set_dir.exists():
        bundle_name = f"{base}_{counter}"
        staging = desktop / bundle_name
        final_zip = desktop / f"{bundle_name}.zip"
        archive_set_dir = desktop / f"{bundle_name}_archives"
        counter += 1

    reports_dir = staging / "reports"
    insights_dir = reports_dir / "insights"

    return ExportPaths(
        desktop=desktop,
        source_root=source_root,
        project_name=project_name,
        bundle_name=bundle_name,
        staging_dir=staging,
        final_zip=final_zip,
        archive_set_dir=archive_set_dir,
        project_dir=staging / project_name,
        reports_dir=reports_dir,
        insights_dir=insights_dir,
        manifest_file=staging / "manifest.json",
        project_profile_file=staging / "PROJECT_PROFILE.json",
        index_file=staging / "INDEX.md",
        structure_report=reports_dir / "01_structure.txt",
        git_report=reports_dir / "02_git.txt",
        text_dump=reports_dir / "03_text_dump.txt",
    )


def should_ignore_dir(name: str, extra: frozenset[str] | set[str] = frozenset()) -> bool:
    normalised = name.casefold()
    defaults = {item.casefold() for item in IGNORED_DIR_NAMES}
    extras = {item.casefold() for item in extra}
    return normalised in defaults or normalised in extras


def rel_display(path: Path, root: Path) -> str:
    rel = path.relative_to(root)
    if str(rel) == ".":
        return "."
    return ".\\" + str(rel).replace("/", "\\")


def ps_mode(path: Path) -> str:
    return "d-----" if path.is_dir() else "-a----"


def ps_date(timestamp: float) -> str:
    dt = datetime.fromtimestamp(timestamp)
    month = MONTHS[dt.month - 1]
    return f"{dt.day:02d}-{month}-{dt.year % 100:02d}     {dt.hour:02d}:{dt.minute:02d}"