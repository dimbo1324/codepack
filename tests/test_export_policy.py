"""Tests for the export policy that decides whether a file is skipped based on safety mode."""

from __future__ import annotations

import unittest
from pathlib import Path

from project_exporter_desktop.services.export_policy import should_skip_file_for_safety


class ExportPolicyTests(unittest.TestCase):
    def test_safe_mode_skips_env_file(self) -> None:
        decision = should_skip_file_for_safety(Path(".env"), "safe")
        self.assertTrue(decision.skip)
        self.assertEqual(decision.severity, "critical")

    def test_safe_mode_keeps_env_example(self) -> None:
        decision = should_skip_file_for_safety(Path(".env.example"), "safe")
        self.assertFalse(decision.skip)

    def test_full_mode_keeps_sensitive_file(self) -> None:
        decision = should_skip_file_for_safety(Path("private.pem"), "full")
        self.assertFalse(decision.skip)


if __name__ == "__main__":
    unittest.main()
