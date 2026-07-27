"""Running a declared list of commands, and reporting honestly which ones failed.

Several scripts are a named sequence of commands with a summary at the end. That shape
belongs here once; the sequences themselves stay in each script's JSON.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from .console import fail, ok, step, summary, warn
from .processes import NOT_FOUND, run


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

    for index, entry in enumerate(steps):
        label = entry["label"]
        argv = entry["argv"]
        required = bool(entry.get("required", True))

        step(label)
        result = run(list(argv), root)

        if result.ok:
            ok(label)
            rows.append((label, "ok"))
            continue

        if result.returncode == NOT_FOUND:
            note = f"tool not found: {argv[0]}"
            if required:
                fail(f"{label} — {note}")
                rows.append((label, "MISSING (required)"))
                failed = True
                rows.extend(_not_reached(steps[index + 1 :]))
                break
            warn(f"{label} — {note}, skipped")
            rows.append((label, "missing (optional)"))
            continue

        if required:
            fail(f"{label} — exit {result.returncode}")
            rows.append((label, f"FAILED ({result.returncode})"))
            failed = True
            rows.extend(_not_reached(steps[index + 1 :]))
            break

        warn(f"{label} — exit {result.returncode}, not required")
        rows.append((label, f"failed ({result.returncode}), optional"))

    summary(title, rows)
    return 1 if failed else 0


def _not_reached(remaining: list[dict[str, Any]]) -> list[tuple[str, str]]:
    """Rows for the steps a required failure prevented from running.

    Omitting them made the summary read as though the run had been shorter than it was
    declared to be — a reader comparing a failing summary against a passing one saw
    lines disappear and had to guess whether they were skipped or removed. Naming them
    as not run is the honest report the summary is for.
    """
    return [(entry["label"], "not run (earlier step failed)") for entry in remaining]
