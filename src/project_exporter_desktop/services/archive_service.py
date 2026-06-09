from __future__ import annotations

import json
import threading
import zipfile
from collections import defaultdict
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
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


@dataclass(slots=True)
class ArchivePartPlan:
    index: int
    group_hint: str
    entries: list[ArchiveEntry] = field(default_factory=list)

    @property
    def estimated_bytes(self) -> int:
        return sum(entry.size for entry in self.entries)

    @property
    def file_count(self) -> int:
        return len(self.entries)

    @property
    def groups(self) -> list[str]:
        return sorted({entry.group for entry in self.entries})


@dataclass(slots=True)
class ArchivePlan:
    split: bool
    limit_bytes: int
    target_bytes: int
    skipped_project_files: int
    entries: list[ArchiveEntry] = field(default_factory=list)
    parts: list[ArchivePartPlan] = field(default_factory=list)

    @property
    def estimated_bytes(self) -> int:
        return sum(entry.size for entry in self.entries)

    @property
    def file_count(self) -> int:
        return len(self.entries)


def _is_inside(path: Path, parent: Path) -> bool:
    try:
        path.resolve().relative_to(parent.resolve())
        return True
    except ValueError:
        return False


def classify_archive_group(arcname: Path, project_name: str) -> str:
    parts = [part.casefold() for part in arcname.parts]
    suffix = arcname.suffix.casefold().lstrip(".")
    name = arcname.name.casefold()

    if not parts or parts[0] in {"index.md", "manifest.json", "project_profile.json"}:
        return "00_metadata"
    if parts[0] == "reports":
        if "ai_context" in parts:
            return "02_ai_context"
        if "ai_prompts" in parts:
            return "03_ai_prompts"
        return "01_reports"
    if "test" in parts or "tests" in parts or any(part in {"__tests__", "spec"} for part in parts):
        return "20_tests"
    if name in {"readme.md", "license", "changelog.md"} or suffix in {"md", "rst", "adoc", "txt"}:
        return "30_docs"
    if suffix in {"py", "pyw", "pyi"}:
        return "40_python_source"
    if suffix in {
        "js",
        "jsx",
        "ts",
        "tsx",
        "mjs",
        "cjs",
        "css",
        "scss",
        "sass",
        "html",
        "vue",
        "svelte",
        "astro",
    }:
        return "41_frontend_source"
    if suffix in {"go", "rs", "java", "kt", "kts", "cs", "cpp", "c", "h", "hpp", "rb", "php"}:
        return "42_backend_or_system_source"
    if suffix in {
        "json",
        "yaml",
        "yml",
        "toml",
        "ini",
        "cfg",
        "conf",
        "lock",
        "dockerfile",
    } or name.startswith("dockerfile"):
        return "50_config_and_locks"
    if suffix in {"png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "pdf", "docx", "xlsx", "pptx"}:
        return "60_assets_and_binary_docs"
    if suffix in {"csv", "tsv", "sql", "xml"}:
        return "70_data_and_exports"
    if parts and parts[0] == project_name.casefold():
        return "80_other_project_files"
    return "90_other"


def _iter_archive_entries(
    paths: ExportPaths, include_project: bool
) -> tuple[list[ArchiveEntry], int]:
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
                file_path, arcname, size, classify_archive_group(arcname, paths.project_name)
            )
        )
    entries.sort(key=lambda entry: (entry.group, str(entry.arcname).casefold()))
    return entries, skipped_project_files


def _plan_logical_parts(entries: list[ArchiveEntry], target_bytes: int) -> list[ArchivePartPlan]:
    grouped: dict[str, list[ArchiveEntry]] = defaultdict(list)
    for entry in entries:
        grouped[entry.group].append(entry)

    parts: list[ArchivePartPlan] = []
    current_entries: list[ArchiveEntry] = []
    current_size = 0
    current_group = ""

    def flush() -> None:
        nonlocal current_entries, current_size, current_group
        if not current_entries:
            return
        groups = sorted({entry.group for entry in current_entries})
        group_hint = groups[0] if groups else "empty"
        parts.append(
            ArchivePartPlan(index=len(parts) + 1, group_hint=group_hint, entries=current_entries)
        )
        current_entries = []
        current_size = 0
        current_group = ""

    for group in sorted(grouped):
        for entry in grouped[group]:
            # Oversized individual files become their own logical part. The ZIP
            # may still exceed the hard limit; the manifest calls that out.
            if current_entries and (
                current_group != group or current_size + entry.size > target_bytes
            ):
                flush()
            current_entries.append(entry)
            current_size += entry.size
            current_group = group
            if entry.size >= target_bytes:
                flush()
    flush()
    return parts


