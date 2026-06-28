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


def test_application_menus_have_hover_and_shortcut_spacing_styles() -> None:
    styles_dir = SRC / "gui" / "styles"
    for qss_file in ("app_dark.qss", "app_light.qss"):
        text = (styles_dir / qss_file).read_text(encoding="utf-8")
        assert "QMenuBar::item:selected" in text
        assert "QMenu::item:selected" in text
        assert "min-width: 260px" in text
        assert "padding: 8px 90px 8px 18px" in text
        assert "QLineEdit:hover" in text
        assert "QCheckBox:hover" in text


def test_main_window_retranslates_menu_bar_and_zoom_shortcuts() -> None:
    text = (SRC / "gui" / "main_window.py").read_text(encoding="utf-8")
    assert "self.menuBar().clear()" in text
    assert "self._build_menu()" in text
    assert "self.addAction(zoom_in_act)" in text
    assert "self.removeAction(action)" in text
    assert 'QKeySequence("Ctrl+=")' in text
    assert 'QKeySequence("Ctrl++")' in text
    assert "QKeySequence.StandardKey.ZoomIn" in text


def test_zoom_scales_stylesheet_font_sizes() -> None:
    from project_exporter_desktop.gui.main_window import _scaled_stylesheet

    qss = "QWidget { font-size: 10pt; } QLabel { font-size: 20pt; }"
    assert "font-size: 15pt;" in _scaled_stylesheet(qss, 1.5)
    assert "font-size: 30pt;" in _scaled_stylesheet(qss, 1.5)
    assert "font-size: 7pt;" in _scaled_stylesheet(qss, 0.7)
    assert "font-size: 14pt;" in _scaled_stylesheet(qss, 0.7)


def test_full_uninstall_script_requires_explicit_delete_confirmation() -> None:
    root = SRC.parents[1]
    batch_text = (root / "uninstall_project_exporter.bat").read_text(encoding="utf-8")
    ps_text = (root / "tools" / "uninstall_project_exporter.ps1").read_text(encoding="utf-8")
    assert "tools\\uninstall_project_exporter.ps1" in batch_text
    assert "Type DELETE to continue" in ps_text
    assert '$answer -ne "DELETE"' in ps_text
    assert "Get-InstallEntries" in ps_text
    assert "Remove-InstallDirectory" in ps_text


def test_build_and_install_script_runs_full_interactive_packaging_flow() -> None:
    script = SRC.parents[1] / "build_and_install.bat"
    text = script.read_text(encoding="utf-8")
    assert "call tools\\build_exe.bat" in text
    assert "call tools\\build_setup.bat" in text
    assert "ProjectExporterDesktopSetup-*.exe" in text
    assert 'start "" "%SETUP_EXE%"' in text
