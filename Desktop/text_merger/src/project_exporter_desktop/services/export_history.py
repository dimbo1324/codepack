from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from ..constants import SETTINGS_FILE

HISTORY_FILE = SETTINGS_FILE.with_name(".project_exporter_history.json")
MAX_HISTORY_ITEMS = 50


def append_export_history(entry: dict[str, Any]) -> None:
    try:
        history = json.loads(HISTORY_FILE.read_text(encoding="utf-8")) if HISTORY_FILE.exists() else []
        if not isinstance(history, list):
            history = []
        history.insert(0, entry)
        del history[MAX_HISTORY_ITEMS:]
        HISTORY_FILE.write_text(json.dumps(history, ensure_ascii=False, indent=2), encoding="utf-8")
    except Exception:
        pass


def load_export_history() -> list[dict[str, Any]]:
    try:
        value = json.loads(HISTORY_FILE.read_text(encoding="utf-8"))
        return value if isinstance(value, list) else []
    except Exception:
        return []
