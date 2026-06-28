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
from ...i18n import t
from . import make_card, make_scroll_page, set_combo_value, wrap_layout


class SecurityPage(QWidget):

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

        scroll, layout, self._page_title, self._page_hint = make_scroll_page(
            t("security.page_title"),
            t("security.page_hint"),
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
        self._lbl_safe_mode = QLabel(t("security.lbl_safe_mode"))
        form.addRow(self._lbl_safe_mode, wrap_layout(safe_block))

        self.redact_checkbox = QCheckBox(t("security.redact"))
        self.git_patch_checkbox = QCheckBox(t("security.git_patch"))
        self.include_project_checkbox = QCheckBox(t("security.include_project"))
        self.keep_staging_checkbox = QCheckBox(t("security.keep_staging"))
        self._lbl_redact = QLabel(t("security.lbl_redact"))
        self._lbl_git_patch = QLabel(t("security.lbl_git_patch"))
        self._lbl_project_files = QLabel(t("security.lbl_project_files"))
        self._lbl_staging = QLabel(t("security.lbl_staging"))
        form.addRow(self._lbl_redact, self.redact_checkbox)
        form.addRow(self._lbl_git_patch, self.git_patch_checkbox)
        form.addRow(self._lbl_project_files, self.include_project_checkbox)
        form.addRow(self._lbl_staging, self.keep_staging_checkbox)

        self.extra_ignored_edit = QLineEdit()
        self.extra_ignored_edit.setPlaceholderText(".cache, tmp, vendor")
        self._lbl_extra_ignore = QLabel(t("security.lbl_extra_ignore"))
        form.addRow(self._lbl_extra_ignore, self.extra_ignored_edit)

        card_layout.addLayout(form)

        actions = QHBoxLayout()
        self._btn_rules = QPushButton(t("security.btn_rules"))
        self._btn_rules.clicked.connect(self.edit_rules_requested.emit)
        self._btn_goals = QPushButton(t("security.btn_goals"))
        self._btn_goals.clicked.connect(self.edit_prompt_goals_requested.emit)
        self._btn_exportignore = QPushButton(t("security.btn_exportignore"))
        self._btn_exportignore.clicked.connect(self.create_exportignore_requested.emit)
        self._btn_profiles = QPushButton(t("security.btn_profiles"))
        self._btn_profiles.clicked.connect(self.open_profiles_json_requested.emit)
        for btn in (self._btn_rules, self._btn_goals, self._btn_exportignore, self._btn_profiles):
            actions.addWidget(btn)
        actions.addStretch(1)
        card_layout.addLayout(actions)

        settings_actions = QHBoxLayout()
        self._btn_export_settings = QPushButton(t("security.btn_export_settings"))
        self._btn_export_settings.clicked.connect(self.export_settings_requested.emit)
        self._btn_import_settings = QPushButton(t("security.btn_import_settings"))
        self._btn_import_settings.clicked.connect(self.import_settings_requested.emit)
        self._btn_reset_settings = QPushButton(t("security.btn_reset_settings"))
        self._btn_reset_settings.clicked.connect(self.reset_settings_requested.emit)
        self._btn_history = QPushButton(t("security.btn_history"))
        self._btn_history.clicked.connect(self.show_history_requested.emit)
        for btn in (
            self._btn_export_settings,
            self._btn_import_settings,
            self._btn_reset_settings,
            self._btn_history,
        ):
            settings_actions.addWidget(btn)
        settings_actions.addStretch(1)
        card_layout.addLayout(settings_actions)

        layout.addWidget(card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def retranslate(self) -> None:
        self._page_title.setText(t("security.page_title"))
        self._page_hint.setText(t("security.page_hint"))
        self._lbl_safe_mode.setText(t("security.lbl_safe_mode"))
        self._lbl_redact.setText(t("security.lbl_redact"))
        self._lbl_git_patch.setText(t("security.lbl_git_patch"))
        self._lbl_project_files.setText(t("security.lbl_project_files"))
        self._lbl_staging.setText(t("security.lbl_staging"))
        self._lbl_extra_ignore.setText(t("security.lbl_extra_ignore"))
        self.redact_checkbox.setText(t("security.redact"))
        self.git_patch_checkbox.setText(t("security.git_patch"))
        self.include_project_checkbox.setText(t("security.include_project"))
        self.keep_staging_checkbox.setText(t("security.keep_staging"))
        self._btn_rules.setText(t("security.btn_rules"))
        self._btn_goals.setText(t("security.btn_goals"))
        self._btn_exportignore.setText(t("security.btn_exportignore"))
        self._btn_profiles.setText(t("security.btn_profiles"))
        self._btn_export_settings.setText(t("security.btn_export_settings"))
        self._btn_import_settings.setText(t("security.btn_import_settings"))
        self._btn_reset_settings.setText(t("security.btn_reset_settings"))
        self._btn_history.setText(t("security.btn_history"))
        self._sync_safe_hint()

    def _sync_safe_hint(self) -> None:
        mode = self.safe_mode_combo.currentText().strip()
        self.safe_hint.setText(t(f"safe_hint.{mode}") if mode else "")

    def load_from_config(self, config: Config) -> None:
        self.redact_checkbox.setChecked(config.redact_secrets)
        self.git_patch_checkbox.setChecked(config.include_git_patch)
        self.include_project_checkbox.setChecked(config.include_project_in_zip)
        self.keep_staging_checkbox.setChecked(config.keep_staging_folder)
        self.extra_ignored_edit.setText(", ".join(config.extra_ignored_dirs))
        set_combo_value(self.safe_mode_combo, config.normalized_safe_export_mode())
