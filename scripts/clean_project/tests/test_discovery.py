"""Tests for what clean-project decides to delete.

`test_protection.py` covers the pattern rules in isolation. These tests cover the gap
that made those rules insufficient: git reports a wholly untracked or ignored directory
as ONE entry and never says what is inside, so asking the protection list about the
entry's own name protected `certs/` while `shutil.rmtree` deleted `certs/.env`. The plan
printed a path as protected and the same run destroyed it.

That cannot be caught by testing patterns against strings — it needs a real repository
with real files, which is what these build.

Run with:  python -m unittest discover -s scripts -t .
"""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.clean_project.core.discovery import discover
from scripts.clean_project.core.protection import ProtectionRules
from scripts.clean_project.core.removal import prune_empty_dirs

_PATTERNS = [".env", ".env.*", "!.env.example", "*.pem", ".vscode/", "codepack.db"]


def _git(root: Path, *args: str) -> None:
    subprocess.run(
        ["git", *args],
        cwd=root,
        check=True,
        capture_output=True,
        env={**os.environ, "GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_SYSTEM": os.devnull},
    )


def _write(path: Path, text: str = "x\n") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class DiscoveryTest(unittest.TestCase):
    """Each test gets its own repository: these assertions are about git's own view of
    the tree, so a shared fixture would let one test's leftovers decide another's
    result."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name).resolve()
        _git(self.root, "init", "-q", ".")
        _git(self.root, "config", "user.email", "test@example.invalid")
        _git(self.root, "config", "user.name", "test")
        _write(self.root / "README.md")
        _write(self.root / ".gitignore", ".env\n.env.*\nignored_dir/\n")
        _git(self.root, "add", "README.md", ".gitignore")
        _git(self.root, "commit", "-qm", "init")
        self.rules = ProtectionRules(_PATTERNS)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def plan(self) -> dict[str, tuple[bool, str]]:
        return {c.relative: (c.protected, c.reason) for c in discover(self.root, [], self.rules)}

    def assert_protected(self, relative: str) -> str:
        plan = self.plan()
        self.assertIn(relative, plan, f"{relative} was not reported at all: {sorted(plan)}")
        protected, reason = plan[relative]
        self.assertTrue(protected, f"{relative} was queued for deletion")
        return reason

    def assert_cleanable(self, relative: str) -> None:
        protected, _ = self.plan().get(relative, (False, ""))
        self.assertFalse(protected, f"{relative} should have been cleanable")

    def test_a_bare_untracked_secret_is_protected(self) -> None:
        _write(self.root / ".env", "SECRET=live\n")
        self.assert_protected(".env")

    def test_an_untracked_directory_holding_a_secret_is_protected_whole(self) -> None:
        """The original defect: git collapses `certs/` to one entry, the name is not in
        the pattern list, and rmtree took the secret with it."""
        _write(self.root / "certs" / ".env", "DB_PASS=live\n")
        reason = self.assert_protected("certs")
        self.assertIn("certs/.env", reason)

    def test_an_ignored_directory_holding_a_secret_is_protected_whole(self) -> None:
        _write(self.root / "ignored_dir" / ".env", "SECRET=live\n")
        self.assert_protected("ignored_dir")

    def test_signing_material_at_depth_protects_its_directory(self) -> None:
        _write(self.root / "release" / "keys" / "signing.pem")
        reason = self.assert_protected("release")
        self.assertIn("signing.pem", reason)

    def test_a_local_database_protects_its_directory(self) -> None:
        _write(self.root / "state" / "codepack.db")
        self.assert_protected("state")

    def test_a_nested_repository_at_the_top_level_is_protected(self) -> None:
        nested = self.root / "sibling"
        nested.mkdir()
        _git(nested, "init", "-q", ".")
        reason = self.assert_protected("sibling")
        self.assertIn("nested git repository", reason)

    def test_a_nested_repository_below_the_reported_entry_is_protected(self) -> None:
        """`vendor/` reports as one entry, so a top-level-only `.git` check saw nothing
        and queued an unpushed sibling clone for deletion."""
        nested = self.root / "vendor" / "sibling"
        nested.mkdir(parents=True)
        _git(nested, "init", "-q", ".")
        reason = self.assert_protected("vendor")
        self.assertIn("nested git repository", reason)

    def test_ordinary_untracked_output_is_still_cleanable(self) -> None:
        """The protection must not become so broad that the script stops doing its job."""
        _write(self.root / "target" / "debug" / "app.exe")
        _write(self.root / "scratch.txt")
        self.assert_cleanable("target")
        self.assert_cleanable("scratch.txt")

    def test_a_reincluded_example_file_does_not_protect_its_directory(self) -> None:
        _write(self.root / "samples" / ".env.example")
        self.assert_cleanable("samples")


class PruneEmptyDirsTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name).resolve()
        self.rules = ProtectionRules(_PATTERNS)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_an_empty_directory_is_pruned(self) -> None:
        (self.root / "empty").mkdir()
        self.assertEqual(prune_empty_dirs(self.root, ["."], [], self.rules), ["empty"])
        self.assertFalse((self.root / "empty").exists())

    def test_a_nest_collapses_in_one_call(self) -> None:
        (self.root / "a" / "b" / "c").mkdir(parents=True)
        self.assertEqual(prune_empty_dirs(self.root, ["."], [], self.rules), ["a", "a/b", "a/b/c"])
        self.assertFalse((self.root / "a").exists())

    def test_a_protected_directory_is_not_pruned(self) -> None:
        """Pruning used to ignore the protection list entirely, so an empty `.vscode/`
        was deleted while the plan printed it as protected."""
        (self.root / ".vscode").mkdir()
        self.assertEqual(prune_empty_dirs(self.root, ["."], [], self.rules), [])
        self.assertTrue((self.root / ".vscode").exists())

    def test_a_directory_with_content_survives(self) -> None:
        _write(self.root / "keep" / "file.txt")
        self.assertEqual(prune_empty_dirs(self.root, ["."], [], self.rules), [])
        self.assertTrue((self.root / "keep" / "file.txt").exists())

    def test_dry_run_reports_the_same_paths_and_deletes_nothing(self) -> None:
        """The plan has to be able to name these: `--apply` used to prune directories the
        dry run never showed, which defeats the review step the script is built on."""
        (self.root / "a" / "b").mkdir(parents=True)
        (self.root / ".vscode").mkdir()
        reported = prune_empty_dirs(self.root, ["."], [], self.rules, dry_run=True)
        self.assertEqual(reported, ["a", "a/b"])
        self.assertTrue((self.root / "a" / "b").exists())
        self.assertEqual(prune_empty_dirs(self.root, ["."], [], self.rules), reported)

    def test_the_skip_list_still_applies(self) -> None:
        (self.root / ".git" / "objects").mkdir(parents=True)
        self.assertEqual(prune_empty_dirs(self.root, ["."], [".git"], self.rules), [])
        self.assertTrue((self.root / ".git" / "objects").exists())


if __name__ == "__main__":
    unittest.main()
