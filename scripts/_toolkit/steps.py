"""Running a declared list of commands, and reporting honestly which ones failed.

Several scripts are a named sequence of commands with a summary at the end. That shape
belongs here once; the sequences themselves stay in each script's JSON.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from .console import fail, ok, step, summary, warn
from .processes import run


class StepsFailed(RuntimeError):
    """At least one required step failed."""


def run_steps(steps: list[dict[str, Any]], root: Path, *, title: str) -> int:
    """Run every step, then summarise.

    Deliberately does **not** stop at the first failure when the step is optional, and
    does stop for a required one: a missing optional tool should not hide the rest of
    the report, while a failed required step makes everything after it meaningless.

    Returns a process exit code: 0 if every required step passed.
    """
    rows: list[tuple[str, str]] = []
    failed = False

    for entry in steps:
        label = entry["label"]
        argv = entry["argv"]
        required = bool(entry.get("required", True))

        step(label)
        result = run(list(argv), root)

        if result.ok:
            ok(label)
            rows.append((label, "ok"))
            continue

        if result.returncode == 127:
            note = f"tool not found: {argv[0]}"
            if required:
                fail(f"{label} — {note}")
                rows.append((label, "MISSING (required)"))
                failed = True
                break
            warn(f"{label} — {note}, skipped")
            rows.append((label, "missing (optional)"))
            continue

        if required:
            fail(f"{label} — exit {result.returncode}")
            rows.append((label, f"FAILED ({result.returncode})"))
            failed = True
            break

        warn(f"{label} — exit {result.returncode}, not required")
        rows.append((label, f"failed ({result.returncode}), optional"))

    summary(title, rows)
    return 1 if failed else 0
