"""Run the project's quality gate.

The gate is `cargo xtask gate`, which CI runs verbatim; this script is a door to it, not
a reimplementation. If the two ever disagreed, the local gate would stop meaning
anything.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts._toolkit.config import load_config, repo_root
from scripts._toolkit.steps import run_steps

SCRIPT_DIR = Path(__file__).resolve().parent


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="quality-gate",
        description="Run the full quality gate, or the quick subset.",
    )
    parser.add_argument(
        "--quick",
        action="store_true",
        help="skip the test run — the minimum before a push, not before a merge",
    )
    args = parser.parse_args(argv)

    config = load_config(SCRIPT_DIR, "steps.json")
    steps = config["quick_steps" if args.quick else "steps"]
    title = "quality-gate (quick)" if args.quick else "quality-gate (full)"
    return run_steps(steps, repo_root(), title=title)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
