from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

@dataclass(slots=True)
class ExportPaths:
    desktop: Path
    source_root: Path
    project_name: str
    bundle_name: str  # "{project}_export_{timestamp}"
    staging_dir: Path  # ~/Desktop/{bundle_name}
    final_zip: Path  # ~/Desktop/{bundle_name}.zip
    project_dir: Path  # {staging_dir}/{project_name}
    reports_dir: Path  # {staging_dir}/reports
    insights_dir: Path  # {staging_dir}/reports/insights
    manifest_file: Path  # {staging_dir}/manifest.json
    index_file: Path  # {staging_dir}/INDEX.md
    structure_report: Path  # {reports_dir}/01_structure.txt
    git_report: Path  # {reports_dir}/02_git.txt
    text_dump: Path

@dataclass(slots=True)
class CopyStats:
    dirs_created: int = 0
    files_copied: int = 0
    dirs_skipped: int = 0
    files_skipped: int = 0
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
