"""Subprocess execution for catalog scripts."""

from __future__ import annotations

import shlex
import subprocess
import sys
from pathlib import Path

from .models import Lang, ScriptInfo


class ScriptRunner:
    """Launches a catalog script as ``python -m <module>``.

    The argv is always a list and ``shell`` is never true. This is the one place a
    string that came from config or from a user reaches a process launch, so it stays
    deliberately boring.

    Launching by module rather than by file path is what lets a one-file script and a
    fully decomposed one be invoked identically, and lets the decomposed one use
    ordinary relative imports without any ``sys.path`` manipulation.
    """

    def __init__(self, root_dir: Path) -> None:
        self._root_dir = root_dir

    def run(self, script: ScriptInfo, extra_args: list[str]) -> int:
        completed = subprocess.run(
            [sys.executable, "-m", script.module, *extra_args],
            cwd=self._root_dir,
            check=False,
        )
        return completed.returncode

    def prompt_extra_args(self, script: ScriptInfo, lang: Lang) -> list[str]:
        prompt = (
            f"Extra arguments for {script.title} (optional, Enter to skip): "
            if lang == "en"
            else f"Дополнительные аргументы для {script.title} "
            "(необязательно, Enter — пропустить): "
        )
        raw = input(prompt).strip()
        return shlex.split(raw) if raw else []

    def launch(self, script: ScriptInfo, lang: Lang) -> int:
        return self.run(script, self.prompt_extra_args(script, lang))
