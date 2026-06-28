# PySide6 wizard page module: owns one visible screen and emits user actions back to MainWindow.

from __future__ import annotations

from pathlib import Path

from PySide6.QtWidgets import (
    QFileDialog,
    QGridLayout,
    QLabel,
    QLineEdit,
    QPlainTextEdit,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from ...i18n import t
from . import make_card, make_scroll_page


class ProjectPage(QWidget):
    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)

        scroll, layout, self._page_title, self._page_hint = make_scroll_page(
            t("project.page_title"),
            t("project.page_hint"),
        )

        card, card_layout = make_card()

        form = QGridLayout()
        form.setColumnStretch(0, 1)
        self.root_edit = QLineEdit()
        self.root_edit.setPlaceholderText(r"C:\Users\you\Desktop\my-project")
        self._browse_btn = QPushButton(t("project.browse"))
        self._browse_btn.clicked.connect(self._browse_root)
        self._folder_label = QLabel(t("project.folder_label"))
        form.addWidget(self._folder_label, 0, 0, 1, 2)
        form.addWidget(self.root_edit, 1, 0)
        form.addWidget(self._browse_btn, 1, 1)
        card_layout.addLayout(form)

        self._stack_label = QLabel("")
        self._stack_label.setObjectName("PageHint")
        self._stack_label.setWordWrap(True)
        card_layout.addWidget(self._stack_label)

        self.project_hint = QLabel(t("project.default_hint"))
        self.project_hint.setObjectName("PageHint")
        self.project_hint.setWordWrap(True)
        card_layout.addWidget(self.project_hint)
        layout.addWidget(card)

        ctx_card, ctx_layout = make_card()
        self._ctx_title = QLabel(t("project.ctx_title"))
        self._ctx_title.setObjectName("PageTitle")
        ctx_layout.addWidget(self._ctx_title)
        self._ctx_hint = QLabel(t("project.ctx_hint"))
        self._ctx_hint.setObjectName("PageHint")
        self._ctx_hint.setWordWrap(True)
        ctx_layout.addWidget(self._ctx_hint)

        self.context_edit = QPlainTextEdit()
        self.context_edit.setPlaceholderText(t("project.ctx_placeholder"))
        self.context_edit.setMaximumHeight(120)
        ctx_layout.addWidget(self.context_edit)
        layout.addWidget(ctx_card)

        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def retranslate(self) -> None:
        self._page_title.setText(t("project.page_title"))
        self._page_hint.setText(t("project.page_hint"))
        self._browse_btn.setText(t("project.browse"))
        self._folder_label.setText(t("project.folder_label"))
        self.project_hint.setText(t("project.default_hint"))
        self._ctx_title.setText(t("project.ctx_title"))
        self._ctx_hint.setText(t("project.ctx_hint"))
        self.context_edit.setPlaceholderText(t("project.ctx_placeholder"))

    def _browse_root(self) -> None:
        initial = self.root_edit.text().strip() or str(Path.home())
        selected = QFileDialog.getExistingDirectory(self, t("project.browse_dialog"), initial)
        if selected:
            self.root_edit.setText(selected)

    def get_root(self) -> str:
        return self.root_edit.text().strip()

    def set_root(self, text: str) -> None:
        self.root_edit.setText(text)

    def get_developer_context(self) -> str:
        return self.context_edit.toPlainText().strip()

    def set_developer_context(self, text: str) -> None:
        self.context_edit.setPlainText(text)

    def set_detected_stack(self, info: str) -> None:
        self._stack_label.setText(info)
