# Core application module for codepack.

from __future__ import annotations

from PySide6.QtCore import QObject, Signal

# ---------------------------------------------------------------------------
# String tables
# ---------------------------------------------------------------------------

_RU: dict[str, str] = {
    # Sidebar
    "nav.1": "1  Проект",
    "nav.2": "2  Настройки",
    "nav.3": "3  Безопасность",
    "nav.4": "4  Предпросмотр",
    "nav.5": "5  Журнал",
    "nav.6": "6  Результат",
    "nav.7": "7  История",
    "nav.8": "8  Аналитика",
    "sidebar.subtitle": "Снимок проекта для ИИ",
    "sidebar.desktop": "Рабочий стол",
    # Menus
    "menu.file": "Файл",
    "menu.file.desktop": "Рабочий стол",
    "menu.file.last_result": "Последний результат",
    "menu.file.exit": "Выход",
    "menu.tools": "Инструменты",
    "menu.tools.rules": "Правила включения/исключения",
    "menu.tools.prompt_goals": "Промпт-цели",
    "menu.tools.create_exportignore": "Создать .exportignore",
    "menu.tools.export_settings": "Экспорт настроек",
    "menu.tools.import_settings": "Импорт настроек",
    "menu.tools.reset_settings": "Сбросить настройки",
    "menu.tools.history": "История экспортов",
    "menu.view": "Вид",
    "menu.view.zoom_in": "Увеличить масштаб",
    "menu.view.zoom_out": "Уменьшить масштаб",
    "menu.view.zoom_reset": "Сбросить масштаб (100%)",
    "menu.view.language": "Переключить на английский",
    "menu.help": "Справка",
    "menu.help.manual": "Руководство пользователя",
    "menu.help.about": "О программе",
    # Bottom bar
    "status.ready": "Готово",
    "status.building": "Строится план экспорта...",
    "status.exporting": "Выполняется экспорт...",
    "status.clipboard": "Подготовка дампа для буфера обмена...",
    "btn.open_result": "Открыть результат",
    "btn.cancel": "Отмена",
    "btn.clipboard": "Скопировать дамп",
    "btn.clipboard.tip": "Скопировать текст всех включённых файлов в буфер обмена (без создания архива)",
    "btn.codex": "Codex-пакет",
    "btn.create_export": "Создать экспорт",
    # ProjectPage
    "project.page_title": "Выбор проекта",
    "project.page_hint": "Выберите корневую папку проекта. При экспорте исходный проект не изменяется.",
    "project.folder_label": "Папка проекта",
    "project.browse": "Обзор",
    "project.default_hint": (
        "По умолчанию исключаются .git, node_modules, виртуальные окружения,"
        " кэш, артефакты сборки и очевидные секреты."
    ),
    "project.ctx_title": "Контекст для ИИ",
    "project.ctx_hint": (
        "Опишите задачу, проблему или вопрос, с которым поможет ИИ. "
        "Этот текст появится первым в текстовом дампе проекта — "
        "прямо перед исходным кодом."
    ),
    "project.ctx_placeholder": (
        "Пример: «Помоги разобраться в архитектуре этого проекта. "
        "Меня интересует, как работает авторизация и где хранятся пользователи.»"
    ),
    "project.browse_dialog": "Выберите папку проекта",
    # SettingsPage
    "settings.page_title": "Настройки экспорта",
    "settings.page_hint": "Выберите AI-пресет для быстрой конфигурации, либо настройте профиль и параметры вручную.",
    "settings.preset_section": "AI-пресет",
    "settings.no_preset": "— без пресета —",
    "settings.lbl_preset": "Пресет",
    "settings.lbl_profile": "Профиль экспорта",
    "settings.lbl_text_dump": "Текстовый дамп",
    "settings.text_limit": "Ограничить размер файла в текстовом дампе",
    "settings.lbl_zip_limit": "Лимит части ZIP",
    "settings.lbl_theme": "Тема",
    "settings.lbl_watch": "Watch-режим",
    "settings.watch": "Следить за изменениями проекта",
    "settings.watch_clipboard": "Автоматически обновлять clipboard-дамп",
    "settings.lbl_diff": "Режим экспорта",
    "settings.lbl_git_ref": "Git-ссылка",
    "settings.diff_base": "База",
    "settings.diff_target_placeholder": "целевая ссылка",
    "settings.lbl_incremental": "Инкрементальный",
    "settings.incremental": (
        "Экспортировать только файлы, добавленные или изменённые с момента последнего успешного базового снимка"
    ),
    "settings.mb_suffix": " МБ",
    # SecurityPage
    "security.page_title": "Безопасность и фильтрация",
    "security.page_hint": "Настройка режима безопасного экспорта, скрытия секретов, Git-патча и пользовательских правил включения/исключения.",
    "security.lbl_safe_mode": "Режим безопасности",
    "security.lbl_redact": "Скрытие секретов",
    "security.redact": "Скрывать очевидные секреты в текстовых и Git-отчётах",
    "security.lbl_git_patch": "Git-патч",
    "security.git_patch": "Включить полный Git-патч (отключён по умолчанию, так как патчи могут содержать секреты)",
    "security.lbl_project_files": "Файлы проекта",
    "security.include_project": "Включить копию проекта в финальный ZIP",
    "security.lbl_staging": "Рабочая папка",
    "security.keep_staging": "Сохранять рабочую папку после экспорта",
    "security.lbl_extra_ignore": "Дополнительно игнорировать",
    "security.btn_rules": "Правила включения/исключения",
    "security.btn_goals": "Промпт-цели",
    "security.btn_exportignore": "Создать .exportignore",
    "security.btn_profiles": "Профили JSON",
    "security.btn_export_settings": "Экспорт настроек",
    "security.btn_import_settings": "Импорт настроек",
    "security.btn_reset_settings": "Сбросить настройки",
    "security.btn_history": "История экспортов",
    # PreviewPage
    "preview.page_title": "Предпросмотр экспорта",
    "preview.page_hint": (
        "Просмотрите список файлов, которые войдут в экспорт. "
        "Двойной клик на строке — переключить решение вручную. "
        "Голубой = принудительно включён, розовый = принудительно исключён."
    ),
    "preview.waiting": "Ожидание плана экспорта...",
    "preview.building": "Строится план экспорта...",
    "preview.reset_btn": "Сбросить переопределения",
    "preview.legend_included": "■ Включён",
    "preview.legend_excluded": "■ Исключён",
    "preview.col_file": "Файл",
    "preview.col_size": "Размер",
    "preview.col_status": "Статус",
    "preview.col_reason": "Причина",
    "preview.back_btn": "Назад к настройкам",
    "preview.start_btn": "Начать экспорт  →",
    "preview.status_included": "Включён",
    "preview.status_excluded": "Исключён",
    "preview.status_included_ov": "Включён ✎",
    "preview.status_excluded_ov": "Исключён ✎",
    "preview.token_label": "Приблизительно токенов в экспорте: ~{n}",
    "preview.stats": "Включено: {inc} файл(ов)  │  ~{size}  │  Исключено: {exc}",
    "preview.override_note": "  │  Переопределений: {n}",
    "preview.tooltip": "{name}: {tok} / {limit} токенов ({pct}%)",
    # RunPage
    "run.page_title": "Журнал выполнения",
    "run.page_hint": "Экспорт выполняется в отдельном потоке. Прогресс, текущий этап и диагностические сообщения отображаются здесь.",
    "run.waiting": "Ожидание",
    "run.starting": "Начало экспорта...",
    # ResultPage
    "result.page_title": "Итог экспорта",
    "result.page_hint": "После завершения здесь отображается финальный статус и путь к созданному архиву.",
    "result.not_created": "Экспорт ещё не создавался.",
    "result.running": "Выполняется экспорт...",
    "result.success": "Экспорт завершён успешно.",
    "result.cancelled": "Экспорт отменён пользователем. Часть результата могла быть создана.",
    "result.failed": "Ошибка экспорта. Технические подробности записаны в {log}.",
    "result.path_label": "Путь к результату",
    "result.path_placeholder": "Здесь появится путь к архиву",
    "result.btn_open": "Открыть результат",
    "result.btn_desktop": "Рабочий стол",
    # HistoryPage
    "history.page_title": "История",
    "history.page_hint": "Поиск, сортировка, повторный запуск и сравнение сохранённых экспортов.",
    "history.search_placeholder": "Поиск по проекту, дате или пути",
    "history.btn_refresh": "Обновить",
    "history.btn_open": "Открыть результат",
    "history.btn_repeat": "Повторить экспорт",
    "history.btn_compare": "Сравнить два",
    "history.col_date": "Дата",
    "history.col_project": "Проект",
    "history.col_profile": "Профиль",
    "history.col_files": "Файлы",
    "history.col_tokens": "Токены",
    "history.col_status": "Статус",
    "history.col_result": "Результат",
    "history.status_cancelled": "отменён",
    "history.status_done": "готов",
    # SnapshotCompareDialog
    "snapshot.title": "Сравнение снапшотов",
    "snapshot.subtitle": "Сравнение двух экспортов",
    "snapshot.close": "Закрыть",
    "snapshot.no_diff": "Различий в сохранённых снапшотах не найдено.",
    "snapshot.summary": "Добавлено: {added}; изменено: {modified}; удалено: {deleted}; изменение LOC: {loc:+,}.",
    # AnalyticsPage
    "analytics.page_title": "Аналитика",
    "analytics.page_hint": "Локальная сводка по стеку, языкам, зависимостям, Git и рискам проекта.",
    "analytics.btn_refresh": "Обновить аналитику",
    "analytics.chart_title": "Языки и LOC",
    "analytics.deps_title": "Зависимости",
    "analytics.col_manager": "Менеджер",
    "analytics.col_package": "Пакет",
    "analytics.col_version": "Версия",
    "analytics.col_warning": "Предупреждение",
    "analytics.git_title": "Git-активность",
    "analytics.git_not_built": "Git-сводка ещё не построена.",
    "analytics.risks_title": "Риски",
    "analytics.no_commits": "Коммиты не найдены.",
    "analytics.no_risks": "Явные риски не найдены.",
    "analytics.no_lang_data": "Нет данных по языкам",
    "analytics.loading": "Сбор аналитики...",
    "analytics.stat_project": "Проект",
    "analytics.stat_stack": "Стек",
    "analytics.stat_files": "Файлов",
    "analytics.stat_loc": "LOC",
    "analytics.stat_size": "Размер",
    "analytics.stat_error": "Ошибка",
    "analytics.branch_label": "Ветка: {branch}. {status}",
    "analytics.no_data_branch": "нет данных",
    # Dialogs
    "dialog.rules.title": "Правила включения и исключения",
    "dialog.rules.intro": "Вводите по одному элементу на строку или через запятую. Правила безопасного экспорта могут по-прежнему блокировать рисковые файлы.",
    "dialog.rules.excluded_files": "Исключить файлы / glob-шаблоны",
    "dialog.rules.excluded_ext": "Исключить расширения",
    "dialog.rules.always_files": "Всегда включать файлы / glob-шаблоны",
    "dialog.rules.always_dirs": "Всегда включать директории",
    "dialog.goals.title": "Цели промптов",
    "dialog.goals.hint": "Выберите цели, которые будут включены в AI_PROMPTS/CUSTOM_PROMPT.md.",
    "dialog.history.title": "История экспортов",
    "dialog.history.empty": "История экспортов пуста.",
    "dialog.history.close": "Закрыть",
    "dialog.help.title": "Справка — Project Exporter Desktop",
    "dialog.about.title": "О программе {name}",
    "dialog.about.body": (
        "{name} v{version}\n\n"
        "Создаёт снимок вашего проекта в формате, удобном для ИИ-ассистентов:\n"
        "архив с кодом, текстовый дамп, отчёты по структуре и аналитике.\n\n"
        "Поддерживаемые ИИ: Claude Code, ChatGPT, Gemini, Copilot и другие."
    ),
    "dialog.plan.title": "Подтверждение плана экспорта",
    "dialog.plan.header": "Проверьте план экспорта перед копированием",
    "dialog.plan.hint": (
        "Проект не будет скопирован до подтверждения плана. "
        "Правила безопасного экспорта остаются активными, даже если заданы пользовательские правила включения."
    ),
    "dialog.plan.ok": "Начать экспорт",
    "dialog.plan.cancel": "Отмена",
    # MessageBoxes
    "msg.cancel_export.title": "Отмена экспорта",
    "msg.cancel_export.body": "Остановить текущий экспорт? Частичный результат может остаться.",
    "msg.export_done.title": "Экспорт завершён",
    "msg.export_done.body": "Экспорт проекта успешно создан.",
    "msg.export_failed.title": "Ошибка экспорта",
    "msg.export_failed.body": "Экспорт завершился с ошибкой. Технические подробности записаны в:\n{log}",
    "msg.stopped.title": "Остановлено",
    "msg.stopped.body": "Экспорт остановлен. Проверьте результат и журнал.",
    "msg.no_result.title": "Нет результата",
    "msg.no_result.body": "Результат экспорта ещё не создавался.",
    "msg.exportignore_exists.title": ".exportignore уже существует",
    "msg.exportignore_exists.body": ".exportignore уже существует. Перезаписать шаблоном?",
    "msg.reset.title": "Сброс настроек",
    "msg.reset.body": "Сбросить настройки к безопасным значениям по умолчанию?",
    "msg.clipboard_done.title": "Дамп скопирован",
    "msg.clipboard_done.body": (
        "Текстовый дамп скопирован в буфер обмена.\n\n"
        "Размер: {size}\nОценка токенов: {summary}\n\n"
        "Вставьте текст в чат с ИИ-ассистентом."
    ),
    "msg.clipboard_failed.title": "Ошибка копирования",
    "msg.clipboard_failed.body": "Не удалось подготовить дамп. Технические подробности записаны в:\n{log}",
    "msg.bad_path.title": "Неверный путь к проекту",
    "msg.open_error.title": "Ошибка открытия",
    "msg.open_error.body": "Не удалось открыть:\n{path}\n\n{exc}",
    "msg.preview_failed.title": "Ошибка плана экспорта",
    "msg.preview_failed.body": "Не удалось построить план экспорта. Технические подробности записаны в:\n{log}",
    "msg.export_settings_failed.title": "Ошибка экспорта настроек",
    "msg.import_settings_failed.title": "Ошибка импорта настроек",
    "msg.write_error.title": "Ошибка записи",
    "msg.write_error_exportignore": "Не удалось создать .exportignore:\n{exc}",
    "msg.internal_error.title": "Ошибка экспорта",
    "msg.internal_error.body": "Внутреннее состояние было утеряно перед запуском.",
    # Tray / watcher
    "tray.minimized": "Приложение свернуто в трей. Быстрый экспорт доступен из контекстного меню.",
    "tray.quick_running": "Экспорт уже выполняется.",
    "tray.quick_done": "Быстрый экспорт завершён.",
    "tray.quick_failed": "Быстрый экспорт завершился с ошибкой.",
    "tray.quick_stopped": "Быстрый экспорт остановлен.",
    "tray.changed": "Проект изменился: событий файловой системы {n}.",
    "tray.clipboard_updated": "Clipboard-дамп обновлён: {size}, {summary}.",
    "tray.clipboard_failed": "Не удалось обновить clipboard-дамп.",
    "tray.menu_quick": "Быстрый экспорт",
    "tray.menu_open": "Открыть",
    "tray.menu_exit": "Выход",
    # Export hints (DIFF_EXPORT_MODES values translated)
    "diff_hint.all": "Полный экспорт — экспортировать весь выбранный проект.",
    "diff_hint.last_export": "С последнего экспорта — сравнить текущие хэши с историей экспортов.",
    "diff_hint.git_ref": "С Git-ссылки — экспортировать изменения относительно ветки, тега или коммита.",
    "diff_hint.uncommitted": "Только uncommitted — изменения рабочего дерева и новые файлы.",
    # SAFE_EXPORT_MODES values translated
    "safe_hint.safe": "Строгий — исключает секреты, приватные ключи, локальные БД, дампы и архивы.",
    "safe_hint.balanced": "Сбалансированный — исключает высокорисковые ключи и учётные данные, сохраняет больше конфигурации.",
    "safe_hint.full": "Полный — копирует файлы проекта, кроме игнорируемых директорий; только для приватных/локальных экспортов.",
    # AI preset descriptions
    "preset.Claude Code.desc": "Полный контекст: архитектура, Git-история, все AI-отчёты. Оптимально для Claude.",
    "preset.ChatGPT.desc": "Компактный обзор без Git-патча. Подходит для GPT-4 с ограниченным контекстным окном.",
    "preset.Code Review.desc": "Полный снимок кода с Git-патчем. Идеально для детального ревью.",
    "preset.Security Audit.desc": "Акцент на безопасности: конфигурация, зависимости и анализ рисков.",
    "preset.Онбординг.desc": "Краткий обзор для быстрого введения нового разработчика в проект.",
}

