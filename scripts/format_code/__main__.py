"""Format every source file in the repository, or check that it is already formatted.

Thin on purpose: `cargo xtask fmt` already runs rustfmt and Prettier together, and a
second implementation of "how this project formats code" would drift from the one the
pre-commit hook and CI use. The step lists live in config/steps.json.
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
        prog="format-code",
        description="Run rustfmt and Prettier over the whole repository.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="report violations without rewriting anything (what CI does)",
    )
    args = parser.parse_args(argv)

    config = load_config(SCRIPT_DIR, "steps.json")
    steps = config["check_steps" if args.check else "steps"]
    title = "format-code (check only)" if args.check else "format-code"
    return run_steps(steps, repo_root(), title=title)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
