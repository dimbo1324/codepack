from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from project_exporter_desktop.services.export_ignore import ExportIgnoreRules


class ExportIgnoreRulesTests(unittest.TestCase):
    def test_exportignore_supports_dir_file_extension_and_include(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / ".exportignore").write_text(
                "private/\n*.log\n*.db\n!important.log\n!docs/\n", encoding="utf-8"
            )
            rules = ExportIgnoreRules.from_project_and_config(root, excluded_extensions=["sqlite"])

            self.assertTrue(rules.should_skip_dir(Path("private"))[0])
            self.assertTrue(rules.should_skip_file(Path("logs/app.log"))[0])
            self.assertFalse(rules.should_skip_file(Path("important.log"))[0])
            self.assertTrue(rules.should_skip_file(Path("data/main.sqlite"))[0])
            self.assertFalse(rules.should_skip_dir(Path("docs"))[0])


if __name__ == "__main__":
    unittest.main()