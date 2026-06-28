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
    QTextBrowser,
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
        self.setWindowTitle("Подтверждение плана экспорта")
        self.setMinimumSize(760, 560)

        layout = QVBoxLayout(self)
        title = QLabel("Проверьте план экспорта перед копированием")
        title.setObjectName("PageTitle")
        hint = QLabel(
            "Проект не будет скопирован до подтверждения плана. "
            "Правила безопасного экспорта остаются активными, даже если заданы пользовательские правила включения."
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
        buttons.button(QDialogButtonBox.StandardButton.Ok).setText("Начать экспорт")
        buttons.button(QDialogButtonBox.StandardButton.Cancel).setText("Отмена")
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
        self.setWindowTitle("Правила включения и исключения")
        self.setMinimumSize(780, 640)

        self.excluded_files = QTextEdit()
        self.excluded_extensions = QTextEdit()
        self.always_include_files = QTextEdit()
        self.always_include_dirs = QTextEdit()

        fields = [
            ("Исключить файлы / glob-шаблоны", self.excluded_files, excluded_files),
            ("Исключить расширения", self.excluded_extensions, excluded_extensions),
            (
                "Всегда включать файлы / glob-шаблоны",
                self.always_include_files,
                always_include_files,
            ),
            ("Всегда включать директории", self.always_include_dirs, always_include_dirs),
        ]

        layout = QVBoxLayout(self)
        intro = QLabel(
            "Вводите по одному элементу на строку или через запятую. Правила безопасного экспорта могут по-прежнему блокировать рисковые файлы."
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
        self.setWindowTitle("Цели промптов")
        self.setMinimumSize(660, 420)
        selected = set(selected_goals)
        self.checkboxes: dict[str, QCheckBox] = {}

        layout = QVBoxLayout(self)
        hint = QLabel("Выберите цели, которые будут включены в AI_PROMPTS/CUSTOM_PROMPT.md.")
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
        self.setWindowTitle("История экспортов")
        self.setMinimumSize(720, 460)
        layout = QVBoxLayout(self)
        title = QLabel("История экспортов")
        title.setObjectName("PageTitle")
        layout.addWidget(title)
        text = QPlainTextEdit()
        text.setReadOnly(True)
        if not history:
            text.setPlainText("История экспортов пуста.")
        else:
            blocks: list[str] = []
            for item in history[:15]:
                blocks.append(
                    "\n".join(
                        [
                            str(item.get("generated_at", "")),
                            f"Проект: {item.get('project_name', '')}",
                            f"Профиль: {item.get('profile', '')}; Режим: {item.get('safe_export_mode', '')}; Разбивка: {item.get('split_archives', False)}",
                            f"Результат: {item.get('result', '')}",
                        ]
                    )
                )
            text.setPlainText("\n\n".join(blocks))
        layout.addWidget(text, 1)
        close = QPushButton("Закрыть")
        close.clicked.connect(self.accept)
        row = QHBoxLayout()
        row.addStretch(1)
        row.addWidget(close)
        layout.addLayout(row)


_HELP_HTML = """
<html><body style="font-family:sans-serif;font-size:10pt;line-height:1.6;">
<h2>Project Exporter Desktop — Руководство пользователя</h2>

<h3>Назначение приложения</h3>
<p>Project Exporter Desktop создаёт <b>снимок вашего проекта</b> в удобном для ИИ-ассистентов формате:
ZIP-архив с кодом, текстовый дамп всех файлов, отчёты по структуре, Git-истории и аналитике.
Готовый пакет загружается в Claude Code, ChatGPT, Gemini или другой ИИ.</p>

<h3>1. Страница «Проект»</h3>
<ul>
<li><b>Папка проекта</b> — укажите корень вашего проекта (кнопка «Обзор» или вставьте путь вручную).</li>
<li><b>Определённый стек</b> — приложение автоматически определяет технологии и исключает тяжёлые папки
(node_modules, .venv, target и т.д.).</li>
<li><b>Задача / контекст разработчика</b> — опциональный текст, который будет вставлен в самое начало
текстового дампа. Используйте это поле, чтобы указать ИИ, что нужно сделать с кодом.</li>
</ul>

<h3>2. Страница «Настройки»</h3>
<ul>
<li><b>Пресеты</b> — быстрые конфигурации для популярных ИИ-ассистентов (Claude Code, ChatGPT, Code Review,
Security Audit, Онбординг). Выберите пресет — настройки подберутся автоматически.</li>
<li><b>Профиль экспорта</b> — определяет набор включаемых файлов (ai_review, quick, minimal, security, full).</li>
<li><b>Режим экспорта</b> — <em>safe</em> исключает секреты и бинарники; <em>permissive</em> снимает большинство ограничений.</li>
<li><b>Режим diff</b> — экспортировать все файлы или только изменённые (относительно ветки / коммита).</li>
<li><b>Тема</b> — системная, светлая или тёмная.</li>
<li><b>Режим наблюдения</b> — автоматически отслеживает изменения в папке проекта и уведомляет через трей.</li>
<li><b>Лимит текстового файла</b> — ограничение на размер одного файла в текстовом дампе (МБ).</li>
</ul>

<h3>3. Страница «Безопасность»</h3>
<ul>
<li><b>Редактировать секреты</b> — маскирует API-ключи, пароли и токены в текстовых файлах.</li>
<li><b>Включить Git-патч</b> — добавляет git diff в экспорт-пакет.</li>
<li><b>Дополнительные игнорируемые папки</b> — укажите папки через запятую.</li>
<li><b>Правила включения/исключения</b> — детальные glob-правила для отдельных файлов и расширений.</li>
<li><b>.exportignore</b> — файл в корне проекта (аналог .gitignore) для тонкой настройки исключений.</li>
</ul>

<h3>4. Страница «Предпросмотр»</h3>
<p>Перед экспортом отображается дерево файлов с цветовой кодировкой:</p>
<ul>
<li><span style="color:#4caf50;">■ Зелёный</span> — файл будет включён</li>
<li><span style="color:#f44336;">■ Красный</span> — исключён по соображениям безопасности</li>
<li><span style="color:#ff9800;">■ Оранжевый</span> — исключён (средний приоритет)</li>
<li><span style="color:#9e9e9e;">■ Серый</span> — исключён (информация)</li>
<li><span style="color:#00bcd4;">■ Голубой</span> — принудительно включён вами</li>
<li><span style="color:#e91e63;">■ Розовый</span> — принудительно исключён вами</span></li>
</ul>
<p><b>Двойной клик</b> по файлу переключает его включение/исключение. Кнопка «Скопировать дамп»
копирует текст всех включённых файлов в буфер обмена (без создания архива).</p>

<h3>5. Страница «Журнал»</h3>
<p>Отображает прогресс экспорта в реальном времени. 8 шагов: план → копирование → структура →
Git-отчёт → текстовый дамп → аналитика → манифест → архивирование.</p>

<h3>6. Страница «Результат»</h3>
<p>Показывает итоговый ZIP-файл. Кнопка «Открыть результат» открывает папку с архивом в Проводнике.</p>

<h3>7. Страница «История»</h3>
<p>Список всех выполненных экспортов с метаданными: дата, профиль, количество токенов, путь к результату.</p>

<h3>8. Страница «Аналитика»</h3>
<p>Статистика по проекту: количество файлов, языки, размеры, топ-файлы по объёму.</p>

<h3>Масштабирование интерфейса</h3>
<p>Меню <b>Вид → Увеличить / Уменьшить / Сбросить масштаб</b> (или Ctrl++/Ctrl+−/Ctrl+0)
изменяет размер шрифта в интерфейсе от 70% до 150%.</p>

<h3>Системный трей</h3>
<p>При закрытии окна приложение сворачивается в трей. Двойной клик по иконке возвращает окно.
Правая кнопка мыши на иконке открывает меню с опцией <em>Быстрый экспорт</em> и <em>Выход</em>.</p>

<h3>Инструменты (меню)</h3>
<ul>
<li><b>Правила включения/исключения</b> — диалог glob-правил.</li>
<li><b>Промпт-цели</b> — выбор разделов CUSTOM_PROMPT.md.</li>
<li><b>Создать .exportignore</b> — создаёт шаблон файла исключений в корне проекта.</li>
<li><b>Экспорт / Импорт настроек</b> — сохранение и загрузка конфигурации в JSON.</li>
<li><b>Сбросить настройки</b> — возврат к заводским настройкам.</li>
</ul>

<h3>Горячие клавиши</h3>
<table border="0" cellpadding="4">
<tr><td><b>Ctrl++</b></td><td>Увеличить масштаб</td></tr>
<tr><td><b>Ctrl+−</b></td><td>Уменьшить масштаб</td></tr>
<tr><td><b>Ctrl+0</b></td><td>Сбросить масштаб (100%)</td></tr>
<tr><td><b>F1</b></td><td>Открыть справку</td></tr>
</table>

<h3>Советы</h3>
<ul>
<li>Используйте поле <em>Задача / контекст</em>, чтобы ИИ сразу понял, что вы хотите.</li>
<li>Пресет <em>Claude Code</em> — оптимален для работы с Anthropic Claude; снимает лимит размера файла.</li>
<li>Пресет <em>ChatGPT</em> — ограничивает файлы до 1 МБ, чтобы уложиться в 128K контекст.</li>
<li>Режим <em>Быстрый экспорт</em> из трея использует текущие настройки без открытия окна.</li>
<li>Лог-файл приложения находится в <code>%LOCALAPPDATA%/project_exporter_desktop/</code>.</li>
</ul>
</body></html>
"""


class HelpDialog(QDialog):
    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setWindowTitle("Справка — Project Exporter Desktop")
        self.setMinimumSize(780, 620)
        self.resize(900, 700)

        layout = QVBoxLayout(self)

        browser = QTextBrowser()
        browser.setOpenExternalLinks(False)
        browser.setHtml(_HELP_HTML)
        layout.addWidget(browser, 1)

        close = QPushButton("Закрыть")
        close.clicked.connect(self.accept)
        row = QHBoxLayout()
        row.addStretch(1)
        row.addWidget(close)
        layout.addLayout(row)