_EN: dict[str, str] = {
    "nav.1": "1  Project",
    "nav.2": "2  Settings",
    "nav.3": "3  Security",
    "nav.4": "4  Preview",
    "nav.5": "5  Log",
    "nav.6": "6  Result",
    "nav.7": "7  History",
    "nav.8": "8  Analytics",
    "sidebar.subtitle": "AI project snapshot",
    "sidebar.desktop": "Desktop",
    "menu.file": "File",
    "menu.file.desktop": "Desktop",
    "menu.file.last_result": "Last Result",
    "menu.file.exit": "Exit",
    "menu.tools": "Tools",
    "menu.tools.rules": "Include/Exclude Rules",
    "menu.tools.prompt_goals": "Prompt Goals",
    "menu.tools.create_exportignore": "Create .exportignore",
    "menu.tools.export_settings": "Export Settings",
    "menu.tools.import_settings": "Import Settings",
    "menu.tools.reset_settings": "Reset Settings",
    "menu.tools.history": "Export History",
    "menu.view": "View",
    "menu.view.zoom_in": "Zoom In",
    "menu.view.zoom_out": "Zoom Out",
    "menu.view.zoom_reset": "Reset Zoom (100%)",
    "menu.view.language": "Switch to Russian",
    "menu.help": "Help",
    "menu.help.manual": "User Manual",
    "menu.help.about": "About",
    "status.ready": "Ready",
    "status.building": "Building export plan...",
    "status.exporting": "Exporting...",
    "status.clipboard": "Preparing clipboard dump...",
    "btn.open_result": "Open Result",
    "btn.cancel": "Cancel",
    "btn.clipboard": "Copy Dump",
    "btn.clipboard.tip": "Copy text of all included files to clipboard (without creating an archive)",
    "btn.codex": "Codex Package",
    "btn.create_export": "Create Export",
    "project.page_title": "Select Project",
    "project.page_hint": "Select the root folder of your project. The original project is not modified during export.",
    "project.folder_label": "Project Folder",
    "project.browse": "Browse",
    "project.default_hint": (
        "By default, .git, node_modules, virtual environments,"
        " cache, build artifacts, and obvious secrets are excluded."
    ),
    "project.ctx_title": "AI Context",
    "project.ctx_hint": (
        "Describe the task, problem, or question you'd like AI to help with. "
        "This text will appear first in the project text dump — "
        "right before the source code."
    ),
    "project.ctx_placeholder": (
        "Example: 'Help me understand the architecture of this project. "
        "I'm interested in how authorization works and where users are stored.'"
    ),
    "project.browse_dialog": "Select Project Folder",
    "settings.page_title": "Export Settings",
    "settings.page_hint": "Select an AI preset for quick configuration, or manually configure the profile and parameters.",
    "settings.preset_section": "AI Preset",
    "settings.no_preset": "— no preset —",
    "settings.lbl_preset": "Preset",
    "settings.lbl_profile": "Export Profile",
    "settings.lbl_text_dump": "Text Dump",
    "settings.text_limit": "Limit file size in text dump",
    "settings.lbl_zip_limit": "ZIP Part Limit",
    "settings.lbl_theme": "Theme",
    "settings.lbl_watch": "Watch Mode",
    "settings.watch": "Monitor project for changes",
    "settings.watch_clipboard": "Auto-update clipboard dump",
    "settings.lbl_diff": "Export Mode",
    "settings.lbl_git_ref": "Git Reference",
    "settings.diff_base": "Base",
    "settings.diff_target_placeholder": "target reference",
    "settings.lbl_incremental": "Incremental",
    "settings.incremental": (
        "Export only files added or modified since the last successful base snapshot"
    ),
    "settings.mb_suffix": " MB",
    "security.page_title": "Security & Filtering",
    "security.page_hint": "Configure safe export mode, secret redaction, Git patch, and custom include/exclude rules.",
    "security.lbl_safe_mode": "Security Mode",
    "security.lbl_redact": "Secret Redaction",
    "security.redact": "Redact obvious secrets in text and Git reports",
    "security.lbl_git_patch": "Git Patch",
    "security.git_patch": "Include full Git patch (disabled by default, as patches may contain secrets)",
    "security.lbl_project_files": "Project Files",
    "security.include_project": "Include project copy in final ZIP",
    "security.lbl_staging": "Staging Folder",
    "security.keep_staging": "Keep staging folder after export",
    "security.lbl_extra_ignore": "Also Ignore",
    "security.btn_rules": "Include/Exclude Rules",
    "security.btn_goals": "Prompt Goals",
    "security.btn_exportignore": "Create .exportignore",
    "security.btn_profiles": "JSON Profiles",
    "security.btn_export_settings": "Export Settings",
    "security.btn_import_settings": "Import Settings",
    "security.btn_reset_settings": "Reset Settings",
    "security.btn_history": "Export History",
    "preview.page_title": "Export Preview",
    "preview.page_hint": (
        "Review the list of files that will be included in the export. "
        "Double-click a row to manually toggle inclusion. "
        "Cyan = forced included, pink = forced excluded."
    ),
    "preview.waiting": "Waiting for export plan...",
    "preview.building": "Building export plan...",
    "preview.reset_btn": "Reset Overrides",
    "preview.legend_included": "■ Included",
    "preview.legend_excluded": "■ Excluded",
    "preview.col_file": "File",
    "preview.col_size": "Size",
    "preview.col_status": "Status",
    "preview.col_reason": "Reason",
    "preview.back_btn": "Back to Settings",
    "preview.start_btn": "Start Export  →",
    "preview.status_included": "Included",
    "preview.status_excluded": "Excluded",
    "preview.status_included_ov": "Included ✎",
    "preview.status_excluded_ov": "Excluded ✎",
    "preview.token_label": "Estimated tokens in export: ~{n}",
    "preview.stats": "Included: {inc} file(s)  │  ~{size}  │  Excluded: {exc}",
    "preview.override_note": "  │  Overrides: {n}",
    "preview.tooltip": "{name}: {tok} / {limit} tokens ({pct}%)",
    "run.page_title": "Execution Log",
    "run.page_hint": "Export runs in a separate thread. Progress, current stage, and diagnostic messages are shown here.",
    "run.waiting": "Waiting",
    "run.starting": "Starting export...",
    "result.page_title": "Export Result",
    "result.page_hint": "Final status and path to the created archive are displayed here after completion.",
    "result.not_created": "No export created yet.",
    "result.running": "Export in progress...",
    "result.success": "Export completed successfully.",
    "result.cancelled": "Export cancelled by user. A partial result may have been created.",
    "result.failed": "Export failed. Technical details saved to {log}.",
    "result.path_label": "Result Path",
    "result.path_placeholder": "Archive path will appear here",
    "result.btn_open": "Open Result",
    "result.btn_desktop": "Desktop",
    "history.page_title": "History",
    "history.page_hint": "Search, sort, rerun and compare saved exports.",
    "history.search_placeholder": "Search by project, date or path",
    "history.btn_refresh": "Refresh",
    "history.btn_open": "Open Result",
    "history.btn_repeat": "Repeat Export",
    "history.btn_compare": "Compare Two",
    "history.col_date": "Date",
    "history.col_project": "Project",
    "history.col_profile": "Profile",
    "history.col_files": "Files",
    "history.col_tokens": "Tokens",
    "history.col_status": "Status",
    "history.col_result": "Result",
    "history.status_cancelled": "cancelled",
    "history.status_done": "done",
    "snapshot.title": "Snapshot Comparison",
    "snapshot.subtitle": "Comparing Two Exports",
    "snapshot.close": "Close",
    "snapshot.no_diff": "No differences found in saved snapshots.",
    "snapshot.summary": "Added: {added}; modified: {modified}; deleted: {deleted}; LOC change: {loc:+,}.",
    "analytics.page_title": "Analytics",
    "analytics.page_hint": "Local summary of stack, languages, dependencies, Git, and project risks.",
    "analytics.btn_refresh": "Refresh Analytics",
    "analytics.chart_title": "Languages & LOC",
    "analytics.deps_title": "Dependencies",
    "analytics.col_manager": "Manager",
    "analytics.col_package": "Package",
    "analytics.col_version": "Version",
    "analytics.col_warning": "Warning",
    "analytics.git_title": "Git Activity",
    "analytics.git_not_built": "Git summary not yet built.",
    "analytics.risks_title": "Risks",
    "analytics.no_commits": "No commits found.",
    "analytics.no_risks": "No obvious risks found.",
    "analytics.no_lang_data": "No language data",
    "analytics.loading": "Collecting analytics...",
    "analytics.stat_project": "Project",
    "analytics.stat_stack": "Stack",
    "analytics.stat_files": "Files",
    "analytics.stat_loc": "LOC",
    "analytics.stat_size": "Size",
    "analytics.stat_error": "Error",
    "analytics.branch_label": "Branch: {branch}. {status}",
    "analytics.no_data_branch": "no data",
    "dialog.rules.title": "Include/Exclude Rules",
    "dialog.rules.intro": "Enter one item per line or comma-separated. Safe export rules may still block risky files.",
    "dialog.rules.excluded_files": "Exclude files / glob patterns",
    "dialog.rules.excluded_ext": "Exclude extensions",
    "dialog.rules.always_files": "Always include files / glob patterns",
    "dialog.rules.always_dirs": "Always include directories",
    "dialog.goals.title": "Prompt Goals",
    "dialog.goals.hint": "Select goals to include in AI_PROMPTS/CUSTOM_PROMPT.md.",
    "dialog.history.title": "Export History",
    "dialog.history.empty": "Export history is empty.",
    "dialog.history.close": "Close",
    "dialog.help.title": "Help — Project Exporter Desktop",
    "dialog.about.title": "About {name}",
    "dialog.about.body": (
        "{name} v{version}\n\n"
        "Creates a snapshot of your project in a format convenient for AI assistants:\n"
        "code archive, text dump, structure and analytics reports.\n\n"
        "Supported AI: Claude Code, ChatGPT, Gemini, Copilot and others."
    ),
    "dialog.plan.title": "Export Plan Confirmation",
    "dialog.plan.header": "Review the export plan before copying",
    "dialog.plan.hint": (
        "The project will not be copied until the plan is confirmed. "
        "Safe export rules remain active even when custom include rules are set."
    ),
    "dialog.plan.ok": "Start Export",
    "dialog.plan.cancel": "Cancel",
    "msg.cancel_export.title": "Cancel Export",
    "msg.cancel_export.body": "Stop the current export? A partial result may remain.",
    "msg.export_done.title": "Export Complete",
    "msg.export_done.body": "Project export created successfully.",
    "msg.export_failed.title": "Export Error",
    "msg.export_failed.body": "Export failed. Technical details saved to:\n{log}",
    "msg.stopped.title": "Stopped",
    "msg.stopped.body": "Export stopped. Check the result and log.",
    "msg.no_result.title": "No Result",
    "msg.no_result.body": "No export result yet.",
    "msg.exportignore_exists.title": ".exportignore Already Exists",
    "msg.exportignore_exists.body": ".exportignore already exists. Overwrite with template?",
    "msg.reset.title": "Reset Settings",
    "msg.reset.body": "Reset settings to safe defaults?",
    "msg.clipboard_done.title": "Dump Copied",
    "msg.clipboard_done.body": (
        "Text dump copied to clipboard.\n\n"
        "Size: {size}\nToken estimate: {summary}\n\n"
        "Paste it into your AI assistant chat."
    ),
    "msg.clipboard_failed.title": "Copy Error",
    "msg.clipboard_failed.body": "Failed to prepare dump. Technical details saved to:\n{log}",
    "msg.bad_path.title": "Invalid Project Path",
    "msg.open_error.title": "Open Error",
    "msg.open_error.body": "Could not open:\n{path}\n\n{exc}",
    "msg.preview_failed.title": "Export Plan Error",
    "msg.preview_failed.body": "Could not build export plan. Technical details saved to:\n{log}",
    "msg.export_settings_failed.title": "Export Settings Error",
    "msg.import_settings_failed.title": "Import Settings Error",
    "msg.write_error.title": "Write Error",
    "msg.write_error_exportignore": "Could not create .exportignore:\n{exc}",
    "msg.internal_error.title": "Export Error",
    "msg.internal_error.body": "Internal state was lost before launch.",
    "tray.minimized": "App minimized to tray. Quick export is available from the context menu.",
    "tray.quick_running": "Export is already running.",
    "tray.quick_done": "Quick export complete.",
    "tray.quick_failed": "Quick export failed.",
    "tray.quick_stopped": "Quick export stopped.",
    "tray.changed": "Project changed: {n} filesystem events.",
    "tray.clipboard_updated": "Clipboard dump updated: {size}, {summary}.",
    "tray.clipboard_failed": "Failed to update clipboard dump.",
    "tray.menu_quick": "Quick Export",
    "tray.menu_open": "Open",
    "tray.menu_exit": "Exit",
    "diff_hint.all": "Full export — export the entire selected project.",
    "diff_hint.last_export": "Since last export — compare current hashes with export history.",
    "diff_hint.git_ref": "Since Git reference — export changes relative to a branch, tag, or commit.",
    "diff_hint.uncommitted": "Uncommitted only — working tree changes and new files.",
    "safe_hint.safe": "Strict — excludes secrets, private keys, local databases, dumps, and archives.",
    "safe_hint.balanced": "Balanced — excludes high-risk keys and credentials, keeps more configuration.",
    "safe_hint.full": "Full — copies project files except ignored directories; for private/local exports only.",
    "preset.Claude Code.desc": "Full context: architecture, Git history, all AI reports. Optimal for Claude.",
    "preset.ChatGPT.desc": "Compact overview without Git patch. Suitable for GPT-4 with limited context window.",
    "preset.Code Review.desc": "Full code snapshot with Git patch. Ideal for detailed review.",
    "preset.Security Audit.desc": "Security-focused: configuration, dependencies, and risk analysis.",
    "preset.Онбординг.desc": "Brief overview for quickly onboarding a new developer to the project.",
}

