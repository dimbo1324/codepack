# Thin progress-reporting helper that emits structured PROGRESS tab-delimited log lines.
# The GUI worker thread parses these lines to update the progress bar and status label.

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass


@dataclass(slots=True)
class ProgressReporter:

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
        self.log(f"PROGRESS\t{percent}\t{stage}\t{current}")  # tab-delimited sentinel parsed by the GUI worker

    def done(self, stage: str = "Done") -> None:
        self.update(100, stage, "")