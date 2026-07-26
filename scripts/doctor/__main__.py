"""Report what this machine can and cannot do — read-only, always exit-code honest.

Small enough to stay one module. What it probes lives in ``config/tools.json``; adding
a tool is a JSON edit.
"""

from __future__ import annotations

import argparse
import os
import platform
import sys
from pathlib import Path

from scripts._toolkit.config import load_config, repo_root
from scripts._toolkit.console import fail, heading, info, ok, step, warn
from scripts._toolkit.processes import capture, find_tool

SCRIPT_DIR = Path(__file__).resolve().parent


def _first_line(text: str) -> str:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped:
            return stripped
    return "(no version output)"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="doctor",
        description="Check that the tools the other scripts need are present.",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="also fail when an optional tool is missing",
    )
    args = parser.parse_args(argv)

    root = repo_root()
    config = load_config(SCRIPT_DIR, "tools.json")

    heading("codepack — environment check")
    info(f"repository   {root}")
    info(f"python       {sys.version.split()[0]}  ({sys.executable})")
    info(f"platform     {platform.platform()}")
    info(f"sys.platform {sys.platform}")

    missing_required: list[str] = []
    missing_optional: list[str] = []

    step("tools")
    for entry in config["tools"]:
        name = entry["name"]
        resolved = find_tool(name)
        if resolved is None:
            (missing_required if entry["required"] else missing_optional).append(name)
            label = "required" if entry["required"] else "optional"
            warn(f"{name:<12} MISSING ({label}) — needed for {entry['needed_for']}")
            continue
        code, output = capture([name, *entry["version_args"]], root)
        # A tool that resolves but cannot report a version is still a problem worth
        # seeing: that is what a broken shim or a half-finished install looks like.
        if code != 0:
            warn(f"{name:<12} found but `{name} --version` failed ({resolved})")
            continue
        ok(f"{name:<12} {_first_line(output)}")

    step("paths")
    for entry in config["paths"]:
        target = root / entry["path"]
        if target.exists():
            ok(f"{entry['label']}")
        else:
            warn(f"{entry['label']} missing — {entry['hint']}")

    step("platform notes")
    if os.name == "nt":
        ok("Windows host: every script in the catalog is usable here")
    else:
        for name in config["windows_only_scripts"]:
            warn(f"{name} cannot run on {sys.platform} — the product targets Windows")
        info("Owner decision 2026-07-26; see docs/decisions/open-questions.md")

    heading("verdict")
    if missing_required:
        fail(f"missing required tool(s): {', '.join(missing_required)}")
        return 1
    if missing_optional and args.strict:
        fail(f"missing optional tool(s) with --strict: {', '.join(missing_optional)}")
        return 1
    if missing_optional:
        warn(f"optional tool(s) absent: {', '.join(missing_optional)}")
        info("Not a failure — the affected steps will say so when they run.")
    ok("environment is usable")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
