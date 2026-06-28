"""Tests for text_utils: safe file reading with byte limits and secret redaction."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from project_exporter_desktop.utils.text_utils import read_text_safely, redact_secrets


class TextUtilsTests(unittest.TestCase):
    def test_read_text_safely_unlimited(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "sample.txt"
            path.write_text("hello", encoding="utf-8")
            text, info = read_text_safely(path, max_bytes=None)
            self.assertEqual(text, "hello")
            self.assertIn(info, {"utf-8", "utf-8-sig", "latin-1"})

    def test_read_text_safely_limit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "sample.txt"
            path.write_text("hello", encoding="utf-8")
            text, info = read_text_safely(path, max_bytes=2)
            self.assertIsNone(text)
            self.assertTrue(info.startswith("too-large"))

    def test_redact_secrets(self) -> None:
        redacted = redact_secrets("API_KEY=super-secret-value")
        self.assertIn("<REDACTED>", redacted)
        self.assertNotIn("super-secret-value", redacted)


if __name__ == "__main__":
    unittest.main()