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
from ...constants import DIFF_EXPORT_MODES, EXPORT_PROFILES, MAX_ARCHIVE_PART_MB
from . import make_card, make_scroll_page, set_combo_value, wrap_layout


class SettingsPage(QWidget):
    """Страница 2 — профиль экспорта, текстовый дамп, архив и параметры Git diff."""

    def __init__(self, profile_catalog: dict[str, str], parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._profile_catalog = profile_catalog

        scroll, layout = make_scroll_page(
            "Настройки экспорта",
            "Управление профилем, лимитами текстового дампа, размером архива и выбором файлов Git/инкрементального экспорта.",
        )
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

        self.diff_combo = QComboBox()
        self.diff_combo.addItems(list(DIFF_EXPORT_MODES.keys()))
        self.diff_combo.currentTextChanged.connect(self._sync_diff_hint)
        self.diff_hint = QLabel("")
        self.diff_hint.setObjectName("PageHint")
        diff_block = QVBoxLayout()
        diff_block.addWidget(self.diff_combo)
        diff_block.addWidget(self.diff_hint)
        form.addRow("Режим экспорта Git", wrap_layout(diff_block))

        refs_row = QHBoxLayout()
        self.diff_base_edit = QLineEdit()
        self.diff_base_edit.setPlaceholderText("HEAD")
        self.diff_target_edit = QLineEdit()
        self.diff_target_edit.setPlaceholderText("целевая ссылка")
        refs_row.addWidget(QLabel("База"))
        refs_row.addWidget(self.diff_base_edit)
        refs_row.addWidget(QLabel("Цель"))
        refs_row.addWidget(self.diff_target_edit)
        form.addRow("Git-ссылки", wrap_layout(refs_row))

        self.incremental_checkbox = QCheckBox(
            "Экспортировать только файлы, добавленные или изменённые с момента последнего успешного базового снимка"
        )
        form.addRow("Инкрементальный", self.incremental_checkbox)

        card_layout.addLayout(form)
        layout.addWidget(card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def _sync_profile_hint(self) -> None:
        profile = self.profile_combo.currentText().strip()
        self.profile_hint.setText(
            self._profile_catalog.get(profile, EXPORT_PROFILES.get(profile, ""))
        )

    def _sync_diff_hint(self) -> None:
        mode = self.diff_combo.currentText().strip()
        self.diff_hint.setText(DIFF_EXPORT_MODES.get(mode, ""))
        refs_enabled = mode in {"changed_since_ref", "between_refs"}
        self.diff_base_edit.setEnabled(refs_enabled)
        self.diff_target_edit.setEnabled(mode == "between_refs")

    def _sync_text_limit_state(self) -> None:
        self.max_text_mb_spin.setEnabled(self.text_limit_checkbox.isChecked())

    def load_from_config(self, config: Config) -> None:
        self.text_limit_checkbox.setChecked(config.text_file_size_limit_enabled)
        self.max_text_mb_spin.setValue(max(1, int(config.max_text_file_mb)))
        self.zip_limit_spin.setValue(max(1, int(config.zip_part_limit_mb or MAX_ARCHIVE_PART_MB)))
        self.diff_base_edit.setText(config.diff_base_ref)
        self.diff_target_edit.setText(config.diff_target_ref)
        self.incremental_checkbox.setChecked(config.incremental_export_enabled)
        set_combo_value(self.profile_combo, config.normalized_export_profile())
        set_combo_value(self.diff_combo, config.normalized_diff_export_mode())
        self._sync_text_limit_state()
