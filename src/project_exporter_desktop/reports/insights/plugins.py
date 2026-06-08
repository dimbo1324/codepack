from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class ReportPlugin:
    """Descriptor for one generated insight report.

    The current built-in reports are registered as plugins. New reports only need
    to provide filename, supported profiles and a writer callback. This keeps the
    orchestration layer flat while still making report addition/removal explicit.
    """

    filename: str
    profiles: set[str]
    writer: Callable[[Path], None]
    description: str = ''

    def should_run(self, profile: str) -> bool:
        return profile == 'full' or profile in self.profiles


def plugin_catalog(plugins: list[ReportPlugin]) -> list[dict[str, object]]:
    return [
        {
            'filename': plugin.filename,
            'profiles': sorted(plugin.profiles),
            'description': plugin.description,
        }
        for plugin in plugins
    ]