_STRINGS: dict[str, dict[str, str]] = {"ru": _RU, "en": _EN}

# ---------------------------------------------------------------------------
# Help HTML: both language variants are stored here to avoid circular imports
# with gui.dialogs (which itself imports from i18n).
# ---------------------------------------------------------------------------

_HELP_HTML_RU = """
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

_HELP_HTML_EN = """
<html><body style="font-family:sans-serif;font-size:10pt;line-height:1.6;">
<h2>Project Exporter Desktop — User Manual</h2>

<h3>Purpose</h3>
<p>Project Exporter Desktop creates a <b>snapshot of your project</b> in a format
convenient for AI assistants: a ZIP archive with code, a text dump of all files,
and reports on structure, Git history and analytics.
The resulting package can be uploaded to Claude Code, ChatGPT, Gemini or any other AI.</p>

<h3>1. Project page</h3>
<ul>
<li><b>Project Folder</b> — specify the root of your project (Browse button or paste path manually).</li>
<li><b>Detected stack</b> — the app automatically detects technologies and excludes heavy folders
(node_modules, .venv, target, etc.).</li>
<li><b>AI Context</b> — optional text prepended to the beginning of the text dump.
Use this field to tell the AI what you want done with the code.</li>
</ul>

<h3>2. Settings page</h3>
<ul>
<li><b>Presets</b> — quick configurations for popular AI assistants (Claude Code, ChatGPT, Code Review,
Security Audit, Onboarding). Choose a preset and settings are applied automatically.</li>
<li><b>Export profile</b> — determines which files are included (ai_review, quick, minimal, security, full).</li>
<li><b>Security mode</b> — <em>safe</em> excludes secrets and binaries; <em>permissive</em> removes most restrictions.</li>
<li><b>Export mode</b> — export all files or only changed ones (relative to a branch/commit).</li>
<li><b>Theme</b> — system, light, or dark.</li>
<li><b>Watch mode</b> — automatically monitors the project folder for changes and notifies via tray.</li>
<li><b>Text file size limit</b> — limits the size of one file in the text dump (MB).</li>
</ul>

