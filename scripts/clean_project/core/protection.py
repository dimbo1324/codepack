"""Deciding what must never be deleted.

This is the safety boundary of the whole script. A full clean is, by definition,
indistinguishable from destroying uncommitted work — so the only thing standing between
"reduce the repository to what git tracks" and "delete a developer's live credentials"
is this file and the list it reads.
"""

from __future__ import annotations

from fnmatch import fnmatch


class ProtectionRules:
    """Glob rules for paths that survive every mode.

    A leading ``!`` re-includes a path that an earlier pattern protected, which is how
    ``.env.example`` stays cleanable while ``.env`` and ``.env.production`` do not.
    Re-inclusions are checked last, so ordering inside the config file never matters —
    a reader of ``clean.json`` should not have to reason about precedence to know
    whether their secrets are safe.
    """

    def __init__(self, patterns: list[str]) -> None:
        self._protect = [p for p in patterns if not p.startswith("!")]
        self._reinclude = [p[1:] for p in patterns if p.startswith("!")]

    def is_protected(self, relative_path: str) -> bool:
        normalized = relative_path.replace("\\", "/").lstrip("./")
        if any(self._matches(normalized, p) for p in self._reinclude):
            return False
        return any(self._matches(normalized, p) for p in self._protect)

    def _matches(self, path: str, pattern: str) -> bool:
        """Match a pattern against the whole path and against each of its segments.

        Segment matching is what makes ``.env`` protect ``config/.env`` and ``.idea/``
        protect everything beneath it. Without it, a pattern would only ever guard the
        repository root, and a nested secret would be deleted by a rule its author
        believed covered it.
        """
        if pattern.endswith("/"):
            directory = pattern.rstrip("/")
            segments = path.split("/")
            return directory in segments or fnmatch(path, directory)

        if fnmatch(path, pattern):
            return True
        return any(fnmatch(segment, pattern) for segment in path.split("/"))
