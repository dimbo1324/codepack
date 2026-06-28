
from __future__ import annotations

from pathlib import Path

SRC = Path(__file__).resolve().parents[1] / "src" / "project_exporter_desktop"


def test_page_utilities_importable() -> None:
    from project_exporter_desktop.gui.pages import (
        make_card,
        make_scroll_page,
        set_combo_value,
        wrap_layout,
    )


def test_services_do_not_import_gui() -> None:
    gui_patterns = ("from ..gui", "from .gui", "import gui", "from gui")
    violations: list[str] = []
    for directory in [SRC / "services", SRC / "reports"]:
        for py_file in directory.rglob("*.py"):
            text = py_file.read_text(encoding="utf-8", errors="ignore")
            if any(pat in text for pat in gui_patterns):
                violations.append(py_file.name)
    assert violations == [], f"GUI imported inside service/report modules: {violations}"


def test_pages_do_not_import_export_services() -> None:
    forbidden = ("from ...services", "from ..services", "from project_exporter_desktop.services")
    violations: list[str] = []
    pages_dir = SRC / "gui" / "pages"
    for py_file in pages_dir.rglob("*.py"):
        text = py_file.read_text(encoding="utf-8", errors="ignore")
        if any(pat in text for pat in forbidden):
            violations.append(py_file.name)
    assert violations == [], f"Page modules import export services directly: {violations}"