<h3>3. Security page</h3>
<ul>
<li><b>Redact secrets</b> — masks API keys, passwords, and tokens in text files.</li>
<li><b>Include Git patch</b> — adds git diff to the export package.</li>
<li><b>Also ignore folders</b> — specify folders separated by commas.</li>
<li><b>Include/Exclude Rules</b> — detailed glob rules for individual files and extensions.</li>
<li><b>.exportignore</b> — file in the project root (like .gitignore) for fine-grained exclusions.</li>
</ul>

<h3>4. Preview page</h3>
<p>Before export, a color-coded file tree is shown:</p>
<ul>
<li><span style="color:#4caf50;">■ Green</span> — file will be included</li>
<li><span style="color:#f44336;">■ Red</span> — excluded for security reasons</li>
<li><span style="color:#ff9800;">■ Orange</span> — excluded (medium priority)</li>
<li><span style="color:#9e9e9e;">■ Gray</span> — excluded (informational)</li>
<li><span style="color:#00bcd4;">■ Cyan</span> — forced included by you</li>
<li><span style="color:#e91e63;">■ Pink</span> — forced excluded by you</li>
</ul>
<p><b>Double-click</b> a file to toggle its inclusion. The <b>Copy Dump</b> button copies text
of all included files to clipboard (without creating an archive).</p>

