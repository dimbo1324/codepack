"""Tests for the incremental export service that detects added and modified files since last baseline."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from project_exporter_desktop.services import incremental


class IncrementalTests(unittest.TestCase):
    def test_incremental_detects_added_and_modified_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "project"
            root.mkdir()
            state = Path(tmp) / "state.json"
            with patch.object(incremental, "STATE_FILE", state):
                (root / "a.txt").write_text("one", encoding="utf-8")
                incremental.save_incremental_baseline(root, frozenset())
                (root / "a.txt").write_text("modified content", encoding="utf-8")
                (root / "b.txt").write_text("new", encoding="utf-8")
                selection = incremental.resolve_incremental_selection(root, frozenset(), True)
                self.assertIn("a.txt", selection.modified)
                self.assertIn("b.txt", selection.added)
                self.assertEqual(selection.paths, frozenset({"a.txt", "b.txt"}))


if __name__ == "__main__":
    unittest.main()