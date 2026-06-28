from __future__ import annotations

from datetime import datetime


def now_stamp() -> str:
    return datetime.now().strftime("%Y%m%d_%H%M%S")


def human_now() -> str:
    return datetime.now().isoformat(sep=" ", timespec="seconds")