"""Install the formatting pre-commit hook.

Points core.hooksPath at the tracked .githooks/ directory, so the hook is versioned with
the repository instead of living in an untracked .git/hooks that vanishes on every fresh
clone. Run once per clone.
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
        prog="install-hooks",
        description="Activate the repository's tracked git hooks.",
    )
    parser.parse_args(argv)

    config = load_config(SCRIPT_DIR, "steps.json")
    return run_steps(config["steps"], repo_root(), title="install-hooks")


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
