from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class DiffSelection:
    mode: str
    paths: frozenset[str] | None
    warning: str | None = None

    @property
    def is_limited(self) -> bool:
        return self.paths is not None


def _run_git(args: list[str], cwd: Path) -> tuple[int, list[str], str]:
    try:
        completed = subprocess.run(
            ["git", *args],
            cwd=str(cwd),
            text=True,
            capture_output=True,
            timeout=90,
            shell=False,
            encoding="utf-8",
            errors="replace",
        )
        lines = [
            line.strip().replace("/", "\\")
            for line in completed.stdout.splitlines()
            if line.strip()
        ]
        return completed.returncode, lines, completed.stderr.strip()
    except FileNotFoundError:
        return 127, [], "Git executable was not found."
    except Exception as exc:
        return 1, [], f"{type(exc).__name__}: {exc}"


def resolve_diff_selection(
    source_root: Path,
    mode: str,
    base_ref: str = "HEAD",
    target_ref: str = "",
) -> DiffSelection:
    """Return a Git-limited file set or ``paths=None`` for a full export."""
    if mode == "all":
        return DiffSelection(mode=mode, paths=None)

    rc, _inside, err = _run_git(["rev-parse", "--is-inside-work-tree"], source_root)
    if rc != 0:
        return DiffSelection(
            mode="all",
            paths=None,
            warning=f"Git diff mode disabled: {err or 'not a Git repository'}",
        )

    paths: set[str] = set()
    warning: str | None = None

    if mode == "uncommitted":
        commands = [
            ["diff", "--name-only", "--diff-filter=ACMRTUXB", "HEAD", "--"],
            ["ls-files", "--others", "--exclude-standard"],
        ]
        for args in commands:
            rc, lines, err = _run_git(args, source_root)
            if rc == 0:
                paths.update(lines)
            elif warning is None:
                warning = err or f"git {' '.join(args)} failed"

    elif mode == "changed_since_ref":
        ref = (base_ref or "HEAD").strip()
        rc, lines, err = _run_git(
            ["diff", "--name-only", "--diff-filter=ACMRTUXB", f"{ref}...HEAD", "--"], source_root
        )
        if rc == 0:
            paths.update(lines)
        else:
            warning = err or f"Could not diff {ref}...HEAD"

    elif mode == "between_refs":
        base = (base_ref or "HEAD").strip()
        target = (target_ref or "HEAD").strip()
        rc, lines, err = _run_git(
            ["diff", "--name-only", "--diff-filter=ACMRTUXB", f"{base}...{target}", "--"],
            source_root,
        )
        if rc == 0:
            paths.update(lines)
        else:
            warning = err or f"Could not diff {base}...{target}"

    else:
        return DiffSelection(mode="all", paths=None, warning=f"Unknown diff export mode: {mode}")

    if warning:
        return DiffSelection(mode="all", paths=None, warning=f"Git diff mode disabled: {warning}")

    return DiffSelection(mode=mode, paths=frozenset(paths))