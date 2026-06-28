from __future__ import annotations

from PySide6.QtCore import Qt, Signal
from PySide6.QtWidgets import (
    QFrame,
    QLabel,
    QPushButton,
    QVBoxLayout,
)

from ...i18n import t

_NAV_KEYS = [
    "nav.1", "nav.2", "nav.3", "nav.4",
    "nav.5", "nav.6", "nav.7", "nav.8",
]


class Sidebar(QFrame):

    page_requested = Signal(int)
    open_desktop_requested = Signal()

    def __init__(self, parent: QFrame | None = None) -> None:
        super().__init__(parent)
        self.setObjectName("Sidebar")
        self.setFixedWidth(200)

        self._nav_buttons: list[QPushButton] = []

        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 20, 12, 14)
        layout.setSpacing(6)

        title = QLabel("Project\nExporter")
        title.setObjectName("SidebarTitle")
        self._subtitle = QLabel(t("sidebar.subtitle"))
        self._subtitle.setObjectName("SidebarSubtitle")
        layout.addWidget(title)
        layout.addWidget(self._subtitle)
        layout.addSpacing(20)

        for index, key in enumerate(_NAV_KEYS):
            button = QPushButton(t(key))
            button.setObjectName("NavButton")
            button.setCursor(Qt.CursorShape.PointingHandCursor)
            button.clicked.connect(lambda _checked=False, i=index: self.page_requested.emit(i))
            self._nav_buttons.append(button)
            layout.addWidget(button)

        layout.addStretch(1)

        self._desktop_button = QPushButton(t("sidebar.desktop"))
        self._desktop_button.setObjectName("NavButton")
        self._desktop_button.clicked.connect(self.open_desktop_requested.emit)
        layout.addWidget(self._desktop_button)

    def retranslate(self) -> None:
        self._subtitle.setText(t("sidebar.subtitle"))
        for i, key in enumerate(_NAV_KEYS):
            self._nav_buttons[i].setText(t(key))
        self._desktop_button.setText(t("sidebar.desktop"))

    def set_active_page(self, index: int) -> None:
        for i, button in enumerate(self._nav_buttons):
            button.setProperty("active", i == index)
            button.style().unpolish(button)
            button.style().polish(button)
