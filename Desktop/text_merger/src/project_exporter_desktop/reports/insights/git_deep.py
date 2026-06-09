from __future__ import annotations

import threading
from collections.abc import Callable
from pathlib import Path

from ...reports.git_report import run_git_command
from ...utils.text_utils import redact_secrets
from ...utils.time_utils import human_now

def write_git_deep_report(
    source_root: Path,
    output_file: Path,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> None:
    commands: list[list[str]] = [
        ["git", "status", "--short", "--branch"],
        ["git", "branch", "--show-current"],
        ["git", "branch", "-a"],
        ["git", "remote", "-v"],
        ["git", "diff", "--stat"],
        ["git", "diff", "--name-only"],
        ["git", "ls-files"],
        ["git", "ls-files", "--others", "--exclude-standard"],
        ["git", "log", "--oneline", "--decorate", "--graph", "-20"],
        ["git", "rev-parse", "--show-toplevel"],
    ]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Git Deep Report ===\n")
        out.write(f"Source root: {source_root}\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(
            "Important: this report does not switch branches and does not modify the repository.\n"
        )
        out.write("=" * 100 + "\n\n")

        if not (source_root / ".git").exists():
            out.write("No .git directory was found in the selected root.\n")
            return

        for command in commands:
            if cancel.is_set():
                out.write("\nOperation cancelled by user.\n")
                return
            command_text = " ".join(command)
            log(f"Git insight: {command_text}")
            out.write("\n" + "=" * 100 + "\n")
            out.write(f"$ {command_text}\n")
            out.write("=" * 100 + "\n\n")
            code, stdout, stderr = run_git_command(
                command, cwd=source_root, timeout_seconds=120
            )
            out.write(f"Exit code: {code}\n\n")
            out.write("--- STDOUT ---\n")
            out.write(redact_secrets(stdout or ""))
            if stdout and not stdout.endswith("\n"):
                out.write("\n")
            out.write("\n--- STDERR ---\n")
            out.write(redact_secrets(stderr or ""))
            if stderr and not stderr.endswith("\n"):
                out.write("\n")
            out.write("\n")
