from __future__ import annotations

from pathlib import Path

from PySide6.QtWidgets import (
    QFileDialog,
    QGridLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from . import make_card, make_scroll_page


class ProjectPage(QWidget):
    """Page 1 — project root selection."""

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)

        scroll, layout = make_scroll_page(
            "Select project",
            "Choose the source project folder. The original project is read-only during normal export.",
        )
        card, card_layout = make_card()

        form = QGridLayout()
        form.setColumnStretch(0, 1)
        self.root_edit = QLineEdit()
        self.root_edit.setPlaceholderText(r"C:\Users\you\Desktop\my-project")
        browse = QPushButton("Browse")
        browse.clicked.connect(self._browse_root)
        form.addWidget(QLabel("Project root"), 0, 0, 1, 2)
        form.addWidget(self.root_edit, 1, 0)
        form.addWidget(browse, 1, 1)
        card_layout.addLayout(form)

        self.project_hint = QLabel(
            "Safe defaults exclude .git, node_modules, virtual environments,"
            " caches, build artefacts and obvious secrets."
        )
        self.project_hint.setObjectName("PageHint")
        self.project_hint.setWordWrap(True)
        card_layout.addWidget(self.project_hint)
        layout.addWidget(card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def _browse_root(self) -> None:
        initial = self.root_edit.text().strip() or str(Path.home())
        selected = QFileDialog.getExistingDirectory(self, "Select project root", initial)
        if selected:
            self.root_edit.setText(selected)

    def get_root(self) -> str:
        return self.root_edit.text().strip()

    def set_root(self, text: str) -> None:
        self.root_edit.setText(text)
