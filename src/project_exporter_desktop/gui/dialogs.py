from __future__ import annotations

import re
from collections.abc import Iterable

from PySide6.QtWidgets import (
    QCheckBox,
    QDialog,
    QDialogButtonBox,
    QGridLayout,
    QHBoxLayout,
    QLabel,
    QPlainTextEdit,
    QPushButton,
    QScrollArea,
    QTextEdit,
    QVBoxLayout,
    QWidget,
)

from ..services.prompt_builder import PROMPT_GOALS


def _split_rules(text: str) -> list[str]:
    values: list[str] = []
    for token in re.split(r"[,;\n]", text):
        value = token.strip()
        if value and value not in values:
            values.append(value)
    return values


class ExportPlanDialog(QDialog):
    def __init__(self, preview_text: str, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setWindowTitle("Export Plan confirmation")
        self.setMinimumSize(760, 560)

        layout = QVBoxLayout(self)
        title = QLabel("Review Export Plan before copying")
        title.setObjectName("PageTitle")
        hint = QLabel(
            "The project will not be copied until you confirm this plan. "
            "Safe Export exclusions remain active even if custom include rules exist."
        )
        hint.setWordWrap(True)
        hint.setObjectName("PageHint")

        self.preview = QPlainTextEdit()
        self.preview.setReadOnly(True)
        self.preview.setPlainText(preview_text)
        self.preview.setLineWrapMode(QPlainTextEdit.LineWrapMode.NoWrap)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Cancel | QDialogButtonBox.StandardButton.Ok
        )
        buttons.button(QDialogButtonBox.StandardButton.Ok).setText("Continue export")
        buttons.button(QDialogButtonBox.StandardButton.Cancel).setText("Cancel")
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)

        layout.addWidget(title)
        layout.addWidget(hint)
        layout.addWidget(self.preview, 1)
        layout.addWidget(buttons)


class RulesDialog(QDialog):
    def __init__(
        self,
        excluded_files: Iterable[str],
        excluded_extensions: Iterable[str],
        always_include_files: Iterable[str],
        always_include_dirs: Iterable[str],
        parent: QWidget | None = None,
    ) -> None:
        super().__init__(parent)
        self.setWindowTitle("Custom include / exclude rules")
        self.setMinimumSize(780, 640)

        self.excluded_files = QTextEdit()
        self.excluded_extensions = QTextEdit()
        self.always_include_files = QTextEdit()
        self.always_include_dirs = QTextEdit()

        fields = [
            ("Exclude files / glob patterns", self.excluded_files, excluded_files),
            ("Exclude extensions", self.excluded_extensions, excluded_extensions),
            (
                "Always include files / glob patterns",
                self.always_include_files,
                always_include_files,
            ),
            ("Always include directories", self.always_include_dirs, always_include_dirs),
        ]

        layout = QVBoxLayout(self)
        intro = QLabel(
            "Enter one item per line or comma-separated. Safe Export can still block risky files."
        )
        intro.setObjectName("PageHint")
        layout.addWidget(intro)

        grid = QGridLayout()
        for row, (label, widget, values) in enumerate(fields):
            widget.setAcceptRichText(False)
            widget.setPlainText("\n".join(values))
            grid.addWidget(QLabel(label), row * 2, 0)
            grid.addWidget(widget, row * 2 + 1, 0)
        layout.addLayout(grid, 1)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Cancel | QDialogButtonBox.StandardButton.Save
        )
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

    def values(self) -> dict[str, list[str]]:
        return {
            "excluded_files": _split_rules(self.excluded_files.toPlainText()),
            "excluded_extensions": _split_rules(self.excluded_extensions.toPlainText()),
            "always_include_files": _split_rules(self.always_include_files.toPlainText()),
            "always_include_dirs": _split_rules(self.always_include_dirs.toPlainText()),
        }


class PromptGoalsDialog(QDialog):
    def __init__(self, selected_goals: Iterable[str], parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setWindowTitle("Prompt Builder goals")
        self.setMinimumSize(660, 420)
        selected = set(selected_goals)
        self.checkboxes: dict[str, QCheckBox] = {}

        layout = QVBoxLayout(self)
        hint = QLabel("Choose which goals should be included in AI_PROMPTS/CUSTOM_PROMPT.md.")
        hint.setObjectName("PageHint")
        layout.addWidget(hint)

        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        body = QWidget()
        body_layout = QVBoxLayout(body)
        for key, description in PROMPT_GOALS.items():
            checkbox = QCheckBox(f"{key}: {description}")
            checkbox.setChecked(key in selected)
            self.checkboxes[key] = checkbox
            body_layout.addWidget(checkbox)
        body_layout.addStretch(1)
        scroll.setWidget(body)
        layout.addWidget(scroll, 1)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Cancel | QDialogButtonBox.StandardButton.Save
        )
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

    def goals(self) -> list[str]:
        values = [key for key, checkbox in self.checkboxes.items() if checkbox.isChecked()]
        return values or ["architecture_review", "bug_hunt", "write_tests"]


class HistoryDialog(QDialog):
    def __init__(self, history: list[dict[str, object]], parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setWindowTitle("Recent exports")
        self.setMinimumSize(720, 460)
        layout = QVBoxLayout(self)
        title = QLabel("Recent export history")
        title.setObjectName("PageTitle")
        layout.addWidget(title)
        text = QPlainTextEdit()
        text.setReadOnly(True)
        if not history:
            text.setPlainText("Export history is empty.")
        else:
            blocks: list[str] = []
            for item in history[:15]:
                blocks.append(
                    "\n".join(
                        [
                            str(item.get("generated_at", "")),
                            f"Project: {item.get('project_name', '')}",
                            f"Profile: {item.get('profile', '')}; Safe: {item.get('safe_export_mode', '')}; Split: {item.get('split_archives', False)}",
                            f"Result: {item.get('result', '')}",
                        ]
                    )
                )
            text.setPlainText("\n\n".join(blocks))
        layout.addWidget(text, 1)
        close = QPushButton("Close")
        close.clicked.connect(self.accept)
        row = QHBoxLayout()
        row.addStretch(1)
        row.addWidget(close)
        layout.addLayout(row)
