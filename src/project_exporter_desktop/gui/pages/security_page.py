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
    """Страница 3 — режим безопасного экспорта, редактирование и пользовательские правила."""

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
            "Безопасность и фильтрация",
            "Настройка режима безопасного экспорта, скрытия секретов, Git-патча и пользовательских правил включения/исключения.",
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
        form.addRow("Режим безопасности", wrap_layout(safe_block))

        self.redact_checkbox = QCheckBox("Скрывать очевидные секреты в текстовых и Git-отчётах")
        self.git_patch_checkbox = QCheckBox(
            "Включить полный Git-патч (отключён по умолчанию, так как патчи могут содержать секреты)"
        )
        self.include_project_checkbox = QCheckBox("Включить копию проекта в финальный ZIP")
        self.keep_staging_checkbox = QCheckBox("Сохранять рабочую папку после экспорта")
        form.addRow("Скрытие секретов", self.redact_checkbox)
        form.addRow("Git-патч", self.git_patch_checkbox)
        form.addRow("Файлы проекта", self.include_project_checkbox)
        form.addRow("Рабочая папка", self.keep_staging_checkbox)

        self.extra_ignored_edit = QLineEdit()
        self.extra_ignored_edit.setPlaceholderText(".cache, tmp, vendor")
        form.addRow("Дополнительно игнорировать", self.extra_ignored_edit)

        card_layout.addLayout(form)

        actions = QHBoxLayout()
        for text, signal in [
            ("Правила включения/исключения", self.edit_rules_requested),
            ("Промпт-цели", self.edit_prompt_goals_requested),
            ("Создать .exportignore", self.create_exportignore_requested),
            ("Профили JSON", self.open_profiles_json_requested),
        ]:
            button = QPushButton(text)
            button.clicked.connect(signal.emit)
            actions.addWidget(button)
        actions.addStretch(1)
        card_layout.addLayout(actions)

        settings_actions = QHBoxLayout()
        for text, signal in [
            ("Экспорт настроек", self.export_settings_requested),
            ("Импорт настроек", self.import_settings_requested),
            ("Сбросить настройки", self.reset_settings_requested),
            ("История экспортов", self.show_history_requested),
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
