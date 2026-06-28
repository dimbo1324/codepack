from __future__ import annotations

import os
from datetime import datetime
from pathlib import Path


def app_log_dir() -> Path:
    local_app_data = os.environ.get("LOCALAPPDATA")
    if local_app_data:
        return Path(local_app_data) / "ProjectExporterDesktop" / "logs"
    return Path.home() / ".project_exporter_desktop" / "logs"


def app_log_file() -> Path:
    return app_log_dir() / "app.log"


def append_app_log(message: str) -> Path:
    log_file = app_log_file()
    try:
        log_file.parent.mkdir(parents=True, exist_ok=True)
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        with log_file.open("a", encoding="utf-8", newline="\n") as file:
            file.write(f"[{timestamp}] {message}\n")
    except Exception:
        return log_file
    return log_file