<h3>5. Log page</h3>
<p>Shows export progress in real time. 8 steps: plan → copy → structure →
Git report → text dump → analytics → manifest → archive.</p>

<h3>6. Result page</h3>
<p>Shows the final ZIP file. The <b>Open Result</b> button opens the folder with the archive in Explorer.</p>

<h3>7. History page</h3>
<p>List of all completed exports with metadata: date, profile, token count, result path.</p>

<h3>8. Analytics page</h3>
<p>Project statistics: file count, languages, sizes, top files by volume.</p>

<h3>UI Scaling</h3>
<p>Menu <b>View → Zoom In / Zoom Out / Reset Zoom</b> (or Ctrl++/Ctrl+−/Ctrl+0)
changes the font size in the interface from 70% to 150%.</p>

<h3>System Tray</h3>
<p>When the window is closed, the app minimizes to tray. Double-click the icon to restore.
Right-click the icon opens a menu with <em>Quick Export</em> and <em>Exit</em>.</p>

<h3>Tools (menu)</h3>
<ul>
<li><b>Include/Exclude Rules</b> — glob rule dialog.</li>
<li><b>Prompt Goals</b> — select sections for CUSTOM_PROMPT.md.</li>
<li><b>Create .exportignore</b> — creates an exclusion file template in the project root.</li>
<li><b>Export / Import Settings</b> — save and load configuration as JSON.</li>
<li><b>Reset Settings</b> — restore factory defaults.</li>
</ul>

