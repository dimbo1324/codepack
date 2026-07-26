"""Presenting the plan before anything is deleted.

The dry run is the script's main safety feature, so this output has to be good enough
that a reader can actually spot a mistake in it.
"""

from __future__ import annotations

from scripts._toolkit.console import heading, info, ok, warn

from .discovery import Candidate


def human_bytes(size: int) -> str:
    value = float(size)
    for unit in ("B", "KiB", "MiB", "GiB"):
        if value < 1024 or unit == "GiB":
            return f"{value:.0f} {unit}" if unit == "B" else f"{value:.1f} {unit}"
        value /= 1024
    return f"{value:.1f} GiB"


def print_plan(candidates: list[Candidate], *, scope: str, applying: bool) -> None:
    artifacts = [c for c in candidates if c.kind == "artifact"]
    others = [c for c in candidates if c.kind == "other"]
    protected = [c for c in candidates if c.kind == "protected"]

    heading("clean-project — " + ("removing" if applying else "dry run, nothing deleted"))
    info(f"scope: {scope}")

    if artifacts:
        print("\n  build artifacts (regenerable):")
        for candidate in artifacts:
            print(f"    - {candidate.relative}{'/' if candidate.is_dir else ''}")

    if others:
        # Listed in both scopes, but only acted on by 'all'. Showing them either way is
        # deliberate: "what is not tracked here" is useful to see even when this run
        # will not touch it, and hiding it would make the two scopes look identical.
        suffix = "" if scope == "all" else "  (kept — scope is 'artifacts')"
        print(f"\n  other untracked paths:{suffix}")
        for candidate in others:
            print(f"    - {candidate.relative}{'/' if candidate.is_dir else ''}")
        if scope == "all":
            # The one warning worth interrupting for: these are the paths that could be
            # work nobody has committed yet, and the script genuinely cannot tell.
            warn(
                f"{len(others)} path(s) above are not known build output. If any of that "
                "is work you have not committed, it will be gone."
            )

    if protected:
        print("\n  protected, will NOT be touched:")
        for candidate in protected:
            note = f"  ({candidate.reason})" if candidate.reason else ""
            print(f"    - {candidate.relative}{'/' if candidate.is_dir else ''}{note}")

    if not artifacts and not others:
        ok("nothing to remove — the working tree already matches what git tracks")
