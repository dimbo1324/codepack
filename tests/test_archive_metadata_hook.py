"""Tests that the pre-archive hook runs before the ZIP is written in the archive service."""

from __future__ import annotations

import tempfile
import threading
import unittest
import zipfile
from pathlib import Path

from project_exporter_desktop.models import ArchiveBuildResult, ExportPaths
from project_exporter_desktop.services.archive_service import build_final_archives


class ArchiveMetadataHookTests(unittest.TestCase):
    def _paths(self, root: Path) -> ExportPaths:
        staging = root / "bundle"
        return ExportPaths(
            desktop=root,
            source_root=root / "source",
            project_name="source",
            bundle_name="source_export_test",
            staging_dir=staging,
            final_zip=root / "source_export_test.zip",
            archive_set_dir=root / "source_export_test_archives",
            project_dir=staging / "source",
            reports_dir=staging / "reports",
            insights_dir=staging / "reports" / "insights",
            manifest_file=staging / "manifest.json",
            project_profile_file=staging / "PROJECT_PROFILE.json",
            index_file=staging / "INDEX.md",
            structure_report=staging / "reports" / "01_structure.txt",
            git_report=staging / "reports" / "02_git.txt",
            text_dump=staging / "reports" / "03_text_dump.txt",
        )

    def test_pre_archive_hook_updates_files_before_zip_is_written(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            paths = self._paths(root)
            paths.project_dir.mkdir(parents=True)
            paths.insights_dir.mkdir(parents=True)
            (paths.staging_dir / "INDEX.md").write_text("index", encoding="utf-8")
            (paths.project_dir / "main.py").write_text("print('ok')", encoding="utf-8")

            def hook(result: ArchiveBuildResult) -> None:
                paths.manifest_file.write_text(
                    f"archive={result.primary_result.name if result.primary_result else 'none'}",
                    encoding="utf-8",
                )

            result = build_final_archives(
                paths,
                include_project=True,
                log=lambda _message: None,
                cancel=threading.Event(),
                part_limit_bytes=10_000_000,
                pre_archive_hook=hook,
            )

            self.assertFalse(result.split)
            with zipfile.ZipFile(paths.final_zip) as archive:
                self.assertIn("manifest.json", archive.namelist())
                manifest = archive.read("manifest.json").decode("utf-8")
            self.assertIn("source_export_test.zip", manifest)


if __name__ == "__main__":
    unittest.main()