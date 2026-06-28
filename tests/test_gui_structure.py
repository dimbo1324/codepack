"""Tests that verify the GUI module structure: page utilities exist and service/GUI layers stay separate."""

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

    assert callable(make_card)
    assert callable(make_scroll_page)
    assert callable(set_combo_value)
    assert callable(wrap_layout)


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


def test_tray_menu_has_explicit_selected_styles() -> None:
    styles_dir = SRC / "gui" / "styles"
    for qss_file in ("app_dark.qss", "app_light.qss"):
        text = (styles_dir / qss_file).read_text(encoding="utf-8")
        assert "QMenu#TrayMenu" in text
        assert "QMenu#TrayMenu::item:selected" in text
        assert "background: #2563eb" in text


def test_build_and_install_script_runs_full_interactive_packaging_flow() -> None:
    script = SRC.parents[1] / "build_and_install.bat"
    text = script.read_text(encoding="utf-8")
    assert "call tools\\build_exe.bat" in text
    assert "call tools\\build_setup.bat" in text
    assert "ProjectExporterDesktopSetup-*.exe" in text
    assert 'start "" "%SETUP_EXE%"' in text
