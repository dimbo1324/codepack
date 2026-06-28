from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path


@dataclass(slots=True)
class ExportPaths:
    desktop: Path
    source_root: Path
    project_name: str
    bundle_name: str
    staging_dir: Path
    final_zip: Path
    archive_set_dir: Path
    project_dir: Path
    reports_dir: Path
    insights_dir: Path
    manifest_file: Path
    project_profile_file: Path
    index_file: Path
    structure_report: Path
    git_report: Path
    text_dump: Path


@dataclass(slots=True)
class CopyStats:
    dirs_created: int = 0
    files_copied: int = 0
    dirs_skipped: int = 0
    files_skipped: int = 0
    files_skipped_by_safety: int = 0
    files_skipped_by_diff: int = 0
    symlinks_skipped: int = 0
    errors: int = 0


@dataclass(slots=True)
class TextDumpStats:
    scanned: int = 0
    written: int = 0
    skipped_binary: int = 0
    skipped_large: int = 0
    skipped_decode: int = 0
    skipped_not_text: int = 0


@dataclass(slots=True)
class RiskPreviewItem:
    path: str
    reason: str
    size: int = 0
    severity: str = "medium"


@dataclass(slots=True)
class RiskPreviewReport:
    scanned_files: int = 0
    scanned_dirs: int = 0
    ignored_dirs: int = 0
    sensitive_files: list[RiskPreviewItem] = field(default_factory=list)
    large_files: list[RiskPreviewItem] = field(default_factory=list)
    archive_or_dump_files: list[RiskPreviewItem] = field(default_factory=list)
    estimated_selected_bytes: int = 0
    diff_limited: bool = False
    diff_file_count: int | None = None
    git_warning: str | None = None

    @property
    def has_warnings(self) -> bool:
        return bool(
            self.sensitive_files
            or self.large_files
            or self.archive_or_dump_files
            or self.git_warning
        )


@dataclass(slots=True)
class ArchiveBuildResult:
    archives: list[Path] = field(default_factory=list)
    output_dir: Path | None = None
    split: bool = False
    file_count: int = 0
    skipped_project_files: int = 0
    oversized_files: list[str] = field(default_factory=list)

    @property
    def primary_result(self) -> Path | None:
        if self.split and self.output_dir is not None:
            return self.output_dir
        if self.archives:
            return self.archives[0]
        return self.output_dir