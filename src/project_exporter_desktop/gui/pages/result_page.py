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
    """Страница 5 — итог экспорта с путём к архиву и кнопками быстрого доступа."""

    open_result_requested = Signal()
    open_desktop_requested = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)

        scroll, layout = make_scroll_page(
            "Итог экспорта",
            "После завершения здесь отображается финальный статус и путь к созданному архиву.",
        )
        card, card_layout = make_card()

        self._summary_status = QLabel("Экспорт ещё не создавался.")
        self._summary_status.setWordWrap(True)

        self._summary_path = QLineEdit()
        self._summary_path.setReadOnly(True)
        self._summary_path.setPlaceholderText("Здесь появится путь к архиву")

        open_button = QPushButton("Открыть результат")
        open_button.clicked.connect(self.open_result_requested.emit)
        open_desktop = QPushButton("Рабочий стол")
        open_desktop.clicked.connect(self.open_desktop_requested.emit)

        open_row = QHBoxLayout()
        open_row.addWidget(open_button)
        open_row.addWidget(open_desktop)
        open_row.addStretch(1)

        card_layout.addWidget(self._summary_status)
        card_layout.addWidget(QLabel("Путь к результату"))
        card_layout.addWidget(self._summary_path)
        card_layout.addLayout(open_row)

        layout.addWidget(card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def set_running(self) -> None:
        self._summary_status.setText("Выполняется экспорт...")
        self._summary_path.clear()

    def set_success(self, path: Path | None = None) -> None:
        self._summary_status.setText("Экспорт завершён успешно.")
        if path:
            self._summary_path.setText(str(path))

    def set_cancelled(self) -> None:
        self._summary_status.setText(
            "Экспорт отменён пользователем. Часть результата могла быть создана."
        )

    def set_failed(self, log_file_path: str) -> None:
        self._summary_status.setText(
            f"Ошибка экспорта. Технические подробности записаны в {log_file_path}."
        )

    def set_result_path(self, path: Path) -> None:
        self._summary_path.setText(str(path))