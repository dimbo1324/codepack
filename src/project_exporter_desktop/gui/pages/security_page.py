from __future__ import annotations

from PySide6.QtCore import Signal
from PySide6.QtWidgets import (
    QCheckBox,
    QComboBox,
    QFormLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from ...config import Config
from ...constants import SAFE_EXPORT_MODES
from . import make_card, make_scroll_page, set_combo_value, wrap_layout


class SecurityPage(QWidget):
    """Page 3 — safe export mode, redaction and custom rules."""

    # Emitted when a tool action button is clicked; MainWindow handles the action.
    edit_rules_requested = Signal()
    edit_prompt_goals_requested = Signal()
    create_exportignore_requested = Signal()
    open_profiles_json_requested = Signal()
    export_settings_requested = Signal()
    import_settings_requested = Signal()
    reset_settings_requested = Signal()
    show_history_requested = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)

        scroll, layout = make_scroll_page(
            "Safety and filtering",
            "Configure safe export policy, redaction, Git patch inclusion and custom include/exclude rules.",
        )
        card, card_layout = make_card()
        form = QFormLayout()

        self.safe_mode_combo = QComboBox()
        self.safe_mode_combo.addItems(list(SAFE_EXPORT_MODES.keys()))
        self.safe_mode_combo.currentTextChanged.connect(self._sync_safe_hint)
        self.safe_hint = QLabel("")
        self.safe_hint.setObjectName("PageHint")
        safe_block = QVBoxLayout()
        safe_block.addWidget(self.safe_mode_combo)
        safe_block.addWidget(self.safe_hint)
        form.addRow("Safe Export mode", wrap_layout(safe_block))

        self.redact_checkbox = QCheckBox("Redact obvious secrets in text and Git reports")
        self.git_patch_checkbox = QCheckBox(
            "Include full Git patch; disabled by default because patches may contain secrets"
        )
        self.include_project_checkbox = QCheckBox("Include copied project in final ZIP")
        self.keep_staging_checkbox = QCheckBox("Keep staging folder after export")
        form.addRow("Redaction", self.redact_checkbox)
        form.addRow("Git patch", self.git_patch_checkbox)
        form.addRow("Project files", self.include_project_checkbox)
        form.addRow("Staging", self.keep_staging_checkbox)

        self.extra_ignored_edit = QLineEdit()
        self.extra_ignored_edit.setPlaceholderText(".cache, tmp, vendor")
        form.addRow("Extra ignored dirs", self.extra_ignored_edit)

        card_layout.addLayout(form)

        actions = QHBoxLayout()
        for text, signal in [
            ("Edit include/exclude rules", self.edit_rules_requested),
            ("Prompt Builder", self.edit_prompt_goals_requested),
            ("Create .exportignore", self.create_exportignore_requested),
            ("Profiles JSON", self.open_profiles_json_requested),
        ]:
            button = QPushButton(text)
            button.clicked.connect(signal.emit)
            actions.addWidget(button)
        actions.addStretch(1)
        card_layout.addLayout(actions)

        settings_actions = QHBoxLayout()
        for text, signal in [
            ("Export settings", self.export_settings_requested),
            ("Import settings", self.import_settings_requested),
            ("Reset settings", self.reset_settings_requested),
            ("Recent exports", self.show_history_requested),
        ]:
            button = QPushButton(text)
            button.clicked.connect(signal.emit)
            settings_actions.addWidget(button)
        settings_actions.addStretch(1)
        card_layout.addLayout(settings_actions)

        layout.addWidget(card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def _sync_safe_hint(self) -> None:
        mode = self.safe_mode_combo.currentText().strip()
        self.safe_hint.setText(SAFE_EXPORT_MODES.get(mode, ""))

    def load_from_config(self, config: Config) -> None:
        self.redact_checkbox.setChecked(config.redact_secrets)
        self.git_patch_checkbox.setChecked(config.include_git_patch)
        self.include_project_checkbox.setChecked(config.include_project_in_zip)
        self.keep_staging_checkbox.setChecked(config.keep_staging_folder)
        self.extra_ignored_edit.setText(", ".join(config.extra_ignored_dirs))
        set_combo_value(self.safe_mode_combo, config.normalized_safe_export_mode())
