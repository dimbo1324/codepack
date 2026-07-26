"""Launching external tools, portably.

The whole reason this module exists is that "run a command" is not portable, and every
script needs it. Getting it wrong once, here, is far better than getting it wrong seven
times in seven scripts.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

#: Tools that are not a plain executable on Windows. npm-installed CLIs land as
#: ``<name>.CMD`` shims, and ``CreateProcess`` only ever appends ``.exe`` when resolving
#: a bare name — so a bare "pnpm" is reported as missing on a machine that has it. This
#: exact bug was live in ``cargo xtask doctor`` until 2026-07-26.
_WINDOWS_SHIM_SUFFIXES = (".cmd", ".bat", ".exe", "")


class ToolNotFoundError(RuntimeError):
    """A required external tool is not on PATH."""


def find_tool(name: str) -> str | None:
    """Absolute path to ``name``, or ``None``.

    On Windows this tries the shim suffixes explicitly rather than trusting a bare
    lookup, because that is where npm-installed CLIs live.
    """
    direct = shutil.which(name)
    if direct:
        return direct
    if os.name == "nt":
        for suffix in _WINDOWS_SHIM_SUFFIXES:
            if not suffix:
                continue
            found = shutil.which(name + suffix)
            if found:
                return found
    return None


def require_tool(name: str, purpose: str) -> str:
    resolved = find_tool(name)
    if resolved is None:
        raise ToolNotFoundError(
            f"{name!r} is not on PATH, and it is needed to {purpose}."
        )
    return resolved


@dataclass(frozen=True)
class CommandResult:
    argv: list[str]
    returncode: int

    @property
    def ok(self) -> bool:
        return self.returncode == 0


def run(
    argv: list[str],
    cwd: Path,
    *,
    echo: bool = True,
    env: dict[str, str] | None = None,
) -> CommandResult:
    """Run ``argv`` and stream its output straight through.

    Always a list, never ``shell=True``: these argv lists are assembled from JSON
    config, and a shell would turn a config value into shell syntax.

    A missing executable comes back as a normal failing result rather than an
    exception, so a caller running several steps can report "this one is missing" in
    the same shape as "this one failed".
    """
    resolved = find_tool(argv[0])
    real_argv = [resolved or argv[0], *argv[1:]]
    if echo:
        print(f"$ {' '.join(argv)}", flush=True)
    if resolved is None:
        print(f"  not found on PATH: {argv[0]}", file=sys.stderr, flush=True)
        return CommandResult(argv=real_argv, returncode=127)

    merged_env = None
    if env:
        merged_env = {**os.environ, **env}
    completed = subprocess.run(real_argv, cwd=cwd, check=False, env=merged_env)
    return CommandResult(argv=real_argv, returncode=completed.returncode)


def capture(argv: list[str], cwd: Path) -> tuple[int, str]:
    """Run ``argv`` and return ``(returncode, stdout)``, stderr folded in.

    For reading a tool's answer — a version string, ``git`` output — rather than
    showing it. ``errors="replace"`` because a version banner in an unexpected
    encoding must not crash a diagnostic command.
    """
    resolved = find_tool(argv[0])
    if resolved is None:
        return 127, ""
    completed = subprocess.run(
        [resolved, *argv[1:]],
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return completed.returncode, completed.stdout or ""
