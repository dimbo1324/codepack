from __future__ import annotations

import subprocess
import threading
from collections.abc import Callable
from pathlib import Path

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
        stdout = exc.stdout if isinstance(exc.stdout, str) else ""
        stderr = exc.stderr if isinstance(exc.stderr, str) else ""
        return None, stdout, stderr + f"\nTIMEOUT after {timeout_seconds} seconds."
    except FileNotFoundError:
        return (
            None,
            "",
            "Git executable was not found. Install Git for Windows and "
            "ensure git.exe is in PATH.",
        )
    except Exception as exc:
        return None, "", f"{type(exc).__name__}: {exc}"

def write_git_report(
    source_root: Path,
    output_file: Path,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> None:
    log(f"Формирую Git-отчёт: {output_file.name}")

    commands: list[list[str]] = [
        ["git", "status", "--short", "--branch"],
        ["git", "branch", "--show-current"],
        ["git", "log", "--oneline", "-5"],
        ["git", "show", "--stat", "HEAD"],
        ["git", "show", "HEAD"],
    ]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Git Report ===\n")
        out.write(f"Source root: {source_root}\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(
            "Note: Git data is collected from the ORIGINAL project (the .git "
            "directory is intentionally never copied into the bundle).\n"
        )
        out.write("=" * 100 + "\n\n")

        git_dir = source_root / ".git"
        if not git_dir.exists():
            out.write("No .git directory was found in the selected root.\n")
            out.write("Git commands were not executed.\n")
            log("Git-папка не найдена. Git-команды пропущены.")
            return

        for command in commands:
            if cancel.is_set():
                out.write("\nOperation cancelled by user.\n")
                break

            command_text = " ".join(command)
            log(f"Выполняю: {command_text}")

            out.write("\n" + "=" * 100 + "\n")
            out.write(f"$ {command_text}\n")
            out.write("=" * 100 + "\n\n")

            code, stdout, stderr = run_git_command(command, cwd=source_root)
            out.write(f"Exit code: {code}\n\n")

            out.write("--- STDOUT ---\n")
            out.write(stdout or "")
            if stdout and not stdout.endswith("\n"):
                out.write("\n")

            out.write("\n--- STDERR ---\n")
            out.write(stderr or "")
            if stderr and not stderr.endswith("\n"):
                out.write("\n")

            out.write("\n")

    log("Git-отчёт готов")
