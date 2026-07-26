"""Reduce the working tree to exactly what git tracks.

Removes untracked and ignored paths, then prunes the empty directories git cannot track
and therefore never cleans up on its own.

Two deliberate asymmetries, because this is the one script that destroys data:

* The **default action is a dry run.** Choosing a wide scope and choosing to delete are
  separate decisions, and the second one has to be typed.
* The **protected list wins over everything.** ``.env`` holds this project's only copy
  of real credentials by its own security rules, so a clean that removed it would
  destroy something that exists nowhere else. See ``config/clean.json``.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts._toolkit.config import load_config, repo_root
from scripts._toolkit.console import confirm, fail, heading, info, ok, warn

from .core.discovery import DiscoveryError, discover
from .core.protection import ProtectionRules
from .core.removal import prune_empty_dirs, remove_all
from .core.report import human_bytes, print_plan

SCRIPT_DIR = Path(__file__).resolve().parent


def build_parser(default_scope: str) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="clean-project",
        description="Reduce the working tree to what git tracks. Dry run by default.",
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="actually delete; without this the script only reports",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="skip the confirmation prompt (for scripted use)",
    )
    parser.add_argument(
        "--scope",
        choices=("artifacts", "all"),
        default=default_scope,
        help=(
            "'artifacts' removes only known regenerable output; "
            "'all' removes everything git does not track"
        ),
    )
    parser.add_argument(
        "--keep-empty-dirs",
        action="store_true",
        help="do not prune empty directories",
    )
    return parser


def main(argv: list[str]) -> int:
    config = load_config(SCRIPT_DIR, "clean.json")
    args = build_parser(config.get("default_scope", "all")).parse_args(argv)

    root = repo_root()
    protection = ProtectionRules(config["protected"])

    try:
        candidates = discover(root, config["artifacts"], protection)
    except DiscoveryError as error:
        fail(str(error))
        return 1

    selected = [
        c
        for c in candidates
        if not c.protected and (args.scope == "all" or c.artifact)
    ]

    print_plan(candidates, scope=args.scope, applying=args.apply)

    pruning = config.get("remove_empty_dirs", True) and not args.keep_empty_dirs
    prune_argv = (
        root,
        config.get("empty_dir_roots", ["."]),
        config.get("empty_dir_skip", []),
        protection,
    )

    if not args.apply:
        if pruning:
            heading("empty directories that would also be pruned")
            already_empty = prune_empty_dirs(*prune_argv, dry_run=True)
            for relative in already_empty:
                info(f"- {relative}")
            if not already_empty:
                ok("none right now")
            info("Deleting the paths above may leave more directories empty.")
        info("")
        info("Re-run with --apply to delete. Add --yes to skip the prompt.")
        if args.scope == "all":
            info("Or use --scope artifacts to remove only regenerable build output.")
        return 0

    if selected:
        question = f"Delete {len(selected)} path(s) listed above?"
        if not confirm(question, assume_yes=args.yes):
            warn("cancelled — nothing was deleted")
            return 1

        outcome = remove_all(root, [c.relative for c in selected])
        heading("removed")
        info(f"files:      {outcome.removed_files}")
        info(f"directories:{outcome.removed_dirs}")
        info(f"reclaimed:  {human_bytes(outcome.freed_bytes)}")
        if outcome.failures:
            for relative, error in outcome.failures:
                fail(f"{relative}: {error}")
            warn(
                "Some paths could not be removed. On Windows this usually means a "
                "running process holds them — close editors and builds, then re-run."
            )
            return 1

    if pruning:
        pruned = prune_empty_dirs(*prune_argv)
        heading("empty directories pruned")
        if pruned:
            for relative in pruned:
                info(f"- {relative}")
        else:
            ok("none left behind")

    ok("working tree now matches what git tracks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
