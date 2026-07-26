"""Finding what git does not track, and sorting it into artifacts and everything else.

``git`` is the authority on what is tracked — reimplementing ``.gitignore`` semantics in
Python would be both wrong and unnecessary.
"""

from __future__ import annotations

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

        # A nested repository is somebody else's history. `git clean` refuses to touch
        # one without -ff for exactly this reason, and losing an unpushed sibling repo
        # would be unrecoverable — so it is protected here regardless of the pattern
        # list, which a reader of clean.json cannot be expected to have anticipated.
        if is_dir and (absolute / ".git").exists():
            protected = True
            reason = "nested git repository"

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
