"""Tests for the protected-path rules — the safety boundary of clean-project.

Uses stdlib ``unittest`` deliberately: the project has no Python test runner as a
dependency, and the one piece of logic standing between "clean the tree" and "delete a
developer's live credentials" must not be untested for want of one.

Run with:  python -m unittest discover -s scripts -t .
"""

from __future__ import annotations

import unittest

from scripts.clean_project.core.protection import ProtectionRules

_PATTERNS = [
    ".env",
    ".env.*",
    "!.env.example",
    "*.local.json",
    ".vscode/",
    "*.pem",
    "codepack.db",
]


class ProtectionRulesTest(unittest.TestCase):
    def setUp(self) -> None:
        self.rules = ProtectionRules(_PATTERNS)

    def assert_protected(self, path: str) -> None:
        self.assertTrue(self.rules.is_protected(path), f"{path} must be protected")

    def assert_cleanable(self, path: str) -> None:
        self.assertFalse(self.rules.is_protected(path), f"{path} must be cleanable")

    def test_dotenv_and_its_variants_are_protected(self) -> None:
        for path in (".env", ".env.production", ".env.local"):
            self.assert_protected(path)

    def test_a_leading_dot_is_not_stripped_away(self) -> None:
        """Regression: ``lstrip("./")`` strips *characters*, so ``.env`` became ``env``
        and stopped matching. A sandbox run caught a planted ``.env`` listed for
        deletion — the exact outcome this module exists to prevent."""
        self.assert_protected(".env")
        self.assert_cleanable("environment.txt")

    def test_reinclusion_lets_the_example_file_be_cleaned(self) -> None:
        self.assert_cleanable(".env.example")

    def test_patterns_match_at_any_depth(self) -> None:
        for path in ("config/.env", "deep/nested/.env", "a/b/settings.local.json"):
            self.assert_protected(path)

    def test_a_directory_pattern_covers_everything_inside_it(self) -> None:
        self.assert_protected(".vscode")
        self.assert_protected(".vscode/settings.json")

    def test_windows_separators_normalize(self) -> None:
        self.assert_protected(r"config\.env")
        self.assert_protected(r".vscode\settings.json")

    def test_ordinary_sources_and_build_output_stay_cleanable(self) -> None:
        for path in ("src/main.rs", "target", "README.md", "src/wip.rs", "node_modules"):
            self.assert_cleanable(path)

    def test_reinclusion_order_in_the_config_does_not_matter(self) -> None:
        """Re-inclusions are checked before protections on purpose: a reader of
        clean.json should not have to reason about precedence to know whether their
        secrets are safe."""
        reversed_rules = ProtectionRules(list(reversed(_PATTERNS)))
        self.assertFalse(reversed_rules.is_protected(".env.example"))
        self.assertTrue(reversed_rules.is_protected(".env"))


if __name__ == "__main__":
    unittest.main()
