"""Finding what git does not track, and sorting it into artifacts and everything else.

``git`` is the authority on what is tracked — reimplementing ``.gitignore`` semantics in
Python would be both wrong and unnecessary.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from fnmatch import fnmatch
from pathlib import Path

from scripts._toolkit.processes import NOT_FOUND, TIMED_OUT, capture_bytes

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

#: Far longer than the toolkit's default. This is not a version probe: it walks the whole
#: working tree including ignored paths, and on Windows an antivirus scanner inspecting a
#: cold `node_modules` or a kernel-sized checkout makes minutes ordinary. The timeout is
#: here to stop an outright hang, not to bound normal work — cutting a slow-but-healthy
#: run short would turn a working clean into a refusal.
_LIST_TIMEOUT = 600.0

#: Git warnings this script knows the cure for, mapped to the cure. A fail-closed
#: refusal that does not say what to do next is a dead end: the owner hit exactly this,
#: with pnpm's deeply nested store tripping Windows' 260-character path limit, and the
#: message said only "resolve these first".
_KNOWN_WARNINGS = (
    (
        "filename too long",
        "Windows caps paths at 260 characters and git refuses to look past that. "
        "Enable long paths for this clone:\n"
        "    git config core.longpaths true\n"
        "  (Deeply nested pnpm and cargo directories are the usual trigger.)",
    ),
    (
        "permission denied",
        "Something in the tree cannot be read by this user. Close anything holding it, "
        "or run from a shell that can read it, then try again.",
    ),
    (
        "unable to read",
        "Git could not read part of the tree. A running build or an antivirus scan "
        "holding files open is the usual cause on Windows.",
    ),
)


def _explain(warnings: str) -> str:
    """Append the cure for any warning this module recognises."""
    lowered = warnings.lower()
    cures = [cure for marker, cure in _KNOWN_WARNINGS if marker in lowered]
    if not cures:
        return ""
    return "\n\nHow to fix:\n  " + "\n  ".join(cures)


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
    # Streams kept apart: a git warning carries no NUL, so folding stderr into stdout
    # glued it onto the next record and made that path vanish from the plan entirely.
    #
    # Bytes, not text: a path git reports is a sequence of bytes, and decoding it with
    # replacement characters would hand the protection rules a name that is not the real
    # one. What the rules must match is exactly what `rmtree` will be given.
    code, raw_output, raw_errors = capture_bytes(_LIST_ARGV, root, timeout=_LIST_TIMEOUT)
    errors = raw_errors.decode("utf-8", errors="replace")
    if code == NOT_FOUND:
        raise DiscoveryError("git is not on PATH, so nothing can be classified safely")
    if code == TIMED_OUT:
        raise DiscoveryError(
            "`git status` did not finish in time, so the working tree state is unknown"
        )
    if code != 0:
        raise DiscoveryError(f"`git status` failed with exit {code}: {errors.strip()}")
    if errors.strip():
        # Not fatal to git — it warns about unreadable directories and still reports the
        # rest — but the reader has to know the picture is incomplete before authorising
        # deletion, and now also how to make it complete.
        raise DiscoveryError(
            "`git status` reported warnings, so the list of what is safe to delete may be "
            f"incomplete. Resolve these first:\n{errors.strip()}{_explain(errors)}"
        )

    try:
        output = raw_output.decode("utf-8")
    except UnicodeDecodeError as exc:
        # Fail closed, in the same spirit as an unreadable subtree: a path that cannot be
        # decoded cannot be matched against the protected list, so nothing here can be
        # classified safely. `git config core.quotepath` does not apply under `-z`.
        raise DiscoveryError(
            "`git status` reported a path this script cannot decode as UTF-8 "
            f"(byte {exc.start} of its output), so the protected-path rules cannot be "
            "applied to it. Rename or remove that path by hand, then try again."
        ) from exc

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
    """Why ``directory`` may not be deleted, or ``None`` if nothing stops it.

    A directory that shelters something protected is itself protected, whole. Deleting
    the rest of it and keeping the one file would be a partial result nobody asked for,
    from a plan that listed the directory as a single line — declining and naming what
    stopped it leaves the decision with the person who can make it.

    Three things stop a directory: a protected path inside it, a nested repository at any
    depth (an untracked ``vendor/`` reports to git as one entry, so a top-level-only check
    queued an unpushed sibling clone for deletion — `git clean` refuses such a directory
    without `-ff`, and so does this), and **a subtree that cannot be enumerated**.

    That last one is the fail-closed rule, and it is why this returns a reason rather
    than a bool. Discarding an ``os.walk`` error turns "I could not look" into "there is
    nothing there", so a directory holding a secret behind a permission wall read as
    cleanable. An enumeration error now protects the directory and says so.

    One walk answers all three questions. Two separate walks meant every candidate
    traversed ``target/`` and ``node_modules`` twice, which took the repository's own dry
    run from near-instant to 34 seconds.
    """
    if (directory / ".git").exists():
        return f"nested git repository {_as_relative(root, directory / '.git')}"

    blocked: list[str] = []

    def unreadable(error: OSError) -> None:
        blocked.append(_as_relative(root, Path(getattr(error, "filename", directory))))

    for current, dirs, files in os.walk(directory, onerror=unreadable):
        if blocked:
            return f"unreadable subtree {blocked[0]} — cannot prove it holds nothing protected"
        current_path = Path(current)
        if ".git" in dirs:
            return f"nested git repository {_as_relative(root, current_path / '.git')}"
        # `followlinks` defaults to False, so os.walk never descends through a POSIX
        # symlink. Excluding the names as well keeps a link's *target* from deciding the
        # candidate's fate. Windows junctions are a real gap: `is_symlink()` reports False
        # for them, so they are traversed. That errs toward protecting more, not less.
        dirs[:] = [d for d in dirs if not (current_path / d).is_symlink()]
        for name in files + dirs:
            relative = _as_relative(root, current_path / name)
            if protection.is_protected(relative):
                return relative

    if blocked:
        return f"unreadable subtree {blocked[0]} — cannot prove it holds nothing protected"
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
