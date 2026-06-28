"""Tests for the copy service: safe-mode filtering, diff-based selection, and always-include rules."""

from __future__ import annotations

import tempfile
import threading
import unittest
from pathlib import Path

from project_exporter_desktop.services.copy_service import copy_project
from project_exporter_desktop.services.export_ignore import ExportIgnoreRules


class CopyServiceTests(unittest.TestCase):
    def test_copy_skips_node_modules_and_env_in_safe_mode(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source"
            dest = Path(tmp) / "dest"
            (source / "node_modules").mkdir(parents=True)
            (source / "node_modules" / "x.js").write_text("x", encoding="utf-8")
            (source / "src").mkdir()
            (source / "src" / "main.py").write_text("print('ok')", encoding="utf-8")
            (source / ".env").write_text("TOKEN=secret", encoding="utf-8")

            logs: list[str] = []
            stats = copy_project(
                source,
                dest,
                extra_ignored_dirs=frozenset({"node_modules"}),
                log=logs.append,
                cancel=threading.Event(),
                safe_export_mode="safe",
            )

            self.assertEqual(stats.files_copied, 1)
            self.assertEqual(stats.files_skipped_by_safety, 1)
            self.assertTrue((dest / "src" / "main.py").exists())
            self.assertFalse((dest / ".env").exists())
            self.assertFalse((dest / "node_modules" / "x.js").exists())

    def test_copy_with_diff_selection_prunes_unselected_siblings(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source"
            dest = Path(tmp) / "dest"
            (source / "src" / "deep").mkdir(parents=True)
            (source / "src" / "deep" / "selected.py").write_text("print(1)", encoding="utf-8")
            (source / "src" / "deep" / "other.py").write_text("print(2)", encoding="utf-8")
            (source / "src" / "sibling.py").write_text("print(3)", encoding="utf-8")

            stats = copy_project(
                source,
                dest,
                extra_ignored_dirs=frozenset(),
                log=lambda _message: None,
                cancel=threading.Event(),
                safe_export_mode="balanced",
                include_relative_paths=frozenset({"src\\deep\\selected.py"}),
            )

            self.assertEqual(stats.files_copied, 1)
            self.assertTrue((dest / "src" / "deep" / "selected.py").exists())
            self.assertFalse((dest / "src" / "deep" / "other.py").exists())
            self.assertFalse((dest / "src" / "sibling.py").exists())

    def test_copy_honors_always_include_inside_exportignored_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source"
            dest = Path(tmp) / "dest"
            source.mkdir()
            (source / ".exportignore").write_text(
                "private/\n!private/keep/allowed.txt\n", encoding="utf-8"
            )
            (source / "private" / "keep").mkdir(parents=True)
            (source / "private" / "drop").mkdir(parents=True)
            (source / "private" / "keep" / "allowed.txt").write_text("ok", encoding="utf-8")
            (source / "private" / "keep" / "other.txt").write_text("no", encoding="utf-8")
            (source / "private" / "drop" / "secret.txt").write_text("no", encoding="utf-8")
            rules = ExportIgnoreRules.from_project_and_config(source)

            stats = copy_project(
                source,
                dest,
                extra_ignored_dirs=frozenset(),
                log=lambda _message: None,
                cancel=threading.Event(),
                safe_export_mode="balanced",
                export_rules=rules,
            )

            self.assertEqual(stats.files_copied, 2)
            self.assertTrue((dest / ".exportignore").exists())
            self.assertTrue((dest / "private" / "keep" / "allowed.txt").exists())
            self.assertFalse((dest / "private" / "keep" / "other.txt").exists())
            self.assertFalse((dest / "private" / "drop" / "secret.txt").exists())


if __name__ == "__main__":
    unittest.main()
