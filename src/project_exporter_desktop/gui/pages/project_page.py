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

from . import make_card, make_scroll_page


class ProjectPage(QWidget):

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)

        scroll, layout = make_scroll_page(
            "Выбор проекта",
            "Выберите корневую папку проекта. При экспорте исходный проект не изменяется.",
        )

        card, card_layout = make_card()

        form = QGridLayout()
        form.setColumnStretch(0, 1)
        self.root_edit = QLineEdit()
        self.root_edit.setPlaceholderText(r"C:\Users\you\Desktop\my-project")
        browse = QPushButton("Обзор")
        browse.clicked.connect(self._browse_root)
        form.addWidget(QLabel("Папка проекта"), 0, 0, 1, 2)
        form.addWidget(self.root_edit, 1, 0)
        form.addWidget(browse, 1, 1)
        card_layout.addLayout(form)

        self._stack_label = QLabel("")
        self._stack_label.setObjectName("PageHint")
        self._stack_label.setWordWrap(True)
        card_layout.addWidget(self._stack_label)

        self.project_hint = QLabel(
            "По умолчанию исключаются .git, node_modules, виртуальные окружения,"
            " кэш, артефакты сборки и очевидные секреты."
        )
        self.project_hint.setObjectName("PageHint")
        self.project_hint.setWordWrap(True)
        card_layout.addWidget(self.project_hint)
        layout.addWidget(card)

        ctx_card, ctx_layout = make_card()
        ctx_title = QLabel("Контекст для ИИ")
        ctx_title.setObjectName("PageTitle")
        ctx_layout.addWidget(ctx_title)
        ctx_hint = QLabel(
            "Опишите задачу, проблему или вопрос, с которым поможет ИИ. "
            "Этот текст появится первым в текстовом дампе проекта — "
            "прямо перед исходным кодом."
        )
        ctx_hint.setObjectName("PageHint")
        ctx_hint.setWordWrap(True)
        ctx_layout.addWidget(ctx_hint)

        self.context_edit = QPlainTextEdit()
        self.context_edit.setPlaceholderText(
            "Пример: «Помоги разобраться в архитектуре этого проекта. "
            "Меня интересует, как работает авторизация и где хранятся пользователи.»"
        )
        self.context_edit.setMaximumHeight(120)
        ctx_layout.addWidget(self.context_edit)
        layout.addWidget(ctx_card)

        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)


    def _browse_root(self) -> None:
        initial = self.root_edit.text().strip() or str(Path.home())
        selected = QFileDialog.getExistingDirectory(self, "Выберите папку проекта", initial)
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