def build_archive_plan(
    paths: ExportPaths, include_project: bool, part_limit_bytes: int = MAX_ARCHIVE_PART_BYTES
) -> ArchivePlan:
    limit_bytes = max(1, int(part_limit_bytes))
    target_bytes = min(ARCHIVE_PART_TARGET_BYTES, max(1, limit_bytes - 8 * 1024 * 1024))
    entries, skipped_project_files = _iter_archive_entries(paths, include_project)
    estimated_total = sum(entry.size for entry in entries)
    if estimated_total <= target_bytes:
        part = ArchivePartPlan(index=1, group_hint="single", entries=entries)
        return ArchivePlan(False, limit_bytes, target_bytes, skipped_project_files, entries, [part])
    return ArchivePlan(
        True,
        limit_bytes,
        target_bytes,
        skipped_project_files,
        entries,
        _plan_logical_parts(entries, target_bytes),
    )


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


def _write_restore_files(
    paths: ExportPaths,
    result: ArchiveBuildResult,
    part_map: list[dict[str, object]],
    limit_bytes: int,
) -> None:
    if result.output_dir is None:
        return
    manifest = {
        "generated_at": human_now(),
        "bundle_name": paths.bundle_name,
        "split": result.split,
        "limit_bytes": limit_bytes,
        "limit_human": format_bytes(limit_bytes),
        "archives": [path.name for path in result.archives],
        "parts": part_map,
        "oversized_files": result.oversized_files,
        "restore_note": "Extract all ZIP files into the same destination folder. Every archive preserves original paths.",
    }
    (result.output_dir / "ARCHIVE_SET_MANIFEST.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8", newline="\n"
    )
    (result.output_dir / "RESTORE_INSTRUCTIONS.md").write_text(
        "# Restore Instructions\n\n"
        "1. Create an empty destination folder.\n"
        "2. Extract every ZIP file from this archive set into that same folder.\n"
        "3. Keep the relative paths exactly as stored in the archives.\n"
        "4. Optionally run `python restore_archives.py <destination-folder>` from this directory.\n\n"
        "The file `ARCHIVE_SET_MANIFEST.json` lists all archive parts and any oversized entries.\n",
        encoding="utf-8",
        newline="\n",
    )
    (result.output_dir / "restore_archives.py").write_text(
        """from __future__ import annotations

import sys
import zipfile
from pathlib import Path


def main() -> int:
    source_dir = Path(__file__).resolve().parent
    destination = Path(sys.argv[1]).expanduser().resolve() if len(sys.argv) > 1 else source_dir / 'restored'
    destination.mkdir(parents=True, exist_ok=True)
    archives = sorted(source_dir.glob('*.zip'))
    if not archives:
        print('No .zip archives found next to restore_archives.py')
        return 1
    for archive_path in archives:
        print(f'Extracting {archive_path.name} -> {destination}')
        with zipfile.ZipFile(archive_path) as archive:
            archive.extractall(destination)
    print(f'Done: {destination}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
""",
        encoding="utf-8",
        newline="\n",
    )


