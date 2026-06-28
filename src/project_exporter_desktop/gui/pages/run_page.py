from __future__ import annotations

from datetime import datetime

from PySide6.QtWidgets import (
    QLabel,
    QPlainTextEdit,
    QProgressBar,
    QVBoxLayout,
    QWidget,
)

from . import make_card, make_scroll_page


class _ProgressBar(QWidget):

    def __init__(self) -> None:
        super().__init__()
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        self._bar = QProgressBar()
        self._bar.setRange(0, 100)
        self._bar.setTextVisible(True)
        layout.addWidget(self._bar)

    def setValue(self, value: int) -> None:
        self._bar.setValue(value)


class RunPage(QWidget):

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)

        scroll, layout = make_scroll_page(
            "Журнал выполнения",
            "Экспорт выполняется в отдельном потоке. Прогресс, текущий этап и диагностические сообщения отображаются здесь.",
        )
        card, card_layout = make_card()

        self._progress_bar = _ProgressBar()
        card_layout.addWidget(self._progress_bar)

        self._stage_label = QLabel("Ожидание")
        self._stage_label.setObjectName("PageHint")
        self._current_item_label = QLabel("")
        self._current_item_label.setObjectName("PageHint")
        self._current_item_label.setWordWrap(True)
        card_layout.addWidget(self._stage_label)
        card_layout.addWidget(self._current_item_label)

        self._log_view = QPlainTextEdit()
        self._log_view.setReadOnly(True)
        self._log_view.setMinimumHeight(420)
        card_layout.addWidget(self._log_view, 1)

        layout.addWidget(card, 1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def reset(self) -> None:
        self._progress_bar.setValue(0)
        self._stage_label.setText("Начало экспорта...")
        self._current_item_label.clear()

    def append_log(self, message: str) -> None:
        timestamp = datetime.now().strftime("%H:%M:%S")
        self._log_view.appendPlainText(f"[{timestamp}] {message}")
        self._log_view.verticalScrollBar().setValue(self._log_view.verticalScrollBar().maximum())

    def set_progress(self, percent: int, stage: str, current: str) -> None:
        self._progress_bar.setValue(max(0, min(100, int(percent))))
        self._stage_label.setText(f"{percent}% — {stage}")
        self._current_item_label.setText(current)