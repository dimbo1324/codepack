# Local convenience wrapper for building the Windows executable from the repository root.

from __future__ import annotations

from tools.build_exe import main

if __name__ == "__main__":
    raise SystemExit(main())
