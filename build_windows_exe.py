from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
ENTRYPOINT = ROOT / 'main.py'
ICON = ROOT / 'assets' / 'ICO.ico'


def main() -> int:
    args = [
        sys.executable,
        '-m',
        'PyInstaller',
        '--noconfirm',
        '--clean',
        '--onefile',
        '--windowed',
        '--name',
        'ProjectExporterDesktop',
    ]
    if ICON.exists():
        args.extend(['--icon', str(ICON)])
    args.append(str(ENTRYPOINT))
    print('Running:', ' '.join(args))
    return subprocess.call(args, cwd=str(ROOT))


if __name__ == '__main__':
    raise SystemExit(main())