def write_archive_plan_report(plan: ArchivePlan, output_file: Path) -> None:
    output_file.parent.mkdir(parents=True, exist_ok=True)
    data = {
        "generated_at": human_now(),
        "split_planned": plan.split,
        "limit_bytes": plan.limit_bytes,
        "limit_human": format_bytes(plan.limit_bytes),
        "target_bytes": plan.target_bytes,
        "target_human": format_bytes(plan.target_bytes),
        "estimated_total_bytes": plan.estimated_bytes,
        "estimated_total_human": format_bytes(plan.estimated_bytes),
        "files": plan.file_count,
        "skipped_project_files": plan.skipped_project_files,
        "parts": [
            {
                "index": part.index,
                "group_hint": part.group_hint,
                "groups": part.groups,
                "files": part.file_count,
                "estimated_bytes": part.estimated_bytes,
                "estimated_human": format_bytes(part.estimated_bytes),
            }
            for part in plan.parts
        ],
    }
    output_file.with_suffix(".json").write_text(
        json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8", newline="\n"
    )
    with output_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write("# Archive Plan\n\n")
        out.write(f"Generated: {data['generated_at']}\n\n")
        out.write(f"- Split planned: `{plan.split}`\n")
        out.write(f"- Hard limit: {format_bytes(plan.limit_bytes)}\n")
        out.write(f"- Planning target: {format_bytes(plan.target_bytes)}\n")
        out.write(f"- Estimated archive input size: {format_bytes(plan.estimated_bytes)}\n")
        out.write(f"- Files to archive: {plan.file_count:,}\n")
        out.write(f"- Planned archive count: {len(plan.parts):,}\n\n")
        out.write("## Parts\n\n")
        for part in plan.parts:
            out.write(
                f"- Part {part.index:03d} `{part.group_hint}` — {part.file_count:,} files — {format_bytes(part.estimated_bytes)} — groups: {', '.join(part.groups)}\n"
            )


def _predicted_result_for_plan(paths: ExportPaths, plan: ArchivePlan) -> ArchiveBuildResult:
    if not plan.split:
        return ArchiveBuildResult(
            archives=[paths.final_zip],
            output_dir=None,
            split=False,
            file_count=plan.file_count,
            skipped_project_files=plan.skipped_project_files,
        )
    archives = [
        paths.archive_set_dir / f"{paths.bundle_name}_part_{part.index:03d}_{part.group_hint}.zip"
        for part in plan.parts
    ]
    return ArchiveBuildResult(
        archives=archives,
        output_dir=paths.archive_set_dir,
        split=True,
        file_count=plan.file_count,
        skipped_project_files=plan.skipped_project_files,
    )


