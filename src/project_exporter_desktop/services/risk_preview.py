from __future__ import annotations

import os
from pathlib import Path

from ..constants import ARCHIVE_PART_TARGET_BYTES
from ..models import RiskPreviewItem, RiskPreviewReport
from ..utils.path_utils import rel_display, should_ignore_dir
from ..utils.text_utils import format_bytes
from .export_policy import classify_sensitive_file
from .git_diff import DiffSelection

_ARCHIVE_OR_DUMP_SUFFIXES = {
    "zip",
    "rar",
    "7z",
    "tar",
    "gz",
    "xz",
    "bz2",
    "db",
    "sqlite",
    "sqlite3",
    "dump",
    "bak",
    "backup",
}


def build_pre_export_risk_preview(
    source_root: Path,
    ignored_dirs: frozenset[str] | set[str],
    diff_selection: DiffSelection,
    large_file_threshold_bytes: int = ARCHIVE_PART_TARGET_BYTES,
) -> RiskPreviewReport:
    report = RiskPreviewReport(
        diff_limited=diff_selection.is_limited,
        diff_file_count=len(diff_selection.paths) if diff_selection.paths is not None else None,
        git_warning=diff_selection.warning,
    )
    selected_paths = diff_selection.paths

    for current_dir, dirnames, filenames in os.walk(source_root, topdown=True, followlinks=False):
        current = Path(current_dir)
        report.scanned_dirs += 1

        safe_dirnames: list[str] = []
        for dirname in dirnames:
            child = current / dirname
            if should_ignore_dir(dirname, ignored_dirs) or child.is_symlink():
                report.ignored_dirs += 1
                continue
            safe_dirnames.append(dirname)
        dirnames[:] = safe_dirnames

        for filename in filenames:
            path = current / filename
            if path.is_symlink():
                continue
            try:
                rel = path.relative_to(source_root)
            except ValueError:
                continue
            rel_key = str(rel).replace("/", "\\")
            if selected_paths is not None and rel_key not in selected_paths:
                continue

            report.scanned_files += 1
            try:
                size = path.stat().st_size
            except Exception:
                size = 0
            report.estimated_selected_bytes += size

            safety = classify_sensitive_file(rel)
            if safety.skip and len(report.sensitive_files) < 100:
                report.sensitive_files.append(
                    RiskPreviewItem(
                        rel_display(path, source_root), safety.reason, size, safety.severity
                    )
                )

            suffix = path.suffix.casefold().lstrip(".")
            if suffix in _ARCHIVE_OR_DUMP_SUFFIXES and len(report.archive_or_dump_files) < 100:
                report.archive_or_dump_files.append(
                    RiskPreviewItem(
                        rel_display(path, source_root),
                        f"archive/dump/database suffix .{suffix}",
                        size,
                        "medium",
                    )
                )

            if size >= large_file_threshold_bytes and len(report.large_files) < 100:
                report.large_files.append(
                    RiskPreviewItem(
                        rel_display(path, source_root),
                        f"large file >= {format_bytes(large_file_threshold_bytes)}",
                        size,
                        "medium",
                    )
                )

    return report


def format_risk_preview_for_user(report: RiskPreviewReport, safe_mode: str) -> str:
    lines = [
        "Pre-export risk preview",
        "",
        f"Scanned files: {report.scanned_files:,}",
        f"Estimated selected size: {format_bytes(report.estimated_selected_bytes)}",
        f"Ignored folders: {report.ignored_dirs:,}",
        f"Safe export mode: {safe_mode}",
    ]
    if report.diff_limited:
        lines.append(
            f"Diff-limited export: yes ({report.diff_file_count or 0:,} Git paths selected)"
        )
    if report.git_warning:
        lines.append(f"Git warning: {report.git_warning}")

    def add_section(title: str, items: list[RiskPreviewItem]) -> None:
        lines.extend(["", title])
        if not items:
            lines.append("- none")
            return
        for item in items[:12]:
            lines.append(
                f"- [{item.severity}] {item.path} — {item.reason}; {format_bytes(item.size)}"
            )
        if len(items) > 12:
            lines.append(f"- ... and {len(items) - 12:,} more")

    add_section("Sensitive-looking files", report.sensitive_files)
    add_section("Large files", report.large_files)
    add_section("Archives, dumps or local databases", report.archive_or_dump_files)

    lines.extend(
        [
            "",
            "Continue only if this preview matches what you expect.",
            "In Safe mode, high-risk files are excluded during project copy.",
        ]
    )
    return "\n".join(lines)
