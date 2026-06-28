from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass


@dataclass(slots=True)
class ProgressReporter:
    """Small UI-agnostic progress adapter.

    Tkinter receives plain log messages through the existing queue. Messages
    starting with PROGRESS\t are parsed by the UI; all other consumers can ignore
    them safely.
    """

    log: Callable[[str], None]
    total_steps: int = 8
    current_step: int = 0

    def step(self, title: str, current: str = "") -> None:
        self.current_step += 1
        percent = min(100, round((self.current_step - 1) / max(1, self.total_steps) * 100))
        self.update(percent, title, current)
        self.log(title)

    def update(self, percent: int, stage: str, current: str = "") -> None:
        percent = max(0, min(100, int(percent)))
        self.log(f"PROGRESS\t{percent}\t{stage}\t{current}")

    def done(self, stage: str = "Done") -> None:
        self.update(100, stage, "")