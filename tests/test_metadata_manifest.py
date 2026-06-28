"""Tests for write_manifest: verifies diff metadata, archive info, and ignored dirs are recorded."""

from __future__ import annotations

import json
from pathlib import Path

from project_exporter_desktop.config import Config
from project_exporter_desktop.models import (
    ArchiveBuildResult,
    CopyStats,
    ExportPaths,
    TextDumpStats,
)
from project_exporter_desktop.reports.metadata import write_manifest
from project_exporter_desktop.services.diff_service import DiffFile, DiffSelection


def _paths(root: Path) -> ExportPaths:
    staging = root / "bundle"
    reports = staging / "reports"
    insights = reports / "insights"
    return ExportPaths(
        desktop=root,
        source_root=root / "source",
        project_name="source",
        bundle_name="source_export_test",
        staging_dir=staging,
        final_zip=root / "source_export_test.zip",
        archive_set_dir=root / "source_export_test_archives",
        project_dir=staging / "source",
        reports_dir=reports,
        insights_dir=insights,
        manifest_file=staging / "manifest.json",
        project_profile_file=staging / "PROJECT_PROFILE.json",
        index_file=staging / "INDEX.md",
        structure_report=reports / "01_structure.txt",
        git_report=reports / "02_git.txt",
        text_dump=reports / "03_text_dump.txt",
    )


def test_manifest_preserves_deleted_diff_files_and_archive_payload(tmp_path: Path) -> None:
    paths = _paths(tmp_path)
    paths.staging_dir.mkdir(parents=True)
    archive_result = ArchiveBuildResult(
        archives=[tmp_path / "part_001.zip", tmp_path / "part_002.zip"],
        output_dir=tmp_path / "archive_set",
        split=True,
        file_count=42,
        skipped_project_files=3,
        oversized_files=["huge.bin"],
    )
    diff_selection = DiffSelection(
        mode="last_export",
        base="последний экспорт",
        paths=frozenset({"changed.py"}),
        files=(
            DiffFile("changed.py", "modified"),
            DiffFile("old.py", "deleted"),
            DiffFile("new_name.py", "renamed", "old_name.py"),
        ),
    )

    write_manifest(
        paths=paths,
        config=Config(diff_export_mode="last_export", extra_ignored_dirs=["Cache"]),
        copy_stats=CopyStats(files_copied=1, files_skipped_by_diff=2),
        text_stats=TextDumpStats(scanned=1, written=1),
        extra_ignored_dirs=frozenset({"cache"}),
        cancelled=False,
        archive_result=archive_result,
        diff_selection=diff_selection,
    )

    data = json.loads(paths.manifest_file.read_text(encoding="utf-8"))
    assert data["diff_selection"]["selected_paths_count"] == 1
    assert {"path": "old.py", "status": "deleted"} in data["diff_selection"]["deleted_files"]
    assert {
        "path": "new_name.py",
        "status": "renamed",
        "old_path": "old_name.py",
    } in data["diff_selection"]["files"]
    assert data["archives"]["split"] is True
    assert data["archives"]["archive_names"] == ["part_001.zip", "part_002.zip"]
    assert data["archives"]["oversized_files"] == ["huge.bin"]
    assert data["ignored_dirs"]["user_extras"] == ["cache"]
