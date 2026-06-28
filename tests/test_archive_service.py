from __future__ import annotations

import tempfile
import threading
import unittest
import zipfile
from pathlib import Path

from project_exporter_desktop.models import ExportPaths
from project_exporter_desktop.services.archive_service import build_final_archives


class ArchiveServiceTests(unittest.TestCase):
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

    def test_single_archive_created_under_limit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            paths = self._paths(root)
            paths.project_dir.mkdir(parents=True)
            paths.reports_dir.mkdir(parents=True)
            (paths.staging_dir / "INDEX.md").write_text("index", encoding="utf-8")
            (paths.project_dir / "main.py").write_text("print('ok')", encoding="utf-8")
            logs: list[str] = []

            result = build_final_archives(
                paths, True, logs.append, threading.Event(), part_limit_bytes=10_000_000
            )

            self.assertFalse(result.split)
            self.assertTrue(paths.final_zip.exists())
            self.assertEqual(len(result.archives), 1)

    def test_split_archive_writes_restore_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            paths = self._paths(root)
            paths.project_dir.mkdir(parents=True)
            paths.reports_dir.mkdir(parents=True)
            (paths.staging_dir / "INDEX.md").write_text("index", encoding="utf-8")
            (paths.project_dir / "main.py").write_text("print('ok')\n", encoding="utf-8")
            logs: list[str] = []

            result = build_final_archives(
                paths, True, logs.append, threading.Event(), part_limit_bytes=1
            )

            self.assertTrue(result.split)
            self.assertTrue((paths.archive_set_dir / "ARCHIVE_SET_MANIFEST.json").exists())
            restore_script = paths.archive_set_dir / "restore_archives.py"
            self.assertTrue(restore_script.exists())
            self.assertIn("def main()", restore_script.read_text(encoding="utf-8"))

    def test_reports_only_archive_skips_project_tree(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            paths = self._paths(root)
            paths.project_dir.mkdir(parents=True)
            paths.reports_dir.mkdir(parents=True)
            (paths.staging_dir / "INDEX.md").write_text("index", encoding="utf-8")
            (paths.project_dir / "main.py").write_text("print('ok')\n", encoding="utf-8")
            (paths.reports_dir / "summary.txt").write_text("summary", encoding="utf-8")

            result = build_final_archives(
                paths, False, lambda _message: None, threading.Event(), part_limit_bytes=10_000_000
            )

            self.assertEqual(result.skipped_project_files, 1)
            with zipfile.ZipFile(paths.final_zip) as archive:
                names = set(archive.namelist())
            self.assertIn("INDEX.md", names)
            self.assertIn("reports/summary.txt", names)
            self.assertNotIn("source/main.py", names)


if __name__ == "__main__":
    unittest.main()
