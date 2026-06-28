# Build helper used by release packaging to produce the PyInstaller executable.

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "build" / "pyinstaller" / "ProjectExporterDesktop.spec"


def main() -> int:
    if not SPEC.exists():
        print(f"PyInstaller spec not found: {SPEC}", file=sys.stderr)
        return 2
    command = [sys.executable, "-m", "PyInstaller", "--noconfirm", "--clean", str(SPEC)]
    print("Running:", " ".join(command))
    return subprocess.call(command, cwd=str(ROOT))


if __name__ == "__main__":
    raise SystemExit(main())
