"""Tests for validate_source_root: rejects blank paths and files, accepts valid project directories."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from project_exporter_desktop.utils.path_utils import validate_source_root


class PathValidationTests(unittest.TestCase):
    def test_rejects_empty_path(self) -> None:
        with self.assertRaises(ValueError):
            validate_source_root("   ")

    def test_accepts_existing_project_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "project"
            source.mkdir()
            self.assertEqual(validate_source_root(str(source)), source.resolve())

    def test_rejects_file_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            file_path = Path(tmp) / "not_a_directory.txt"
            file_path.write_text("x", encoding="utf-8")
            with self.assertRaises(ValueError):
                validate_source_root(str(file_path))


if __name__ == "__main__":
    unittest.main()