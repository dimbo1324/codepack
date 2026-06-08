from __future__ import annotations

import json
import threading
import zipfile
from collections import defaultdict
from collections.abc import Callable, Iterable
from dataclasses import asdict, dataclass
from pathlib import Path

from ..constants import ARCHIVE_PART_TARGET_BYTES, MAX_ARCHIVE_PART_BYTES
from ..models import ArchiveBuildResult, ExportPaths
from ..utils.text_utils import format_bytes
from ..utils.time_utils import human_now


@dataclass(frozen=True, slots=True)
class ArchiveEntry:
    path: Path
    arcname: Path
    size: int
    group: str


def _is_inside(path: Path, parent: Path) -> bool:
    try:
        path.resolve().relative_to(parent.resolve())
        return True
    except ValueError:
        return False


def _classify_archive_group(arcname: Path, project_name: str) -> str:
    parts = [part.casefold() for part in arcname.parts]
    suffix = arcname.suffix.casefold().lstrip(".")
    name = arcname.name.casefold()

    if not parts or parts[0] in {"index.md", "manifest.json", "project_profile.json"}:
        return "00_metadata"
    if parts[0] == "reports":
        return "01_reports_and_ai_context"
    if "test" in parts or "tests" in parts or any(part in {"__tests__", "spec"} for part in parts):
        return "20_tests"
    if name in {"readme.md", "license", "changelog.md"} or suffix in {"md", "rst", "adoc", "txt"}:
        return "30_docs"
    if suffix in {"py", "pyw", "pyi"}:
        return "40_python_source"
    if suffix in {"js", "jsx", "ts", "tsx", "mjs", "cjs", "css", "scss", "sass", "html", "vue", "svelte", "astro"}:
        return "41_frontend_source"
    if suffix in {"go", "rs", "java", "kt", "kts", "cs", "cpp", "c", "h", "hpp", "rb", "php"}:
        return "42_backend_or_system_source"
    if suffix in {"json", "yaml", "yml", "toml", "ini", "cfg", "conf", "lock", "dockerfile"} or name.startswith("dockerfile"):
        return "50_config_and_locks"
    if suffix in {"png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "pdf", "docx", "xlsx", "pptx"}:
        return "60_assets_and_binary_docs"
    if suffix in {"csv", "tsv", "sql", "xml"}:
        return "70_data_and_exports"
    if parts and parts[0] == project_name.casefold():
        return "80_other_project_files"
    return "90_other"


def _iter_archive_entries(paths: ExportPaths, include_project: bool) -> tuple[list[ArchiveEntry], int]:
    entries: list[ArchiveEntry] = []
    skipped_project_files = 0

    project_dir_resolved = paths.project_dir.resolve()
    staging_resolved = paths.staging_dir.resolve()

    for file_path in paths.staging_dir.rglob("*"):
        if not file_path.is_file():
            continue
        resolved = file_path.resolve()
        if not include_project and _is_inside(resolved, project_dir_resolved):
            skipped_project_files += 1
            continue
        try:
            arcname = resolved.relative_to(staging_resolved)
        except ValueError:
            arcname = Path(file_path.name)
        try:
            size = file_path.stat().st_size
        except Exception:
            size = 0
        entries.append(
            ArchiveEntry(
                path=file_path,
                arcname=arcname,
                size=size,
                group=_classify_archive_group(arcname, paths.project_name),
            )
        )

    entries.sort(key=lambda entry: (entry.group, str(entry.arcname).casefold()))
    return entries, skipped_project_files


def _write_zip(archive_path: Path, entries: Iterable[ArchiveEntry], cancel: threading.Event) -> int:
    count = 0
    with zipfile.ZipFile(
        archive_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6
    ) as archive:
        for entry in entries:
            if cancel.is_set():
                break
            archive.write(entry.path, entry.arcname)
            count += 1
    return count


def _plan_logical_parts(entries: list[ArchiveEntry], target_bytes: int) -> list[list[ArchiveEntry]]:
    grouped: dict[str, list[ArchiveEntry]] = defaultdict(list)
    for entry in entries:
        grouped[entry.group].append(entry)

    parts: list[list[ArchiveEntry]] = []
    current: list[ArchiveEntry] = []
    current_size = 0
    current_group = ""

    for group in sorted(grouped):
        for entry in grouped[group]:
            should_start_new = (
                current
                and (current_group != group or current_size + entry.size > target_bytes)
            )
            if should_start_new:
                parts.append(current)
                current = []
                current_size = 0
            current.append(entry)
            current_size += entry.size
            current_group = group
    if current:
        parts.append(current)
    return parts


def _write_split_manifest(
    paths: ExportPaths,
    result: ArchiveBuildResult,
    part_map: list[dict[str, object]],
    limit_bytes: int,
) -> None:
    if result.output_dir is None:
        return
    data = {
        "generated_at": human_now(),
        "bundle_name": paths.bundle_name,
        "split": result.split,
        "limit_bytes": limit_bytes,
        "limit_human": format_bytes(limit_bytes),
        "archives": [str(path.name) for path in result.archives],
        "parts": part_map,
        "oversized_files": result.oversized_files,
        "restore_note": (
            "Extract all ZIP files into the same destination folder. "
            "Normal parts preserve original paths. Oversized single-file chunks, "
            "if any, are stored under oversized_chunks/ and are documented here."
        ),
    }
    (result.output_dir / "ARCHIVE_SET_MANIFEST.json").write_text(
        json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8"
    )


def build_final_archives(
    paths: ExportPaths,
    include_project: bool,
    log: Callable[[str], None],
    cancel: threading.Event,
    part_limit_bytes: int = MAX_ARCHIVE_PART_BYTES,
) -> ArchiveBuildResult:
    """Create one ZIP or a logical set of <=512 MB ZIP archives.

    Strategy: first try a single archive. If it exceeds the configured limit,
    remove it and create a folder on the Desktop with logical archive parts. The
    split plan groups files by reports/source/docs/config/assets/data instead of
    arbitrary order.
    """
    limit_bytes = max(1, part_limit_bytes)
    target_bytes = min(ARCHIVE_PART_TARGET_BYTES, max(1, limit_bytes - 8 * 1024 * 1024))
    entries, skipped_project_files = _iter_archive_entries(paths, include_project)

    log(f"Создаю итоговый ZIP-архив: {paths.final_zip.name}")
    count = _write_zip(paths.final_zip, entries, cancel)
    if cancel.is_set():
        log("Создание архива остановлено пользователем.")
        return ArchiveBuildResult(
            archives=[paths.final_zip] if paths.final_zip.exists() else [],
            split=False,
            file_count=count,
            skipped_project_files=skipped_project_files,
        )

    single_size = paths.final_zip.stat().st_size if paths.final_zip.exists() else 0
    if single_size <= limit_bytes:
        if skipped_project_files:
            log(
                f"Копия проекта исключена из ZIP по настройке "
                f"({skipped_project_files:,} файлов)"
            )
        log(f"ZIP готов: {count:,} файлов, {format_bytes(single_size)} → {paths.final_zip}")
        return ArchiveBuildResult(
            archives=[paths.final_zip],
            split=False,
            file_count=count,
            skipped_project_files=skipped_project_files,
        )

    log(
        f"Один ZIP получился больше лимита ({format_bytes(single_size)} > {format_bytes(limit_bytes)}). "
        "Перехожу к логическому разбиению."
    )
    try:
        paths.final_zip.unlink(missing_ok=True)
    except Exception:
        pass

    paths.archive_set_dir.mkdir(parents=True, exist_ok=True)
    logical_parts = _plan_logical_parts(entries, target_bytes)
    result = ArchiveBuildResult(
        archives=[],
        output_dir=paths.archive_set_dir,
        split=True,
        file_count=0,
        skipped_project_files=skipped_project_files,
    )
    part_map: list[dict[str, object]] = []

    for index, part_entries in enumerate(logical_parts, start=1):
        if cancel.is_set():
            break
        groups = sorted({entry.group for entry in part_entries})
        primary_group = groups[0] if groups else "empty"
        archive_name = f"{paths.bundle_name}_part_{index:03d}_{primary_group}.zip"
        archive_path = paths.archive_set_dir / archive_name
        part_count = _write_zip(archive_path, part_entries, cancel)
        result.archives.append(archive_path)
        result.file_count += part_count
        size = archive_path.stat().st_size if archive_path.exists() else 0
        if size > limit_bytes:
            result.oversized_files.extend(
                str(entry.arcname).replace("\\", "/") for entry in part_entries if entry.size >= target_bytes
            )
        part_map.append(
            {
                "archive": archive_name,
                "groups": groups,
                "files": part_count,
                "compressed_size": size,
                "compressed_size_human": format_bytes(size),
            }
        )
        log(f"Создан архив-часть {index}: {archive_name} ({part_count:,} файлов, {format_bytes(size)})")

    _write_split_manifest(paths, result, part_map, limit_bytes)
    if skipped_project_files:
        log(
            f"Копия проекта исключена из ZIP по настройке "
            f"({skipped_project_files:,} файлов)"
        )
    log(f"Архивы готовы: {len(result.archives):,} файлов ZIP → {paths.archive_set_dir}")
    return result


# Backwards-compatible wrapper for older imports/tests.
def build_final_zip(
    paths: ExportPaths,
    include_project: bool,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> int:
    result = build_final_archives(paths, include_project, log, cancel)
    return result.file_count
