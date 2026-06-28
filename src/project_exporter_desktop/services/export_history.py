# Persists a rolling log of completed exports to a JSON file in the user settings directory.
# Used by diff_service to locate the snapshot from the previous successful export.

from __future__ import annotations

import json
import logging
from typing import Any

from ..constants import SETTINGS_FILE

HISTORY_FILE = SETTINGS_FILE.with_name(".project_exporter_history.json")
MAX_HISTORY_ITEMS = 50
logger = logging.getLogger(__name__)


def append_export_history(entry: dict[str, Any]) -> None:
    try:
        history = (
            json.loads(HISTORY_FILE.read_text(encoding="utf-8")) if HISTORY_FILE.exists() else []
        )
        if not isinstance(history, list):
            history = []
        history.insert(0, entry)  # newest entry first so callers can short-circuit on the first match
        del history[MAX_HISTORY_ITEMS:]
        HISTORY_FILE.write_text(json.dumps(history, ensure_ascii=False, indent=2), encoding="utf-8")
    except Exception as exc:
        logger.warning("Не удалось обновить историю экспортов: %s", exc)


def load_export_history() -> list[dict[str, Any]]:
    if not HISTORY_FILE.exists():
        return []
    try:
        value = json.loads(HISTORY_FILE.read_text(encoding="utf-8"))
        return value if isinstance(value, list) else []
    except Exception as exc:
        logger.warning("Не удалось загрузить историю экспортов: %s", exc)
        return []
