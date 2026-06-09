from __future__ import annotations

import sys

from .constants import APP_NAME


def main() -> None:
    try:
        from .gui.main_window import run_app
    except ModuleNotFoundError as exc:
        if exc.name and exc.name.startswith("PySide6"):
            raise SystemExit(
                f"{APP_NAME} requires PySide6. Install dependencies with: "
                "python -m pip install -r requirements.txt"
            ) from exc
        raise

    raise SystemExit(run_app())


if __name__ == "__main__":
    main()
