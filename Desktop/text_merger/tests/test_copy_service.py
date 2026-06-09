from __future__ import annotations

import tempfile
import threading
import unittest
from pathlib import Path

from project_exporter_desktop.services.copy_service import copy_project


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


if __name__ == "__main__":
    unittest.main()
