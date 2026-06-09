from __future__ import annotations

import sys
from pathlib import Path


def project_root() -> Path:
    """Return the project/resource root both in source and frozen builds."""
    frozen_root = getattr(sys, "_MEIPASS", None)
    if frozen_root:
        return Path(frozen_root)
    return Path(__file__).resolve().parents[3]


def resource_path(*parts: str) -> Path:
    return project_root().joinpath(*parts)


def asset_path(*parts: str) -> Path:
    return resource_path("assets", *parts)


def style_path(name: str = "app.qss") -> Path:
    return resource_path("project_exporter_desktop", "gui", "styles", name)


def read_text_resource(path: Path, default: str = "") -> str:
    try:
        if path.exists():
            return path.read_text(encoding="utf-8")
    except Exception:
        return default
    return default
