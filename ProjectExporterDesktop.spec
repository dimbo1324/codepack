# PyInstaller spec for building the desktop executable from the repository root.

from __future__ import annotations

from pathlib import Path

ROOT = Path(SPECPATH).resolve()
ENTRYPOINT = ROOT / "main.py"
SRC = ROOT / "src"
ICON = ROOT / "assets" / "ICO.ico"

DATA_FILES = []
if (ROOT / "assets").exists():
    DATA_FILES.append((str(ROOT / "assets"), "assets"))
if (SRC / "project_exporter_desktop" / "gui" / "styles").exists():
    DATA_FILES.append(
        (
            str(SRC / "project_exporter_desktop" / "gui" / "styles"),
            "project_exporter_desktop/gui/styles",
        )
    )


a = Analysis(
    [str(ENTRYPOINT)],
    pathex=[str(SRC)],
    binaries=[],
    datas=DATA_FILES,
    hiddenimports=["PySide6.QtCore", "PySide6.QtGui", "PySide6.QtWidgets"],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["pytest", "unittest"],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name="ProjectExporterDesktop",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=str(ICON) if ICON.exists() else None,
)
