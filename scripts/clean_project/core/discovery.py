"""Finding what git does not track, and sorting it into artifacts and everything else.

``git`` is the authority on what is tracked — reimplementing ``.gitignore`` semantics in
Python would be both wrong and unnecessary.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from fnmatch import fnmatch
from pathlib import Path

from scripts._toolkit.processes import capture

#: ``git clean -xdn`` cannot emit NUL-separated output (no ``-z`` switch), and this list
#: decides what gets deleted — so a path containing a newline or a quote must not be
#: able to shift a parse. ``git status`` does support ``-z``, and asked this way it
#: reports the same set: untracked (``??``) plus ignored (``!!``), with directories
#: collapsed exactly as ``-d`` would collapse them.
_LIST_ARGV = [
    "git",
    "status",
    "--porcelain",
    "-z",
    "--ignored=traditional",
    "--untracked-files=normal",
]

_UNTRACKED = "??"
_IGNORED = "!!"


@dataclass(frozen=True)
class Candidate:
    relative: str
    is_dir: bool
    artifact: bool
    protected: bool
    reason: str = ""

    @property
    def kind(self) -> str:
        if self.protected:
            return "protected"
        return "artifact" if self.artifact else "other"


class DiscoveryError(RuntimeError):
    """git could not report the working tree state."""


def discover(root: Path, artifacts: list[str], protection) -> list[Candidate]:
    code, output = capture(_LIST_ARGV, root)
    if code == 127:
        raise DiscoveryError("git is not on PATH, so nothing can be classified safely")
    if code != 0:
        raise DiscoveryError(f"`git status` failed with exit {code}: {output.strip()}")

    candidates: list[Candidate] = []
    for entry in output.split("\0"):
        if len(entry) < 4:
            continue
        status, path = entry[:2], entry[3:]
        if status not in (_UNTRACKED, _IGNORED):
            continue

        relative = path.rstrip("/")
        if not relative:
            continue

        absolute = root / relative
        is_dir = absolute.is_dir()

        protected = protection.is_protected(relative)
        reason = "protected pattern" if protected else ""

        # git reports a wholly untracked or ignored directory as ONE entry and never
        # lists what is inside it. Checking only the entry's own name therefore asks the
        # protection list about `certs/` while `shutil.rmtree` deletes `certs/.env` —
        # the plan printed a path as protected and the same run destroyed it. So a
        # directory candidate is judged by its contents, not by its name.
        if not protected and is_dir:
            sheltered = _first_protected_inside(root, absolute, protection)
            if sheltered is not None:
                protected = True
                reason = f"holds {sheltered}"

        candidates.append(
            Candidate(
                relative=relative,
                is_dir=is_dir,
                artifact=_is_artifact(relative, artifacts),
                protected=protected,
                reason=reason,
            )
        )

    candidates.sort(key=lambda c: c.relative)
    return candidates


def _first_protected_inside(root: Path, directory: Path, protection) -> str | None:
    """The first protected path, or nested repository, found anywhere under ``directory``.

    A directory that shelters something protected is itself protected, whole. Deleting
    the rest of it and keeping the one file would be a partial result nobody asked for,
    from a plan that listed the directory as a single line — declining and naming what
    stopped it leaves the decision with the person who can make it.

    Symlinks are not followed: a link pointing outside the tree would otherwise decide
    the fate of a directory inside it.
    """
    nested_repo = _find_nested_repo(directory)
    if nested_repo is not None:
        return f"nested git repository {_as_relative(root, nested_repo)}"

    for current, dirs, files in os.walk(directory, onerror=lambda _e: None):
        current_path = Path(current)
        dirs[:] = [d for d in dirs if not (current_path / d).is_symlink()]
        for name in files + dirs:
            relative = _as_relative(root, current_path / name)
            if protection.is_protected(relative):
                return relative
    return None


def _find_nested_repo(directory: Path) -> Path | None:
    """A ``.git`` anywhere beneath ``directory``, not only at its top level.

    An untracked `vendor/` holding `vendor/sibling/.git` reports to git as `vendor/`
    alone, so a top-level-only check saw no repository and queued the whole thing for
    deletion — losing an unpushed sibling clone, the very case this guard exists for.
    `git clean` refuses such a directory without `-ff`; so does this.
    """
    if (directory / ".git").exists():
        return directory / ".git"
    for current, dirs, _files in os.walk(directory, onerror=lambda _e: None):
        current_path = Path(current)
        dirs[:] = [d for d in dirs if not (current_path / d).is_symlink()]
        if ".git" in dirs or (current_path / ".git").exists():
            return current_path / ".git"
    return None


def _as_relative(root: Path, path: Path) -> str:
    try:
        return str(path.relative_to(root)).replace("\\", "/")
    except ValueError:
        return str(path).replace("\\", "/")


def _is_artifact(relative: str, artifacts: list[str]) -> bool:
    normalized = relative.replace("\\", "/").rstrip("/")
    for pattern in artifacts:
        target = pattern.rstrip("/")
        if normalized == target or normalized.startswith(target + "/"):
            return True
        # A bare name like `__pycache__` should match at any depth, which is where
        # Python and tool caches actually appear.
        if "/" not in target and (
            target in normalized.split("/") or fnmatch(normalized, target)
        ):
            return True
    return False
