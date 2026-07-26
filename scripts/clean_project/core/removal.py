"""Actually deleting, and the two Windows problems that make it non-trivial."""

from __future__ import annotations

import os
import shutil
import stat
from dataclasses import dataclass
from pathlib import Path


@dataclass
class RemovalOutcome:
    removed_files: int = 0
    removed_dirs: int = 0
    freed_bytes: int = 0
    failures: list[tuple[str, str]] = None  # type: ignore[assignment]

    def __post_init__(self) -> None:
        if self.failures is None:
            self.failures = []


def _force_writable(func, path, _exc_info):  # type: ignore[no-untyped-def]
    """``shutil.rmtree`` error handler for read-only files.

    Cargo leaves read-only files in ``target/`` and npm does the same in
    ``node_modules``. On Windows a read-only file cannot be unlinked at all, so without
    this a clean fails part-way and leaves the tree in a worse state than it started.
    Clearing the bit and retrying once is the standard fix; a second failure is a real
    error and propagates.
    """
    try:
        os.chmod(path, stat.S_IWRITE)
        func(path)
    except OSError:
        raise


def _tree_size(path: Path) -> int:
    """Bytes under ``path``. Best effort: a file that disappears mid-walk, or that the
    OS will not stat, must not abort a size estimate."""
    if path.is_file():
        try:
            return path.stat().st_size
        except OSError:
            return 0
    total = 0
    for current, _dirs, files in os.walk(path, onerror=lambda _e: None):
        for name in files:
            try:
                total += (Path(current) / name).stat().st_size
            except OSError:
                continue
    return total


def remove_all(root: Path, relatives: list[str]) -> RemovalOutcome:
    outcome = RemovalOutcome()
    for relative in relatives:
        target = root / relative
        if not target.exists():
            # Removing a parent directory can take a child listed separately; that is
            # success, not a failure worth reporting.
            continue
        size = _tree_size(target)
        try:
            if target.is_dir() and not target.is_symlink():
                shutil.rmtree(target, onerror=_force_writable)
                outcome.removed_dirs += 1
            else:
                try:
                    target.unlink()
                except PermissionError:
                    os.chmod(target, stat.S_IWRITE)
                    target.unlink()
                outcome.removed_files += 1
            outcome.freed_bytes += size
        except OSError as error:
            # A locked file (an editor, a running build, antivirus) is the common case
            # on Windows. Report it and keep going: a partial clean that names what it
            # could not touch is more useful than one that stops at the first lock.
            outcome.failures.append((relative, str(error)))
    return outcome


def prune_empty_dirs(
    root: Path,
    roots: list[str],
    skip: list[str],
    protection=None,
    dry_run: bool = False,
) -> list[str]:
    """Empty directories to delete, deleted unless ``dry_run``.

    Git does not track directories, so one left empty by a deletion or a move never
    goes away on its own. Repeating matters because removing the innermost directory is
    what makes its parent empty — a single pass would leave the nest half-collapsed.

    ``protection`` is consulted here too. It used to be skipped, so an empty ``.idea/``
    was pruned while the plan listed it as protected: the module promises the protected
    list wins over everything, and one of the two deletion phases did not honour it.

    ``dry_run`` simulates instead of deleting, so the plan can name these paths. Without
    it the second phase removed directories the dry run never showed, which defeats the
    review step the whole script is built around.
    """
    skip_set = set(skip)
    removed: list[str] = []
    simulated: set[Path] = set()

    def is_empty(path: Path, dirs: list[str], files: list[str]) -> bool:
        if files:
            return False
        return all((path / d) in simulated for d in dirs)

    while True:
        removed_this_pass = False
        for start in roots:
            base = (root / start).resolve()
            if not base.is_dir():
                continue
            # topdown=False: children before parents, so a parent that becomes empty is
            # considered in the same pass.
            for current, dirs, files in os.walk(base, topdown=False, onerror=lambda _e: None):
                current_path = Path(current)
                if current_path == base or current_path in simulated:
                    continue
                relative = current_path.relative_to(root)
                if skip_set & set(relative.parts):
                    continue
                if protection is not None and protection.is_protected(str(relative)):
                    continue
                if not is_empty(current_path, dirs, files):
                    continue
                if dry_run:
                    simulated.add(current_path)
                else:
                    try:
                        current_path.rmdir()
                    except OSError:
                        continue
                removed.append(str(relative).replace("\\", "/"))
                removed_this_pass = True
        if not removed_this_pass:
            break

    return sorted(removed)
