from __future__ import annotations

from pathlib import Path

from PySide6.QtCore import Signal
from PySide6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from . import make_card, make_scroll_page


class ResultPage(QWidget):
    """Page 5 — export result summary with path and quick-open buttons."""

    open_result_requested = Signal()
    open_desktop_requested = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)

        scroll, layout = make_scroll_page(
            "Result summary",
            "After completion, this page shows the final status and where the archive was created.",
        )
        card, card_layout = make_card()

        self._summary_status = QLabel("No export has been created yet.")
        self._summary_status.setWordWrap(True)

        self._summary_path = QLineEdit()
        self._summary_path.setReadOnly(True)
        self._summary_path.setPlaceholderText("Result path will appear here")

        open_button = QPushButton("Open result")
        open_button.clicked.connect(self.open_result_requested.emit)
        open_desktop = QPushButton("Open Desktop")
        open_desktop.clicked.connect(self.open_desktop_requested.emit)

        open_row = QHBoxLayout()
        open_row.addWidget(open_button)
        open_row.addWidget(open_desktop)
        open_row.addStretch(1)

        card_layout.addWidget(self._summary_status)
        card_layout.addWidget(QLabel("Result path"))
        card_layout.addWidget(self._summary_path)
        card_layout.addLayout(open_row)

        layout.addWidget(card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def set_running(self) -> None:
        self._summary_status.setText("Export is running...")
        self._summary_path.clear()

    def set_success(self, path: Path | None = None) -> None:
        self._summary_status.setText("Export completed successfully.")
        if path:
            self._summary_path.setText(str(path))

    def set_cancelled(self) -> None:
        self._summary_status.setText(
            "Export stopped by user. Partial output may have been created."
        )

    def set_failed(self, log_file_path: str) -> None:
        self._summary_status.setText(
            f"Export failed. Technical details were written to {log_file_path}."
        )

    def set_result_path(self, path: Path) -> None:
        self._summary_path.setText(str(path))
