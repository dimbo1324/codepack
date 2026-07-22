from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
COMMANDS = {"sync-agents": ROOT / "scripts" / "dev_tools" / "sync_agents_md.py"}

USAGE = """\
Usage: python dev_tools_scripts_runner.py <command> [options]

Commands:
  sync-agents [--check]   Regenerate AGENTS.md from the .ai/ rule modules.

This runner is temporary scaffolding for the greenfield stage. Stage S0 of
ROADMAP.md replaces it with `cargo xtask`.
"""


def main(argv: list[str]) -> int:
    if not argv or argv[0] in {"-h", "--help"}:
        print(USAGE)
        return 0 if argv else 2
    command, *rest = argv
    script = COMMANDS.get(command)
    if script is None:
        print(f"ERROR: unknown command: {command}\n\n{USAGE}", file=sys.stderr)
        return 2
    return subprocess.call([sys.executable, str(script), *rest])


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
