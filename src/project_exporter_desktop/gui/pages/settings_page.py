from __future__ import annotations

from PySide6.QtCore import Qt
from PySide6.QtWidgets import (
    QCheckBox,
    QComboBox,
    QFormLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QSpinBox,
    QVBoxLayout,
    QWidget,
)

from ...config import Config
from ...constants import AI_PRESETS, DIFF_EXPORT_MODES, EXPORT_PROFILES, MAX_ARCHIVE_PART_MB
from ...i18n import t
from . import make_card, make_scroll_page, set_combo_value, wrap_layout


class SettingsPage(QWidget):

    def __init__(self, profile_catalog: dict[str, str], parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._profile_catalog = profile_catalog
        self._applying_preset = False

        scroll, layout, self._page_title, self._page_hint = make_scroll_page(
            t("settings.page_title"),
            t("settings.page_hint"),
        )

        preset_card, preset_layout = make_card()
        self._preset_card_title = QLabel(t("settings.preset_section"))
        self._preset_card_title.setObjectName("PageTitle")
        preset_layout.addWidget(self._preset_card_title)

        preset_form = QFormLayout()
        self.preset_combo = QComboBox()
        self.preset_combo.addItem(t("settings.no_preset"))
        self.preset_combo.addItems(list(AI_PRESETS.keys()))
        self.preset_combo.currentTextChanged.connect(self._on_preset_changed)
        self.preset_hint = QLabel("")
        self.preset_hint.setObjectName("PageHint")
        self.preset_hint.setWordWrap(True)
        preset_block = QVBoxLayout()
        preset_block.addWidget(self.preset_combo)
        preset_block.addWidget(self.preset_hint)
        self._lbl_preset = QLabel(t("settings.lbl_preset"))
        preset_form.addRow(self._lbl_preset, wrap_layout(preset_block))
        preset_layout.addLayout(preset_form)
        layout.addWidget(preset_card)

        card, card_layout = make_card()
        form = QFormLayout()
        form.setLabelAlignment(Qt.AlignmentFlag.AlignLeft)

        self.profile_combo = QComboBox()
        self.profile_combo.addItems(list(profile_catalog.keys()))
        self.profile_combo.currentTextChanged.connect(self._sync_profile_hint)
        self.profile_hint = QLabel("")
        self.profile_hint.setObjectName("PageHint")
        profile_block = QVBoxLayout()
        profile_block.addWidget(self.profile_combo)
        profile_block.addWidget(self.profile_hint)
        self._lbl_profile = QLabel(t("settings.lbl_profile"))
        form.addRow(self._lbl_profile, wrap_layout(profile_block))

        self.text_limit_checkbox = QCheckBox(t("settings.text_limit"))
        self.text_limit_checkbox.toggled.connect(self._sync_text_limit_state)
        self.max_text_mb_spin = QSpinBox()
        self.max_text_mb_spin.setRange(1, 4096)
        self.max_text_mb_spin.setSuffix(t("settings.mb_suffix"))
        text_limit_row = QHBoxLayout()
        text_limit_row.addWidget(self.text_limit_checkbox)
        text_limit_row.addWidget(self.max_text_mb_spin)
        text_limit_row.addStretch(1)
        self._lbl_text_dump = QLabel(t("settings.lbl_text_dump"))
        form.addRow(self._lbl_text_dump, wrap_layout(text_limit_row))

        self.zip_limit_spin = QSpinBox()
        self.zip_limit_spin.setRange(1, 102400)
        self.zip_limit_spin.setSuffix(t("settings.mb_suffix"))
        self._lbl_zip = QLabel(t("settings.lbl_zip_limit"))
        form.addRow(self._lbl_zip, self.zip_limit_spin)

        self.theme_combo = QComboBox()
        self.theme_combo.addItems(["system", "light", "dark"])
        self._lbl_theme = QLabel(t("settings.lbl_theme"))
        form.addRow(self._lbl_theme, self.theme_combo)

        watch_row = QVBoxLayout()
        self.watch_checkbox = QCheckBox(t("settings.watch"))
        self.watch_clipboard_checkbox = QCheckBox(t("settings.watch_clipboard"))
        watch_row.addWidget(self.watch_checkbox)
        watch_row.addWidget(self.watch_clipboard_checkbox)
        self._lbl_watch = QLabel(t("settings.lbl_watch"))
        form.addRow(self._lbl_watch, wrap_layout(watch_row))

        self.diff_combo = QComboBox()
        self.diff_combo.addItems(list(DIFF_EXPORT_MODES.keys()))
        self.diff_combo.currentTextChanged.connect(self._sync_diff_hint)
        self.diff_hint = QLabel("")
        self.diff_hint.setObjectName("PageHint")
        diff_block = QVBoxLayout()
        diff_block.addWidget(self.diff_combo)
        diff_block.addWidget(self.diff_hint)
        self._lbl_diff = QLabel(t("settings.lbl_diff"))
        form.addRow(self._lbl_diff, wrap_layout(diff_block))

        refs_row = QHBoxLayout()
        self.diff_base_edit = QLineEdit()
        self.diff_base_edit.setPlaceholderText("HEAD")
        self.diff_target_edit = QLineEdit()
        self.diff_target_edit.setPlaceholderText(t("settings.diff_target_placeholder"))
        self._diff_base_lbl = QLabel(t("settings.diff_base"))
        refs_row.addWidget(self._diff_base_lbl)
        refs_row.addWidget(self.diff_base_edit)
        self.diff_target_edit.setVisible(False)
        self._lbl_git_ref = QLabel(t("settings.lbl_git_ref"))
        form.addRow(self._lbl_git_ref, wrap_layout(refs_row))

        self.incremental_checkbox = QCheckBox(t("settings.incremental"))
        self.incremental_checkbox.setVisible(False)
        self._lbl_incremental = QLabel(t("settings.lbl_incremental"))
        form.addRow(self._lbl_incremental, self.incremental_checkbox)

        card_layout.addLayout(form)
        layout.addWidget(card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def retranslate(self) -> None:
        self._page_title.setText(t("settings.page_title"))
        self._page_hint.setText(t("settings.page_hint"))
        self._preset_card_title.setText(t("settings.preset_section"))
        self._lbl_preset.setText(t("settings.lbl_preset"))
        self._lbl_profile.setText(t("settings.lbl_profile"))
        self._lbl_text_dump.setText(t("settings.lbl_text_dump"))
        self._lbl_zip.setText(t("settings.lbl_zip_limit"))
        self._lbl_theme.setText(t("settings.lbl_theme"))
        self._lbl_watch.setText(t("settings.lbl_watch"))
        self._lbl_diff.setText(t("settings.lbl_diff"))
        self._lbl_git_ref.setText(t("settings.lbl_git_ref"))
        self._lbl_incremental.setText(t("settings.lbl_incremental"))
        self._diff_base_lbl.setText(t("settings.diff_base"))
        self.text_limit_checkbox.setText(t("settings.text_limit"))
        self.watch_checkbox.setText(t("settings.watch"))
        self.watch_clipboard_checkbox.setText(t("settings.watch_clipboard"))
        self.incremental_checkbox.setText(t("settings.incremental"))
        self.diff_target_edit.setPlaceholderText(t("settings.diff_target_placeholder"))
        suffix = t("settings.mb_suffix")
        self.max_text_mb_spin.setSuffix(suffix)
        self.zip_limit_spin.setSuffix(suffix)
        # Update no-preset item text (always at index 0)
        self.preset_combo.setItemText(0, t("settings.no_preset"))
        self._sync_diff_hint()
        self._sync_profile_hint()
        if self.preset_combo.currentIndex() != 0:
            preset_name = self.preset_combo.currentText()
            self.preset_hint.setText(t(f"preset.{preset_name}.desc"))

    def _on_preset_changed(self, preset_name: str) -> None:
        if self.preset_combo.currentIndex() == 0 or self._applying_preset:
            self.preset_hint.setText("")
            return
        preset = AI_PRESETS.get(preset_name)
        if not preset:
            return
        self.preset_hint.setText(t(f"preset.{preset_name}.desc"))
        self._apply_preset(preset)

    def _apply_preset(self, preset: dict[str, object]) -> None:
        self._applying_preset = True
        try:
            if "export_profile" in preset:
                set_combo_value(self.profile_combo, str(preset["export_profile"]))
            if "diff_export_mode" in preset:
                set_combo_value(self.diff_combo, str(preset["diff_export_mode"]))
            if "text_file_size_limit_enabled" in preset:
                self.text_limit_checkbox.setChecked(bool(preset["text_file_size_limit_enabled"]))
            if "max_text_file_mb" in preset:
                self.max_text_mb_spin.setValue(int(preset["max_text_file_mb"]))
        finally:
            self._applying_preset = False
        self._sync_text_limit_state()

    def _sync_profile_hint(self) -> None:
        if self._applying_preset:
            return
        profile = self.profile_combo.currentText().strip()
        self.profile_hint.setText(
            self._profile_catalog.get(profile, EXPORT_PROFILES.get(profile, ""))
        )

    def _sync_diff_hint(self) -> None:
        mode = self.diff_combo.currentText().strip()
        self.diff_hint.setText(t(f"diff_hint.{mode}") if mode else "")
        refs_enabled = mode == "git_ref"
        self.diff_base_edit.setEnabled(refs_enabled)
        self.diff_target_edit.setEnabled(False)

    def _sync_text_limit_state(self) -> None:
        self.max_text_mb_spin.setEnabled(self.text_limit_checkbox.isChecked())

    def load_from_config(self, config: Config) -> None:
        self.preset_combo.setCurrentIndex(0)
        self.text_limit_checkbox.setChecked(config.text_file_size_limit_enabled)
        self.max_text_mb_spin.setValue(max(1, int(config.max_text_file_mb)))
        self.zip_limit_spin.setValue(max(1, int(config.zip_part_limit_mb or MAX_ARCHIVE_PART_MB)))
        set_combo_value(self.theme_combo, config.normalized_theme())
        self.watch_checkbox.setChecked(config.watch_enabled)
        self.watch_clipboard_checkbox.setChecked(config.watch_clipboard_auto_update)
        self.diff_base_edit.setText(config.diff_base_ref)
        self.diff_target_edit.setText(config.diff_target_ref)
        self.incremental_checkbox.setChecked(False)
        set_combo_value(self.profile_combo, config.normalized_export_profile())
        set_combo_value(self.diff_combo, config.normalized_diff_export_mode())
        self._sync_text_limit_state()
        self._sync_profile_hint()
        self._sync_diff_hint()

    def get_preset_name(self) -> str:
        if self.preset_combo.currentIndex() == 0:
            return ""
        return self.preset_combo.currentText()
