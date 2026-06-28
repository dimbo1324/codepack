from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from project_exporter_desktop.config import Config
from project_exporter_desktop.services.export_ignore import ExportIgnoreRules
from project_exporter_desktop.services.export_plan import build_export_plan
from project_exporter_desktop.services.git_diff import DiffSelection
from project_exporter_desktop.services.incremental import IncrementalSelection


class ExportPlanTests(unittest.TestCase):
    def test_plan_marks_safe_sensitive_files_as_excluded(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "src").mkdir()
            (root / "src" / "main.py").write_text("print(1)", encoding="utf-8")
            (root / ".env").write_text("TOKEN=secret", encoding="utf-8")
            cfg = Config(last_root=str(root), safe_export_mode="safe")
            plan = build_export_plan(
                root,
                cfg,
                cfg.effective_ignored_dirs(),
                DiffSelection(mode="all", paths=None),
                IncrementalSelection(enabled=False),
                ExportIgnoreRules.from_project_and_config(root),
            )
            included = {item.relative_path for item in plan.included_files}
            excluded = {item.relative_path for item in plan.excluded_files}
            self.assertIn("src\\main.py", included)
            self.assertIn(".env", excluded)


if __name__ == "__main__":
    unittest.main()