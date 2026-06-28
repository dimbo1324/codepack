from __future__ import annotations

from PySide6.QtCore import Qt, Signal
from PySide6.QtWidgets import (
    QFrame,
    QLabel,
    QPushButton,
    QVBoxLayout,
)

_NAV_LABELS = [
    "1  Проект",
    "2  Настройки",
    "3  Безопасность",
    "4  Предпросмотр",
    "5  Журнал",
    "6  Результат",
    "7  История",
    "8  Аналитика",
]


class Sidebar(QFrame):
    """Fixed-width left navigation panel."""

    page_requested = Signal(int)
    open_desktop_requested = Signal()

    def __init__(self, parent: QFrame | None = None) -> None:
        super().__init__(parent)
        self.setObjectName("Sidebar")
        self.setFixedWidth(250)

        self._nav_buttons: list[QPushButton] = []

        layout = QVBoxLayout(self)
        layout.setContentsMargins(18, 24, 18, 18)
        layout.setSpacing(10)

        title = QLabel("Project\nExporter")
        title.setObjectName("SidebarTitle")
        subtitle = QLabel("Снимок проекта для ИИ")
        subtitle.setObjectName("SidebarSubtitle")
        layout.addWidget(title)
        layout.addWidget(subtitle)
        layout.addSpacing(20)

        for index, text in enumerate(_NAV_LABELS):
            button = QPushButton(text)
            button.setObjectName("NavButton")
            button.setCursor(Qt.CursorShape.PointingHandCursor)
            button.clicked.connect(lambda _checked=False, i=index: self.page_requested.emit(i))
            self._nav_buttons.append(button)
            layout.addWidget(button)

        layout.addStretch(1)

        desktop_button = QPushButton("Рабочий стол")
        desktop_button.setObjectName("NavButton")
        desktop_button.clicked.connect(self.open_desktop_requested.emit)
        layout.addWidget(desktop_button)

    def set_active_page(self, index: int) -> None:
        for i, button in enumerate(self._nav_buttons):
            button.setProperty("active", i == index)
            button.style().unpolish(button)
            button.style().polish(button)