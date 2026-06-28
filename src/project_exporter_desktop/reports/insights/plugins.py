# Insight report generator: reads the copied project and writes one focused analysis artifact into reports/insights.

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class ReportPlugin:
    filename: str
    profiles: set[str]
    writer: Callable[[Path], None]
    description: str = ""

    def should_run(self, profile: str) -> bool:
        return profile == "full" or profile in self.profiles


def plugin_catalog(plugins: list[ReportPlugin]) -> list[dict[str, object]]:
    return [
        {
            "filename": plugin.filename,
            "profiles": sorted(plugin.profiles),
            "description": plugin.description,
        }
        for plugin in plugins
    ]
