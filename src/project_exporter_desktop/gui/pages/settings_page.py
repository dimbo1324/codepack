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
from . import make_card, make_scroll_page, set_combo_value, wrap_layout

_NO_PRESET = "— без пресета —"


class SettingsPage(QWidget):

    def __init__(self, profile_catalog: dict[str, str], parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._profile_catalog = profile_catalog
        self._applying_preset = False

        scroll, layout = make_scroll_page(
            "Настройки экспорта",
            "Выберите AI-пресет для быстрой конфигурации, либо настройте профиль и параметры вручную.",
        )

        preset_card, preset_layout = make_card()
        preset_title = QLabel("AI-пресет")
        preset_title.setObjectName("PageTitle")
        preset_layout.addWidget(preset_title)

        preset_form = QFormLayout()
        self.preset_combo = QComboBox()
        self.preset_combo.addItem(_NO_PRESET)
        self.preset_combo.addItems(list(AI_PRESETS.keys()))
        self.preset_combo.currentTextChanged.connect(self._on_preset_changed)
        self.preset_hint = QLabel("")
        self.preset_hint.setObjectName("PageHint")
        self.preset_hint.setWordWrap(True)
        preset_block = QVBoxLayout()
        preset_block.addWidget(self.preset_combo)
        preset_block.addWidget(self.preset_hint)
        preset_form.addRow("Пресет", wrap_layout(preset_block))
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
        form.addRow("Профиль экспорта", wrap_layout(profile_block))

        self.text_limit_checkbox = QCheckBox("Ограничить размер файла в текстовом дампе")
        self.text_limit_checkbox.toggled.connect(self._sync_text_limit_state)
        self.max_text_mb_spin = QSpinBox()
        self.max_text_mb_spin.setRange(1, 4096)
        self.max_text_mb_spin.setSuffix(" МБ")
        text_limit_row = QHBoxLayout()
        text_limit_row.addWidget(self.text_limit_checkbox)
        text_limit_row.addWidget(self.max_text_mb_spin)
        text_limit_row.addStretch(1)
        form.addRow("Текстовый дамп", wrap_layout(text_limit_row))

        self.zip_limit_spin = QSpinBox()
        self.zip_limit_spin.setRange(1, 102400)
        self.zip_limit_spin.setSuffix(" МБ")
        form.addRow("Лимит части ZIP", self.zip_limit_spin)

        self.theme_combo = QComboBox()
        self.theme_combo.addItems(["system", "light", "dark"])
        form.addRow("Тема", self.theme_combo)

        watch_row = QVBoxLayout()
        self.watch_checkbox = QCheckBox("Следить за изменениями проекта")
        self.watch_clipboard_checkbox = QCheckBox("Автоматически обновлять clipboard-дамп")
        watch_row.addWidget(self.watch_checkbox)
        watch_row.addWidget(self.watch_clipboard_checkbox)
        form.addRow("Watch-режим", wrap_layout(watch_row))

        self.diff_combo = QComboBox()
        self.diff_combo.addItems(list(DIFF_EXPORT_MODES.keys()))
        self.diff_combo.currentTextChanged.connect(self._sync_diff_hint)
        self.diff_hint = QLabel("")
        self.diff_hint.setObjectName("PageHint")
        diff_block = QVBoxLayout()
        diff_block.addWidget(self.diff_combo)
        diff_block.addWidget(self.diff_hint)
        form.addRow("Режим экспорта", wrap_layout(diff_block))

        refs_row = QHBoxLayout()
        self.diff_base_edit = QLineEdit()
        self.diff_base_edit.setPlaceholderText("HEAD")
        self.diff_target_edit = QLineEdit()
        self.diff_target_edit.setPlaceholderText("целевая ссылка")
        refs_row.addWidget(QLabel("База"))
        refs_row.addWidget(self.diff_base_edit)
        self.diff_target_edit.setVisible(False)
        form.addRow("Git-ссылка", wrap_layout(refs_row))

        self.incremental_checkbox = QCheckBox(
            "Экспортировать только файлы, добавленные или изменённые с момента последнего успешного базового снимка"
        )
        self.incremental_checkbox.setVisible(False)
        form.addRow("Инкрементальный", self.incremental_checkbox)

        card_layout.addLayout(form)
        layout.addWidget(card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)


    def _on_preset_changed(self, preset_name: str) -> None:
        if preset_name == _NO_PRESET or self._applying_preset:
            self.preset_hint.setText("")
            return
        preset = AI_PRESETS.get(preset_name)
        if not preset:
            return
        self.preset_hint.setText(str(preset.get("description", "")))
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
        self.diff_hint.setText(DIFF_EXPORT_MODES.get(mode, ""))
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
        text = self.preset_combo.currentText()
        return "" if text == _NO_PRESET else text