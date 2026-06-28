"""Basic Git report: runs a small set of read-only git commands and writes their output.

Used as step 5 of the export pipeline to give consumers a quick status snapshot of
the source project without including the .git directory itself.
"""

from __future__ import annotations

import subprocess
import threading
from collections.abc import Callable
from pathlib import Path

from ..utils.text_utils import redact_secrets
from ..utils.time_utils import human_now


def run_git_command(
    args: list[str], cwd: Path, timeout_seconds: int = 120
) -> tuple[int | None, str, str]:
    try:
        completed = subprocess.run(
            args,
            cwd=str(cwd),
            text=True,
            capture_output=True,
            timeout=timeout_seconds,
            shell=False,
            encoding="utf-8",
            errors="replace",
        )
        return completed.returncode, completed.stdout, completed.stderr
    except subprocess.TimeoutExpired as exc:
        # exc.stdout/stderr may be bytes when text=True fails mid-stream; guard accordingly.
        stdout = exc.stdout if isinstance(exc.stdout, str) else ""
        stderr = exc.stderr if isinstance(exc.stderr, str) else ""
        return None, stdout, stderr + f"\nTIMEOUT after {timeout_seconds} seconds."
    except FileNotFoundError:
        return (
            None,
            "",
            "Git executable was not found. Install Git for Windows and ensure git.exe is in PATH.",
        )
    except Exception as exc:
        return None, "", f"{type(exc).__name__}: {exc}"


def write_git_report(
    source_root: Path,
    output_file: Path,
    log: Callable[[str], None],
    cancel: threading.Event,
    include_patch: bool = False,
    redact: bool = True,
) -> None:
    log(f"Формирую Git-отчёт: {output_file.name}")

    # The command list is intentionally minimal: only safe, fast, read-only commands.
    commands: list[list[str]] = [
        ["git", "status", "--short", "--branch"],
        ["git", "branch", "--show-current"],
        ["git", "log", "--oneline", "-5"],
        ["git", "show", "--stat", "--name-status", "HEAD"],
    ]
    if include_patch:
        commands.append(["git", "show", "--patch", "--find-renames", "HEAD"])

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Git Report ===\n")
        out.write(f"Source root: {source_root}\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(f"Patch included: {'yes' if include_patch else 'no'}\n")
        out.write("Git commands are read-only. The .git directory is not copied.\n")
        out.write(
            "Secret redaction is applied to command output.\n"
            if redact
            else "Secret redaction is disabled.\n"
        )
        out.write("=" * 100 + "\n\n")

        for command in commands:
            if cancel.is_set():
                out.write("\nCANCELLED BY USER\n")
                return

            out.write(f"$ {' '.join(command)}\n")
            code, stdout, stderr = run_git_command(command, source_root)
            if redact:
                stdout = redact_secrets(stdout)
                stderr = redact_secrets(stderr)
            out.write(f"exit_code: {code}\n")
            if stdout:
                out.write("--- stdout ---\n")
                out.write(stdout)
                if not stdout.endswith("\n"):
                    out.write("\n")
            if stderr:
                out.write("--- stderr ---\n")
                out.write(stderr)
                if not stderr.endswith("\n"):
                    out.write("\n")
            out.write("\n" + "-" * 100 + "\n\n")
