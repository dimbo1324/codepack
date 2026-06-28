from __future__ import annotations

import threading
from collections import Counter
from collections.abc import Callable
from pathlib import Path

from ...reports.git_report import run_git_command
from ...utils.time_utils import human_now


def _git_lines(source_root: Path, command: list[str], timeout: int = 120) -> list[str]:
    code, stdout, _stderr = run_git_command(command, cwd=source_root, timeout_seconds=timeout)
    if code != 0:
        return []
    return [line for line in stdout.splitlines() if line.strip()]


def write_git_timeline_report(
    source_root: Path,
    output_file: Path,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> None:
    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("# Git Timeline Report\n\n")
        out.write(f"Source root: `{source_root}`\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write("This report uses read-only Git commands and does not change the repository.\n\n")

        if not (source_root / ".git").exists():
            out.write("No `.git` directory was found in the selected root.\n")
            return

        commands = {
            "Current branch": ["git", "branch", "--show-current"],
            "HEAD commit": ["git", "rev-parse", "--short", "HEAD"],
            "Contributors": ["git", "shortlog", "-sne", "HEAD"],
        }
        out.write("## Repository summary\n\n")
        for title, command in commands.items():
            if cancel.is_set():
                out.write("Operation cancelled.\n")
                return
            log(f"Git timeline: {' '.join(command)}")
            lines = _git_lines(source_root, command)
            out.write(f"### {title}\n\n")
            if lines:
                for line in lines[:50]:
                    out.write(f"- `{line}`\n")
            else:
                out.write("- not available\n")
            out.write("\n")

        out.write("## Last 30 commits\n\n")
        log("Git timeline: last commits")
        commits = _git_lines(
            source_root,
            ["git", "log", "--date=short", "--pretty=format:%h%x09%ad%x09%an%x09%s", "-30"],
        )
        if commits:
            for line in commits:
                parts = line.split("\t", 3)
                if len(parts) == 4:
                    h, date, author, subject = parts
                    out.write(f"- `{h}` {date} — {author}: {subject}\n")
                else:
                    out.write(f"- `{line}`\n")
        else:
            out.write("No commits detected.\n")

        out.write("\n## Files with the most churn in the last 200 commits\n\n")
        log("Git timeline: numstat churn")
        numstat = _git_lines(
            source_root, ["git", "log", "--numstat", "--pretty=format:", "-200"], timeout=180
        )
        churn: Counter[str] = Counter()
        changes: Counter[str] = Counter()
        for line in numstat:
            parts = line.split("\t")
            if len(parts) != 3:
                continue
            added, deleted, path = parts
            try:
                delta = (0 if added == "-" else int(added)) + (
                    0 if deleted == "-" else int(deleted)
                )
            except ValueError:
                continue
            churn[path] += delta
            changes[path] += 1
        if churn:
            out.write(f"{'Churn':>10} {'Touches':>8}  File\n")
            for path, amount in churn.most_common(50):
                out.write(f"{amount:>10,} {changes[path]:>8,}  {path}\n")
        else:
            out.write("No churn data available.\n")