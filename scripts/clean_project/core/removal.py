"""Actually deleting, and the two Windows problems that make it non-trivial."""

from __future__ import annotations

import os
import shutil
import stat
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class RemovalOutcome:
    removed_files: int = 0
    removed_dirs: int = 0
    freed_bytes: int = 0
    failures: list[tuple[str, str]] = field(default_factory=list)


class EscapesRootError(RuntimeError):
    """A path to delete does not resolve to somewhere inside the repository."""


def _force_writable(func, path, _exc):  # type: ignore[no-untyped-def]
    """``shutil.rmtree`` error handler for read-only files.

    Cargo leaves read-only files in ``target/`` and npm does the same in
    ``node_modules``. On Windows a read-only file cannot be unlinked at all, so without
    this a clean fails part-way and leaves the tree in a worse state than it started.
    Clearing the bit and retrying once is the standard fix; a second failure propagates
    to ``remove_all``, which records it and moves on.

    Passed as ``onexc``, which replaced ``onerror`` in Python 3.12. The two differ only
    in the third argument — an exception rather than an ``exc_info`` triple — and this
    handler never used it.
    """
    os.chmod(path, stat.S_IWRITE)
    func(path)


def _inside(root: Path, relative: str) -> Path:
    """Prove ``relative`` names something inside ``root``, and return it **unresolved**.

    Defence in depth. The candidates come from ``git status``, which does not emit
    absolute paths or ``..`` segments — but this is the one function in the repository
    that deletes trees, and "the input is currently trustworthy" is a property of today's
    caller, not of this function. ``Path("/a") / "C:/x"`` is ``C:/x``: one absolute string
    reaching here would silently retarget the deletion outside the project.

    Only the **parent** is resolved, and the returned path keeps the final component as
    written. Resolving the whole thing would follow a symlink to its destination, and the
    caller's ``is_symlink()`` guard — the thing that stops ``rmtree`` from walking through
    a link and deleting somebody else's tree — would then always see ``False``. Checking
    the parent is what the guard needs anyway: it proves the name being unlinked lives in
    a directory inside the repository, which says nothing about, and needs nothing from,
    where a link at that name happens to point.
    """
    target = root / relative
    root_resolved = root.resolve()
    if target.resolve() == root_resolved:
        raise EscapesRootError("refusing to delete the repository root itself")
    parent = target.parent.resolve()
    if parent != root_resolved and root_resolved not in parent.parents:
        raise EscapesRootError(
            f"{relative!r} sits in {parent}, which is outside {root_resolved}"
        )
    return target


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
        try:
            target = _inside(root, relative)
        except EscapesRootError as error:
            outcome.failures.append((relative, str(error)))
            continue
        if not target.exists():
            # Removing a parent directory can take a child listed separately; that is
            # success, not a failure worth reporting.
            continue
        size = _tree_size(target)
        try:
            if target.is_dir() and not target.is_symlink():
                shutil.rmtree(target, onexc=_force_writable)
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
    protected_trees: list[str] | None = None,
) -> list[str]:
    """Empty directories to delete, deleted unless ``dry_run``.

    Git does not track directories, so one left empty by a deletion or a move never
    goes away on its own. Repeating matters because removing the innermost directory is
    what makes its parent empty — a single pass would leave the nest half-collapsed.

    ``protection`` is consulted here too. It used to be skipped, so an empty ``.idea/``
    was pruned while the plan listed it as protected: the module promises the protected
    list wins over everything, and one of the two deletion phases did not honour it.

    ``protected_trees`` carries the verdicts ``discover`` reached that the pattern list
    alone cannot reproduce — a directory protected because of what it *holds*, or because
    it is a nested repository. Consulting only the patterns still pruned empty directories
    inside a sibling clone the same run had just refused to touch, which contradicts the
    "protected whole" the plan and the docs promise.

    ``dry_run`` simulates instead of deleting, so the plan can name these paths. Without
    it the second phase removed directories the dry run never showed, which defeats the
    review step the whole script is built around.
    """
    skip_set = set(skip)
    reserved = tuple(f"{tree.rstrip('/')}/" for tree in (protected_trees or []))
    reserved_exact = {tree.rstrip("/") for tree in (protected_trees or [])}
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
            # `topdown=True` so `dirs` can be pruned before descending: without that,
            # every pass walked all of `node_modules` and `target` in full and then threw
            # each result away against `skip_set`. The consideration pass below still
            # needs children before parents, so the walk is materialised and reversed.
            #
            # What is recorded is the **unpruned** child list. `is_empty` reads it, and a
            # directory whose only child is a skipped one is not empty — pruning first
            # would have reported it as safe to remove, which in a dry run is a plan that
            # does not match what an apply would do.
            walked: list[tuple[str, list[str], list[str]]] = []
            for current, dirs, files in os.walk(base, topdown=True, onerror=lambda _e: None):
                walked.append((current, list(dirs), files))
                dirs[:] = [d for d in dirs if d not in skip_set]

            for current, dirs, files in reversed(walked):
                current_path = Path(current)
                if current_path == base or current_path in simulated:
                    continue
                relative = current_path.relative_to(root)
                if skip_set & set(relative.parts):
                    continue
                if protection is not None and protection.is_protected(str(relative)):
                    continue
                posix = str(relative).replace("\\", "/")
                if posix in reserved_exact or posix.startswith(reserved):
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
