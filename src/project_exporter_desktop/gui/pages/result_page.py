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

from ...i18n import t
from . import make_card, make_scroll_page


class ResultPage(QWidget):

    open_result_requested = Signal()
    open_desktop_requested = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)

        self._status_key = "result.not_created"
        self._last_log: str = ""

        scroll, layout, self._page_title, self._page_hint = make_scroll_page(
            t("result.page_title"),
            t("result.page_hint"),
        )
        card, card_layout = make_card()

        self._summary_status = QLabel(t("result.not_created"))
        self._summary_status.setWordWrap(True)

        self._summary_path = QLineEdit()
        self._summary_path.setReadOnly(True)
        self._summary_path.setPlaceholderText(t("result.path_placeholder"))

        self._open_button = QPushButton(t("result.btn_open"))
        self._open_button.clicked.connect(self.open_result_requested.emit)
        self._desktop_button = QPushButton(t("result.btn_desktop"))
        self._desktop_button.clicked.connect(self.open_desktop_requested.emit)

        open_row = QHBoxLayout()
        open_row.addWidget(self._open_button)
        open_row.addWidget(self._desktop_button)
        open_row.addStretch(1)

        self._path_label = QLabel(t("result.path_label"))
        card_layout.addWidget(self._summary_status)
        card_layout.addWidget(self._path_label)
        card_layout.addWidget(self._summary_path)
        card_layout.addLayout(open_row)

        layout.addWidget(card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def retranslate(self) -> None:
        self._page_title.setText(t("result.page_title"))
        self._page_hint.setText(t("result.page_hint"))
        self._path_label.setText(t("result.path_label"))
        self._summary_path.setPlaceholderText(t("result.path_placeholder"))
        self._open_button.setText(t("result.btn_open"))
        self._desktop_button.setText(t("result.btn_desktop"))
        if self._status_key == "result.failed":
            self._summary_status.setText(t("result.failed").format(log=self._last_log))
        else:
            self._summary_status.setText(t(self._status_key))

    def set_running(self) -> None:
        self._status_key = "result.running"
        self._summary_status.setText(t("result.running"))
        self._summary_path.clear()

    def set_success(self, path: Path | None = None) -> None:
        self._status_key = "result.success"
        self._summary_status.setText(t("result.success"))
        if path:
            self._summary_path.setText(str(path))

    def set_cancelled(self) -> None:
        self._status_key = "result.cancelled"
        self._summary_status.setText(t("result.cancelled"))

    def set_failed(self, log_file_path: str) -> None:
        self._status_key = "result.failed"
        self._last_log = log_file_path
        self._summary_status.setText(t("result.failed").format(log=log_file_path))

    def set_result_path(self, path: Path) -> None:
        self._summary_path.setText(str(path))
