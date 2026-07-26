"""Build the distributable Windows installer.

Produces the NSIS ``.exe`` under ``target/release/bundle/nsis/`` and reports what it
actually found on disk afterwards.

On a non-Windows host it refuses immediately with the reason, instead of starting a long
release build that would fail somewhere inside the bundler. A tool that wastes ten
minutes before admitting it cannot do the job is worse than one that says so up front.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts._toolkit.config import load_config, repo_root
from scripts._toolkit.console import fail, heading, info, ok, warn
from scripts._toolkit.steps import run_steps

SCRIPT_DIR = Path(__file__).resolve().parent


def _human_mib(size: int) -> str:
    return f"{size / (1024 * 1024):.1f} MiB"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="build-installer",
        description="Build the Windows NSIS installer for distribution.",
    )
    parser.add_argument(
        "--allow-other-platform",
        action="store_true",
        help="attempt the build anyway on a non-Windows host (expected to fail)",
    )
    args = parser.parse_args(argv)

    root = repo_root()
    config = load_config(SCRIPT_DIR, "build.json")

    required = config["requires_platform"]
    if not sys.platform.startswith(required) and not args.allow_other_platform:
        heading("build-installer — not available on this platform")
        info(config["platform_refusal"])
        info("")
        info("Pass --allow-other-platform to try regardless.")
        return 1

    exit_code = run_steps(config["steps"], root, title="build-installer")
    if exit_code != 0:
        return exit_code

    bundle_dir = root / config["artifact_dir"]
    heading("installer")
    if not bundle_dir.is_dir():
        # The build reported success, so a missing directory means the bundler wrote
        # somewhere else — say which directory was checked rather than claim an
        # installer exists.
        fail(f"expected {bundle_dir} to exist after a successful build")
        return 1

    installers = sorted(bundle_dir.glob(config["artifact_glob"]))
    if not installers:
        fail(f"no {config['artifact_glob']} in {bundle_dir}")
        return 1

    for installer in installers:
        ok(f"{installer}  ({_human_mib(installer.stat().st_size)})")
    warn(
        "Unsigned: Windows SmartScreen will warn about an unknown publisher. "
        "Code signing is stage S14."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