def build_final_archives(
    paths: ExportPaths,
    include_project: bool,
    log: Callable[[str], None],
    cancel: threading.Event,
    part_limit_bytes: int = MAX_ARCHIVE_PART_BYTES,
    progress: Callable[[int, str, str], None] | None = None,
    pre_archive_hook: Callable[[ArchiveBuildResult], None] | None = None,
) -> ArchiveBuildResult:
    """Create one ZIP or a logical archive set without first creating a huge ZIP.

    The decision is based on an uncompressed-size planning target below the hard
    limit. This avoids writing a giant temporary archive for large projects. If a
    planned single archive unexpectedly compresses above the limit, the function
    falls back to logical split archives.
    """
    plan = build_archive_plan(paths, include_project, part_limit_bytes)
    write_archive_plan_report(plan, paths.insights_dir / "27_archive_plan.md")
    try:
        from ..reports.insights.dashboard import write_html_dashboard

        write_html_dashboard(paths.insights_dir, paths.insights_dir / "REPORT_DASHBOARD.html")
    except Exception as exc:
        log(f"Не удалось обновить REPORT_DASHBOARD.html перед архивацией: {exc}")

    # Rebuild after writing archive-plan/dashboard files so the archive contains
    # those generated files too. Rewriting the plan once more keeps size/count
    # estimates close to the final archive input set.
    plan = build_archive_plan(paths, include_project, part_limit_bytes)
    write_archive_plan_report(plan, paths.insights_dir / "27_archive_plan.md")
    try:
        from ..reports.insights.dashboard import write_html_dashboard

        write_html_dashboard(paths.insights_dir, paths.insights_dir / "REPORT_DASHBOARD.html")
    except Exception as exc:
        log(f"Не удалось обновить REPORT_DASHBOARD.html перед архивацией: {exc}")
    plan = build_archive_plan(paths, include_project, part_limit_bytes)
    if pre_archive_hook is not None and not cancel.is_set():
        pre_archive_hook(_predicted_result_for_plan(paths, plan))
        # Metadata files can change slightly when manifest/dashboard are refreshed.
        # Rebuild the plan once more so the archive input list contains the latest files.
        plan = build_archive_plan(paths, include_project, part_limit_bytes)

    if plan.skipped_project_files:
        log(f"Копия проекта исключена из ZIP по настройке ({plan.skipped_project_files:,} файлов)")

    if not plan.split:
        log(f"Создаю одиночный ZIP по плану: {paths.final_zip.name}")
        count = _write_zip(paths.final_zip, plan.parts[0].entries, cancel)
        if cancel.is_set():
            return ArchiveBuildResult(
                [paths.final_zip] if paths.final_zip.exists() else [],
                None,
                False,
                count,
                plan.skipped_project_files,
            )
        compressed_size = paths.final_zip.stat().st_size if paths.final_zip.exists() else 0
        if compressed_size <= plan.limit_bytes:
            log(f"ZIP готов: {count:,} файлов, {format_bytes(compressed_size)} → {paths.final_zip}")
            if progress:
                progress(100, "Archive ready", str(paths.final_zip))
            return ArchiveBuildResult(
                [paths.final_zip], None, False, count, plan.skipped_project_files
            )
        # Very rare, but possible due to ZIP overhead/incompressible data.
        log(
            f"Одиночный ZIP превысил лимит после записи ({format_bytes(compressed_size)} > {format_bytes(plan.limit_bytes)}). Пересобираю частями."
        )
        try:
            paths.final_zip.unlink(missing_ok=True)
        except Exception:
            pass
        plan.split = True
        plan.parts = _plan_logical_parts(plan.entries, plan.target_bytes)
        if pre_archive_hook is not None and not cancel.is_set():
            pre_archive_hook(_predicted_result_for_plan(paths, plan))
            plan = build_archive_plan(paths, include_project, part_limit_bytes)
            plan.split = True
            plan.parts = _plan_logical_parts(plan.entries, plan.target_bytes)

    paths.archive_set_dir.mkdir(parents=True, exist_ok=True)
    result = ArchiveBuildResult(
        archives=[],
        output_dir=paths.archive_set_dir,
        split=True,
        file_count=0,
        skipped_project_files=plan.skipped_project_files,
    )
    part_map: list[dict[str, object]] = []
    total_parts = max(1, len(plan.parts))
    for part in plan.parts:
        if cancel.is_set():
            break
        archive_name = f"{paths.bundle_name}_part_{part.index:03d}_{part.group_hint}.zip"
        archive_path = paths.archive_set_dir / archive_name
        if progress:
            percent = 85 + int((part.index - 1) / total_parts * 14)
            progress(percent, "Creating archive parts", archive_name)
        count = _write_zip(archive_path, part.entries, cancel)
        result.archives.append(archive_path)
        result.file_count += count
        compressed_size = archive_path.stat().st_size if archive_path.exists() else 0
        if compressed_size > plan.limit_bytes:
            oversized = [
                str(entry.arcname).replace("\\", "/")
                for entry in part.entries
                if entry.size >= plan.target_bytes
            ]
            result.oversized_files.extend(oversized or [archive_name])
        part_map.append(
            {
                "archive": archive_name,
                "groups": part.groups,
                "files": count,
                "estimated_input_size": part.estimated_bytes,
                "estimated_input_size_human": format_bytes(part.estimated_bytes),
                "compressed_size": compressed_size,
                "compressed_size_human": format_bytes(compressed_size),
            }
        )
        log(
            f"Создан архив-часть {part.index}: {archive_name} ({count:,} файлов, {format_bytes(compressed_size)})"
        )

    _write_restore_files(paths, result, part_map, plan.limit_bytes)
    log(f"Архивы готовы: {len(result.archives):,} ZIP → {paths.archive_set_dir}")
    if progress:
        progress(100, "Archive set ready", str(paths.archive_set_dir))
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