<h3>Keyboard Shortcuts</h3>
<table border="0" cellpadding="4">
<tr><td><b>Ctrl++</b></td><td>Zoom In</td></tr>
<tr><td><b>Ctrl+−</b></td><td>Zoom Out</td></tr>
<tr><td><b>Ctrl+0</b></td><td>Reset Zoom (100%)</td></tr>
<tr><td><b>F1</b></td><td>Open Help</td></tr>
</table>

<h3>Tips</h3>
<ul>
<li>Use the <em>AI Context</em> field so the AI immediately understands what you want.</li>
<li>The <em>Claude Code</em> preset is optimal for Anthropic Claude; removes file size limit.</li>
<li>The <em>ChatGPT</em> preset limits files to 1 MB to fit the 128K context window.</li>
<li>The <em>Quick Export</em> from tray uses current settings without opening the window.</li>
<li>The app log file is located in <code>%LOCALAPPDATA%/project_exporter_desktop/</code>.</li>
</ul>
</body></html>
"""


# ---------------------------------------------------------------------------
# I18n singleton
# ---------------------------------------------------------------------------


class I18n(QObject):
    language_changed = Signal()

    def __init__(self) -> None:
        super().__init__()
        self._lang = "ru"

    @property
    def lang(self) -> str:
        return self._lang

    def set_language(self, lang: str) -> None:
        if lang not in _STRINGS:
            return
        if lang == self._lang:
            return
        self._lang = lang
        self.language_changed.emit()

    def t(self, key: str) -> str:
        table = _STRINGS.get(self._lang, _RU)
        if key in table:
            return table[key]
        return _RU.get(key, key)

    def init_language(self, lang: str) -> None:
        """Set language at startup without emitting language_changed signal."""
        if lang in _STRINGS:
            self._lang = lang

    def help_html(self) -> str:
        return _HELP_HTML_EN if self._lang == "en" else _HELP_HTML_RU


_i18n = I18n()


def t(key: str) -> str:
    return _i18n.t(key)


def get_i18n() -> I18n:
    return _i18n


def set_language(lang: str) -> None:
    _i18n.set_language(lang)
