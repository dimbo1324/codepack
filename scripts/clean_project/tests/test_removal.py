"""Tests for the one function in this repository that deletes trees.

`test_protection.py` covers which paths are spared and `test_discovery.py` covers how
the plan is built. These cover what happens when the plan is carried out.
"""

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path

from scripts.clean_project.core.removal import EscapesRootError, _inside, remove_all


class ContainmentTest(unittest.TestCase):
    """Defence in depth. Candidates come from `git status`, which emits neither absolute
    paths nor `..` — but that is a property of today's caller, not of this function."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name).resolve()

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_an_ordinary_relative_path_is_accepted(self) -> None:
        (self.root / "build").mkdir()
        self.assertEqual(_inside(self.root, "build"), self.root / "build")

    def test_an_absolute_path_is_refused(self) -> None:
        # `Path("/a") / "C:/x"` is `C:/x`: one absolute string reaching here would
        # silently retarget the deletion outside the project.
        outside = Path(tempfile.gettempdir()).resolve() / "codepack-elsewhere"
        with self.assertRaises(EscapesRootError):
            _inside(self.root, str(outside))

    def test_a_parent_traversal_is_refused(self) -> None:
        with self.assertRaises(EscapesRootError):
            _inside(self.root, os.path.join("..", "sibling"))

    def test_the_repository_root_itself_is_refused(self) -> None:
        with self.assertRaises(EscapesRootError):
            _inside(self.root, ".")

    def test_a_refused_path_is_reported_and_does_not_stop_the_rest(self) -> None:
        (self.root / "build").mkdir()
        outcome = remove_all(self.root, ["..", "build"])
        self.assertEqual(len(outcome.failures), 1)
        self.assertFalse((self.root / "build").exists())


class ReadOnlyFileTest(unittest.TestCase):
    """Cargo leaves read-only files in `target/` and npm does the same in
    `node_modules`. On Windows a read-only file cannot be unlinked at all."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name).resolve()

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_a_read_only_file_inside_a_tree_is_still_removed(self) -> None:
        import stat

        tree = self.root / "target"
        tree.mkdir()
        locked = tree / "locked.bin"
        locked.write_text("x")
        os.chmod(locked, stat.S_IREAD)

        outcome = remove_all(self.root, ["target"])
        self.assertEqual(outcome.failures, [])
        self.assertFalse(tree.exists())

    def test_a_fresh_outcome_has_its_own_failure_list(self) -> None:
        # A mutable default shared between instances is the classic version of this bug.
        from scripts.clean_project.core.removal import RemovalOutcome

        first, second = RemovalOutcome(), RemovalOutcome()
        first.failures.append(("a", "b"))
        self.assertEqual(second.failures, [])


if __name__ == "__main__":
    unittest.main()
