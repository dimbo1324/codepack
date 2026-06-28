# PySide6 GUI module: keep export/business logic in services and communicate through workers/signals.

from __future__ import annotations

import os
import re
import subprocess
import sys
import threading
from dataclasses import replace
from pathlib import Path

from PySide6.QtCore import QFileSystemWatcher, Qt, QTimer
from PySide6.QtGui import QAction, QCursor, QIcon
from PySide6.QtWidgets import (
    QApplication,
    QDialog,
    QFileDialog,
    QHBoxLayout,
    QLabel,
    QMainWindow,
    QMenu,
    QMessageBox,
    QPushButton,
    QStackedWidget,
    QSystemTrayIcon,
    QVBoxLayout,
    QWidget,
)

from ..config import Config
from ..constants import (
    APP_NAME,
    APP_VERSION,
    DIFF_EXPORT_MODES,
    SAFE_EXPORT_MODES,
    SETTINGS_FILE,
)
from ..i18n import get_i18n, set_language, t
from ..services.export_history import load_export_history
from ..services.export_ignore import EXPORTIGNORE_TEMPLATE
from ..services.export_profiles import (
    apply_custom_profile_if_needed,
    ensure_user_profiles_file,
    load_profile_catalog,
)
from ..utils.path_utils import desktop_path, validate_source_root
from .components.sidebar import Sidebar
from .dialogs import HelpDialog, HistoryDialog, PromptGoalsDialog, RulesDialog
from .logging import app_log_file, append_app_log
from .pages.analytics_page import AnalyticsPage
from .pages.history_page import HistoryPage
from .pages.preview_page import PreviewPage
from .pages.project_page import ProjectPage
from .pages.result_page import ResultPage
from .pages.run_page import RunPage
from .pages.security_page import SecurityPage
from .pages.settings_page import SettingsPage
from .resources import asset_path, read_text_resource, style_path
from .workers import AnalyticsWorker, ClipboardExportWorker, ExportWorker, PlanPreviewWorker

_PAGE_PROJECT = 0
_PAGE_SETTINGS = 1
_PAGE_SECURITY = 2
_PAGE_PREVIEW = 3
_PAGE_RUN = 4
_PAGE_RESULT = 5
_PAGE_HISTORY = 6
_PAGE_ANALYTICS = 7


class MainWindow(QMainWindow):
    def __init__(self) -> None:
        super().__init__()
        self.config = Config.load()
        self.profile_catalog = load_profile_catalog()
        self.cancel_event = threading.Event()
        self.preview_worker: PlanPreviewWorker | None = None
        self.export_worker: ExportWorker | None = None
        self.clipboard_worker: ClipboardExportWorker | None = None
        self.analytics_worker: AnalyticsWorker | None = None
        self.pending_source_root: Path | None = None
        self.pending_config: Config | None = None
        self.last_result_path: Path | None = None
        self.log_file = app_log_file()
        self._last_preview_text: str = ""
        self._tray_quick_mode = False
        self._allow_close = False
        self._watch_change_count = 0
        self._watch_clipboard_mode = False
        self._zoom_factor: float = 1.0
        self._zoom_in_action: QAction | None = None
        self._zoom_out_action: QAction | None = None
        self._lang_action: QAction | None = None
        self.tray_menu: QMenu | None = None
        self.tray_quick_action: QAction | None = None
        self.tray_open_action: QAction | None = None
        self.tray_exit_action: QAction | None = None

        # Apply saved language before building UI (no signal — widgets not yet created)
        lang = getattr(self.config, "language", "ru")
        get_i18n().init_language(lang)

        self.setWindowTitle(f"{APP_NAME} v{APP_VERSION}")
        self.resize(1100, 780)
        self.setMinimumSize(820, 540)
        self._apply_icon()
        self._build_tray()
        self._build_watcher()
        self._build_menu()
        self._build_ui()
        self._load_config_to_ui()
        self._apply_configured_theme()
        self._sync_watcher()
        self._apply_zoom(self.config.normalized_ui_zoom())
        self._append_log(f"{APP_NAME} v{APP_VERSION} готов к работе.")
        self._append_log("Выберите папку проекта и создайте экспорт-пакет.")

        # Wire language change signal
        get_i18n().language_changed.connect(self._on_language_changed)

    def _apply_icon(self) -> None:
        icon = asset_path("ICO.ico")
        if icon.exists():
            self.setWindowIcon(QIcon(str(icon)))

    def _build_menu(self) -> None:
        file_menu = self.menuBar().addMenu(t("menu.file"))
        open_desktop = QAction(t("menu.file.desktop"), self)
        open_desktop.triggered.connect(self._open_desktop)
        file_menu.addAction(open_desktop)
        open_result = QAction(t("menu.file.last_result"), self)
        open_result.triggered.connect(self._open_last_result)
        file_menu.addAction(open_result)
        file_menu.addSeparator()
        exit_action = QAction(t("menu.file.exit"), self)
        exit_action.triggered.connect(self._exit_from_tray)
        file_menu.addAction(exit_action)

        tools_menu = self.menuBar().addMenu(t("menu.tools"))
        tools_menu.addAction(t("menu.tools.rules"), self._edit_rules)
        tools_menu.addAction(t("menu.tools.prompt_goals"), self._edit_prompt_goals)
        tools_menu.addAction(
            t("menu.tools.create_exportignore"), self._create_exportignore_template
        )
        tools_menu.addSeparator()
        tools_menu.addAction(t("menu.tools.export_settings"), self._export_settings)
        tools_menu.addAction(t("menu.tools.import_settings"), self._import_settings)
        tools_menu.addAction(t("menu.tools.reset_settings"), self._reset_settings)
        tools_menu.addSeparator()
        tools_menu.addAction(t("menu.tools.history"), self._show_history)

        view_menu = self.menuBar().addMenu(t("menu.view"))
        zoom_in_act = QAction(t("menu.view.zoom_in"), self)
        zoom_in_act.setShortcut("Ctrl+=")
        zoom_in_act.triggered.connect(self._zoom_in)
        view_menu.addAction(zoom_in_act)
        self._zoom_in_action = zoom_in_act

        zoom_out_act = QAction(t("menu.view.zoom_out"), self)
        zoom_out_act.setShortcut("Ctrl+-")
        zoom_out_act.triggered.connect(self._zoom_out)
        view_menu.addAction(zoom_out_act)
        self._zoom_out_action = zoom_out_act

        zoom_reset_act = QAction(t("menu.view.zoom_reset"), self)
        zoom_reset_act.setShortcut("Ctrl+0")
        zoom_reset_act.triggered.connect(self._zoom_reset)
        view_menu.addAction(zoom_reset_act)

        view_menu.addSeparator()
        lang_act = QAction(t("menu.view.language"), self)
        lang_act.triggered.connect(self._toggle_language)
        view_menu.addAction(lang_act)
        self._lang_action = lang_act

        help_menu = self.menuBar().addMenu(t("menu.help"))
        help_act = QAction(t("menu.help.manual"), self)
        help_act.setShortcut("F1")
        help_act.triggered.connect(self._show_help)
        help_menu.addAction(help_act)
        help_menu.addSeparator()
        about_act = QAction(f"{t('menu.help.about')} {APP_NAME}", self)
        about_act.triggered.connect(self._show_about)
        help_menu.addAction(about_act)

    def _build_tray(self) -> None:
        self.tray_icon = QSystemTrayIcon(self)
        icon = self.windowIcon()
        if icon.isNull():
            icon = QIcon(str(asset_path("ICO.ico")))
        self.tray_icon.setIcon(icon)
        self.tray_icon.setToolTip(APP_NAME)
        menu = QMenu(self)
        menu.setObjectName("TrayMenu")
        self.tray_quick_action = menu.addAction(t("tray.menu_quick"), self._quick_export_from_tray)
        self.tray_open_action = menu.addAction(t("tray.menu_open"), self._show_from_tray)
        menu.addSeparator()
        self.tray_exit_action = menu.addAction(t("tray.menu_exit"), self._exit_from_tray)
        menu.hovered.connect(self._on_tray_menu_hovered)
        self.tray_menu = menu
        self.tray_icon.setContextMenu(menu)
        self.tray_icon.activated.connect(self._on_tray_activated)
        self.tray_icon.show()

    def _build_watcher(self) -> None:
        self.project_watcher = QFileSystemWatcher(self)
        self.project_watcher.directoryChanged.connect(self._on_project_changed)
        self._watch_timer = QTimer(self)
        self._watch_timer.setSingleShot(True)
        self._watch_timer.timeout.connect(self._flush_watch_notification)

    def closeEvent(self, event) -> None:
        if self._allow_close or not self.tray_icon.isVisible():
            event.accept()
            return
        event.ignore()
        self.hide()
        self.tray_icon.showMessage(
            APP_NAME,
            t("tray.minimized"),
            QSystemTrayIcon.MessageIcon.Information,
            2500,
        )

    def _on_tray_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        if reason == QSystemTrayIcon.ActivationReason.DoubleClick:
            self._show_from_tray()
        elif reason == QSystemTrayIcon.ActivationReason.Trigger:
            if self.tray_menu is not None:
                self.tray_menu.popup(QCursor.pos())

    def _on_tray_menu_hovered(self, action: QAction) -> None:
        if action.text():
            self.statusBar().showMessage(action.text(), 1500)

    def _show_from_tray(self) -> None:
        self.showNormal()
        self.activateWindow()

    def _exit_from_tray(self) -> None:
        self._allow_close = True
        if self.tray_icon.isVisible():
            self.tray_icon.hide()
        self.close()
        QApplication.quit()

    def _quick_export_from_tray(self) -> None:
        if self.export_worker and self.export_worker.isRunning():
            self.tray_icon.showMessage(
                APP_NAME,
                t("tray.quick_running"),
                QSystemTrayIcon.MessageIcon.Information,
                2500,
            )
            return
        try:
            source_root = validate_source_root(self.config.last_root)
        except Exception as exc:
            self.tray_icon.showMessage(
                APP_NAME, str(exc), QSystemTrayIcon.MessageIcon.Warning, 3500
            )
            return
        self._tray_quick_mode = True
        self._run_export(source_root, replace(self.config), {})

    def _apply_configured_theme(self) -> None:
        theme = self.config.normalized_theme()
        if theme == "system":
            app = QApplication.instance()
            color_scheme = app.styleHints().colorScheme()
            theme = "dark" if color_scheme == Qt.ColorScheme.Dark else "light"
        qss_name = "app_dark.qss" if theme == "dark" else "app_light.qss"
        qss = read_text_resource(style_path(qss_name)) or read_text_resource(style_path())
        QApplication.instance().setStyleSheet(qss)

    def _sync_watcher(self) -> None:
        if not hasattr(self, "project_watcher"):
            return
        paths = self.project_watcher.directories() + self.project_watcher.files()
        if paths:
            self.project_watcher.removePaths(paths)
        if not self.config.watch_enabled:
            return
        root_text = (
            self.page_project.get_root() if hasattr(self, "page_project") else self.config.last_root
        )
        try:
            root = validate_source_root(root_text)
        except Exception:
            return
        watch_dirs: list[str] = []
        ignored = self.config.effective_ignored_dirs()
        for current_dir, dirnames, _filenames in os.walk(root, topdown=True, followlinks=False):
            current = Path(current_dir)
            dirnames[:] = [
                dirname
                for dirname in dirnames
                if not (current / dirname).is_symlink()
                and dirname.casefold() not in ignored
                and len(watch_dirs) < 256
            ]
            watch_dirs.append(str(current))
            if len(watch_dirs) >= 256:
                break
        if watch_dirs:
            self.project_watcher.addPaths(watch_dirs)

    def _on_project_changed(self, _path: str) -> None:
        if not self.config.watch_enabled:
            return
        self._watch_change_count += 1
        self._watch_timer.start(900)

    def _flush_watch_notification(self) -> None:
        count = self._watch_change_count
        self._watch_change_count = 0
        self.tray_icon.showMessage(
            APP_NAME,
            t("tray.changed").format(n=count),
            QSystemTrayIcon.MessageIcon.Information,
            3000,
        )
        self._sync_watcher()
        if self.config.watch_clipboard_auto_update:
            self._watch_clipboard_mode = True
            self._start_clipboard_export()

    def _build_ui(self) -> None:
        central = QWidget()
        root = QHBoxLayout(central)
        root.setContentsMargins(0, 0, 0, 0)
        root.setSpacing(0)

        self.sidebar = Sidebar()
        self.sidebar.page_requested.connect(self._set_page)
        self.sidebar.open_desktop_requested.connect(self._open_desktop)
        root.addWidget(self.sidebar)

        content_host = QWidget()
        content_layout = QVBoxLayout(content_host)
        content_layout.setContentsMargins(24, 22, 24, 22)
        content_layout.setSpacing(16)

        self.page_project = ProjectPage()
        self.page_settings = SettingsPage(self.profile_catalog)
        self.page_security = SecurityPage()
        self.page_preview = PreviewPage()
        self.page_run = RunPage()
        self.page_result = ResultPage()
        self.page_history = HistoryPage()
        self.page_analytics = AnalyticsPage()

        sec = self.page_security
        sec.edit_rules_requested.connect(self._edit_rules)
        sec.edit_prompt_goals_requested.connect(self._edit_prompt_goals)
        sec.create_exportignore_requested.connect(self._create_exportignore_template)
        sec.open_profiles_json_requested.connect(self._open_profiles_json)
        sec.export_settings_requested.connect(self._export_settings)
        sec.import_settings_requested.connect(self._import_settings)
        sec.reset_settings_requested.connect(self._reset_settings)
        sec.show_history_requested.connect(self._show_history)

        self.page_result.open_result_requested.connect(self._open_last_result)
        self.page_result.open_desktop_requested.connect(self._open_desktop)

        self.page_history.open_result_requested.connect(self._open_path)
        self.page_history.repeat_export_requested.connect(self._repeat_history_export)
        self.page_analytics.refresh_button.clicked.connect(self._refresh_analytics)

        self.page_preview.export_confirmed.connect(self._on_preview_confirmed)
        self.page_preview.export_cancelled.connect(self._on_preview_back)

        self.page_project.root_edit.textChanged.connect(self._on_root_changed)

        self.stack = QStackedWidget()
        for page in [
            self.page_project,
            self.page_settings,
            self.page_security,
            self.page_preview,
            self.page_run,
            self.page_result,
            self.page_history,
            self.page_analytics,
        ]:
            self.stack.addWidget(page)
        content_layout.addWidget(self.stack, 1)

        bottom = QHBoxLayout()
        self.status_label = QLabel(t("status.ready"))
        self.status_label.setObjectName("PageHint")
        bottom.addWidget(self.status_label, 1)
        self.open_result_button = QPushButton(t("btn.open_result"))
        self.open_result_button.setEnabled(False)
        self.open_result_button.clicked.connect(self._open_last_result)
        bottom.addWidget(self.open_result_button)
        self.cancel_button = QPushButton(t("btn.cancel"))
        self.cancel_button.setObjectName("DangerButton")
        self.cancel_button.setEnabled(False)
        self.cancel_button.clicked.connect(self._cancel_export)
        bottom.addWidget(self.cancel_button)
        self.clipboard_button = QPushButton(t("btn.clipboard"))
        self.clipboard_button.setToolTip(t("btn.clipboard.tip"))
        self.clipboard_button.clicked.connect(self._start_clipboard_export)
        bottom.addWidget(self.clipboard_button)
        self.codex_button = QPushButton(t("btn.codex"))
        self.codex_button.clicked.connect(lambda: self._start(codex_package=True))
        bottom.addWidget(self.codex_button)
        self.start_button = QPushButton(t("btn.create_export"))
        self.start_button.setObjectName("PrimaryButton")
        self.start_button.clicked.connect(lambda: self._start(codex_package=False))
        bottom.addWidget(self.start_button)
        content_layout.addLayout(bottom)

        root.addWidget(content_host, 1)
        self.setCentralWidget(central)
        self._set_page(_PAGE_PROJECT)

    # ------------------------------------------------------------------
    # Language support
    # ------------------------------------------------------------------

    def _toggle_language(self) -> None:
        current = get_i18n().lang
        new_lang = "en" if current == "ru" else "ru"
        set_language(new_lang)
        self.config.language = new_lang
        self.config.save()

    def _on_language_changed(self) -> None:
        # Update language toggle action label
        if self._lang_action is not None:
            self._lang_action.setText(t("menu.view.language"))
        if self.tray_quick_action is not None:
            self.tray_quick_action.setText(t("tray.menu_quick"))
        if self.tray_open_action is not None:
            self.tray_open_action.setText(t("tray.menu_open"))
        if self.tray_exit_action is not None:
            self.tray_exit_action.setText(t("tray.menu_exit"))

        # Update bottom bar buttons
        self.open_result_button.setText(t("btn.open_result"))
        self.cancel_button.setText(t("btn.cancel"))
        self.clipboard_button.setText(t("btn.clipboard"))
        self.clipboard_button.setToolTip(t("btn.clipboard.tip"))
        self.codex_button.setText(t("btn.codex"))
        self.start_button.setText(t("btn.create_export"))

        # Update status label if it's in a "ready" state
        current_status = self.status_label.text()
        for key in ("status.ready", "status.building", "status.exporting", "status.clipboard"):
            # Check both RU and EN forms
            from ..i18n import _STRINGS

            for lang_strings in _STRINGS.values():
                if current_status == lang_strings.get(key, ""):
                    self.status_label.setText(t(key))
                    break

        # Retranslate sidebar
        self.sidebar.retranslate()

        # Retranslate all pages
        self.page_project.retranslate()
        self.page_settings.retranslate()
        self.page_security.retranslate()
        self.page_preview.retranslate()
        self.page_run.retranslate()
        self.page_result.retranslate()
        self.page_history.retranslate()
        self.page_analytics.retranslate()

    # ------------------------------------------------------------------

    def _set_page(self, index: int) -> None:
        if hasattr(self, "stack"):
            self.stack.setCurrentIndex(index)
        self.sidebar.set_active_page(index)
        if index == _PAGE_HISTORY and hasattr(self, "page_history"):
            self.page_history.set_history(load_export_history())
        if index == _PAGE_ANALYTICS and hasattr(self, "page_analytics"):
            self._refresh_analytics()

    def _set_running(self, running: bool, preview: bool = False) -> None:
        self.start_button.setEnabled(not running)
        self.codex_button.setEnabled(not running)
        self.clipboard_button.setEnabled(not running)
        self.cancel_button.setEnabled(running and not preview)
        self.open_result_button.setEnabled(
            bool(self.last_result_path and self.last_result_path.exists())
        )
        if preview:
            self.status_label.setText(t("status.building"))
        elif running:
            self.status_label.setText(t("status.exporting"))
        else:
            self.status_label.setText(t("status.ready"))

    def _load_config_to_ui(self) -> None:
        self.page_project.set_root(self.config.last_root)
        self.page_project.set_developer_context(self.config.developer_context)
        self.page_settings.load_from_config(self.config)
        self.page_security.load_from_config(self.config)

    def _config_from_ui(self, save: bool = False) -> Config:
        cfg = self.config if save else replace(self.config)
        cfg.last_root = self.page_project.get_root() or str(Path.home())
        cfg.developer_context = self.page_project.get_developer_context()

        sp = self.page_settings
        cfg.text_file_size_limit_enabled = sp.text_limit_checkbox.isChecked()
        cfg.max_text_file_mb = max(1, sp.max_text_mb_spin.value())
        cfg.zip_part_limit_mb = max(1, sp.zip_limit_spin.value())
        cfg.theme = sp.theme_combo.currentText().strip()
        cfg.watch_enabled = sp.watch_checkbox.isChecked()
        cfg.watch_clipboard_auto_update = sp.watch_clipboard_checkbox.isChecked()
        cfg.export_profile = sp.profile_combo.currentText().strip()
        cfg.diff_export_mode = sp.diff_combo.currentText().strip()
        cfg.diff_base_ref = sp.diff_base_edit.text().strip() or "HEAD"
        cfg.diff_target_ref = sp.diff_target_edit.text().strip()
        cfg.incremental_export_enabled = False

        sc = self.page_security
        cfg.redact_secrets = sc.redact_checkbox.isChecked()
        cfg.include_git_patch = sc.git_patch_checkbox.isChecked()
        cfg.include_project_in_zip = sc.include_project_checkbox.isChecked()
        cfg.keep_staging_folder = sc.keep_staging_checkbox.isChecked()
        cfg.safe_export_mode = sc.safe_mode_combo.currentText().strip()

        extras: list[str] = []
        for token in re.split(r"[,;\n]", sc.extra_ignored_edit.text()):
            value = token.strip()
            if value and value not in extras:
                extras.append(value)
        cfg.extra_ignored_dirs = extras

        cfg = apply_custom_profile_if_needed(cfg.export_profile, cfg)
        if cfg.safe_export_mode not in SAFE_EXPORT_MODES:
            cfg.safe_export_mode = "safe"
        if cfg.diff_export_mode not in DIFF_EXPORT_MODES:
            cfg.diff_export_mode = "all"
        return cfg

    def _save_config_from_ui(self) -> None:
        self.config = self._config_from_ui(save=True)
        self.config.save()
        self._apply_configured_theme()
        self._sync_watcher()

    def _codex_config(self, base: Config) -> Config:
        from ..constants import MAX_ARCHIVE_PART_MB

        return replace(
            base,
            export_profile="ai_review",
            safe_export_mode="safe",
            text_file_size_limit_enabled=False,
            redact_secrets=True,
            include_git_patch=False,
            include_project_in_zip=True,
            diff_export_mode="all",
            incremental_export_enabled=False,
            zip_part_limit_mb=MAX_ARCHIVE_PART_MB,
        )

    def _on_root_changed(self, text: str) -> None:
        from ..services.stack_detector import format_stack_label

        root = Path(text.strip()) if text.strip() else None
        label = format_stack_label(root) if root and root.is_dir() else ""
        self.page_project.set_detected_stack(label)

    def _validate_source_root(self) -> Path | None:
        try:
            return validate_source_root(self.page_project.get_root())
        except Exception as exc:
            QMessageBox.critical(self, t("msg.bad_path.title"), str(exc))
            return None

    def _start(self, codex_package: bool = False) -> None:
        if self.export_worker and self.export_worker.isRunning():
            return
        if self.preview_worker and self.preview_worker.isRunning():
            return

        source_root = self._validate_source_root()
        if source_root is None:
            return

        self._save_config_from_ui()
        run_config = self._codex_config(self.config) if codex_package else replace(self.config)
        self.pending_source_root = source_root
        self.pending_config = run_config

        self.page_preview.reset()
        self._set_page(_PAGE_PREVIEW)
        self._append_log(
            f"Строится план экспорта... профиль={run_config.normalized_export_profile()}, "
            f"режим={run_config.normalized_safe_export_mode()}, "
            f"diff={run_config.normalized_diff_export_mode()}"
        )
        self._set_running(True, preview=True)
        self.preview_worker = PlanPreviewWorker(source_root, run_config, self)
        self.preview_worker.finished_plan.connect(self._on_plan_ready)
        self.preview_worker.finished_preview.connect(self._on_preview_text_ready)
        self.preview_worker.failed.connect(self._on_preview_failed)
        self.preview_worker.start()

    def _on_plan_ready(self, plan: object) -> None:
        self._set_running(False)
        self.page_preview.populate(plan)

    def _on_preview_text_ready(self, preview_text: str) -> None:
        self._last_preview_text = preview_text

    def _on_preview_confirmed(self, overrides: object) -> None:
        file_overrides: dict[str, bool] = overrides if isinstance(overrides, dict) else {}
        if self.pending_source_root is None or self.pending_config is None:
            QMessageBox.critical(self, t("msg.internal_error.title"), t("msg.internal_error.body"))
            return
        self._run_export(self.pending_source_root, self.pending_config, file_overrides)

    def _on_preview_back(self) -> None:
        self._set_page(_PAGE_SETTINGS)

    def _on_preview_failed(self, traceback_text: str) -> None:
        self._set_running(False)
        self._append_diagnostic(traceback_text)
        QMessageBox.critical(
            self,
            t("msg.preview_failed.title"),
            t("msg.preview_failed.body").format(log=self.log_file),
        )

    def _run_export(
        self,
        source_root: Path,
        config: Config,
        file_overrides: dict[str, bool] | None = None,
    ) -> None:
        self.cancel_event.clear()
        self.last_result_path = None
        self.open_result_button.setEnabled(False)
        self.page_result.set_running()
        self.page_run.reset()
        self._set_page(_PAGE_RUN)
        self._set_running(True)
        self._append_log("Запуск потока экспорта...")
        self.export_worker = ExportWorker(
            source_root,
            config,
            self.cancel_event,
            self,
            file_overrides=file_overrides,
        )
        self.export_worker.log_message.connect(self._append_log)
        self.export_worker.progress_changed.connect(self._handle_progress)
        self.export_worker.finished_success.connect(self._on_export_finished)
        self.export_worker.failed.connect(self._on_export_failed)
        self.export_worker.start()

    def _cancel_export(self) -> None:
        if not (self.export_worker and self.export_worker.isRunning()):
            return
        reply = QMessageBox.question(
            self,
            t("msg.cancel_export.title"),
            t("msg.cancel_export.body"),
        )
        if reply == QMessageBox.StandardButton.Yes:
            self.cancel_event.set()
            self._append_log("Запрошена отмена...")

    def _on_export_finished(self, result: object) -> None:
        data = result if isinstance(result, dict) else {}
        result_path = data.get("result_path")
        if isinstance(result_path, Path):
            self.last_result_path = result_path
        cancelled = bool(data.get("cancelled"))
        self._set_running(False)
        self.open_result_button.setEnabled(
            bool(self.last_result_path and self.last_result_path.exists())
        )
        self._set_page(_PAGE_RESULT)
        if cancelled:
            self.page_result.set_cancelled()
            if self._tray_quick_mode:
                self.tray_icon.showMessage(
                    APP_NAME,
                    t("tray.quick_stopped"),
                    QSystemTrayIcon.MessageIcon.Warning,
                    3000,
                )
            else:
                QMessageBox.warning(self, t("msg.stopped.title"), t("msg.stopped.body"))
        else:
            self.page_result.set_success(self.last_result_path)
            if self._tray_quick_mode:
                self.tray_icon.showMessage(
                    APP_NAME,
                    t("tray.quick_done"),
                    QSystemTrayIcon.MessageIcon.Information,
                    3000,
                )
            else:
                QMessageBox.information(self, t("msg.export_done.title"), t("msg.export_done.body"))
        self.status_label.setText(t("status.ready"))
        self._tray_quick_mode = False

    def _on_export_failed(self, traceback_text: str) -> None:
        self._set_running(False)
        self._append_diagnostic(traceback_text)
        self._set_page(_PAGE_RESULT)
        self.page_result.set_failed(str(self.log_file))
        if self._tray_quick_mode:
            self.tray_icon.showMessage(
                APP_NAME,
                t("tray.quick_failed"),
                QSystemTrayIcon.MessageIcon.Critical,
                4000,
            )
        else:
            QMessageBox.critical(
                self,
                t("msg.export_failed.title"),
                t("msg.export_failed.body").format(log=self.log_file),
            )
        self._tray_quick_mode = False

    def _handle_progress(self, percent: int, stage: str, current: str) -> None:
        self.page_run.set_progress(percent, stage, current)

    def _append_log(self, message: str) -> None:
        append_app_log(message)
        self.page_run.append_log(message)

    def _append_diagnostic(self, traceback_text: str) -> None:
        append_app_log("Техническая трассировка:")
        append_app_log(traceback_text)
        self._append_log(f"Технические подробности записаны в: {self.log_file}")

    def _start_clipboard_export(self) -> None:
        if self.clipboard_worker and self.clipboard_worker.isRunning():
            return
        source_root = self._validate_source_root()
        if source_root is None:
            return
        self._save_config_from_ui()
        self.clipboard_button.setEnabled(False)
        self.status_label.setText(t("status.clipboard"))
        self.clipboard_worker = ClipboardExportWorker(source_root, self.config, self)
        self.clipboard_worker.finished.connect(self._on_clipboard_ready)
        self.clipboard_worker.failed.connect(self._on_clipboard_failed)
        self.clipboard_worker.start()

    def _on_clipboard_ready(self, text: str, byte_count: int, summary: str) -> None:
        QApplication.clipboard().setText(text)
        self.clipboard_button.setEnabled(True)
        self.status_label.setText(t("status.ready"))
        from ..utils.text_utils import format_bytes

        if self._watch_clipboard_mode:
            self.tray_icon.showMessage(
                APP_NAME,
                t("tray.clipboard_updated").format(size=format_bytes(byte_count), summary=summary),
                QSystemTrayIcon.MessageIcon.Information,
                3000,
            )
            self._watch_clipboard_mode = False
            return

        QMessageBox.information(
            self,
            t("msg.clipboard_done.title"),
            t("msg.clipboard_done.body").format(size=format_bytes(byte_count), summary=summary),
        )

    def _on_clipboard_failed(self, traceback_text: str) -> None:
        self.clipboard_button.setEnabled(True)
        self.status_label.setText(t("status.ready"))
        self._append_diagnostic(traceback_text)
        if self._watch_clipboard_mode:
            self.tray_icon.showMessage(
                APP_NAME,
                t("tray.clipboard_failed"),
                QSystemTrayIcon.MessageIcon.Warning,
                3500,
            )
            self._watch_clipboard_mode = False
            return
        QMessageBox.critical(
            self,
            t("msg.clipboard_failed.title"),
            t("msg.clipboard_failed.body").format(log=self.log_file),
        )

    def _edit_rules(self) -> None:
        dialog = RulesDialog(
            self.config.custom_excluded_files,
            self.config.custom_excluded_extensions,
            self.config.always_include_files,
            self.config.always_include_dirs,
            self,
        )
        if dialog.exec() != QDialog.DialogCode.Accepted:
            return
        values = dialog.values()
        self.config.custom_excluded_files = values["excluded_files"]
        self.config.custom_excluded_extensions = values["excluded_extensions"]
        self.config.always_include_files = values["always_include_files"]
        self.config.always_include_dirs = values["always_include_dirs"]
        self.config.save()
        self._append_log("Правила включения/исключения сохранены.")

    def _edit_prompt_goals(self) -> None:
        dialog = PromptGoalsDialog(self.config.prompt_goals, self)
        if dialog.exec() != QDialog.DialogCode.Accepted:
            return
        self.config.prompt_goals = dialog.goals()
        self.config.save()
        self._append_log("Цели промптов сохранены.")

    def _create_exportignore_template(self) -> None:
        source_root = self._validate_source_root()
        if source_root is None:
            return
        target = source_root / ".exportignore"
        if target.exists():
            reply = QMessageBox.question(
                self,
                t("msg.exportignore_exists.title"),
                t("msg.exportignore_exists.body"),
            )
            if reply != QMessageBox.StandardButton.Yes:
                return
        try:
            target.write_text(EXPORTIGNORE_TEMPLATE, encoding="utf-8", newline="\n")
            self._append_log(f"Создан .exportignore: {target}")
            self._open_path(target)
        except Exception as exc:
            QMessageBox.critical(
                self, t("msg.write_error.title"), t("msg.write_error_exportignore").format(exc=exc)
            )

    def _export_settings(self) -> None:
        self._save_config_from_ui()
        target, _ = QFileDialog.getSaveFileName(
            self,
            t("menu.tools.export_settings"),
            "project_exporter_settings.json",
            "JSON-файлы (*.json)",
        )
        if not target:
            return
        try:
            Config.export_settings(Path(target), self.config)
            self._append_log(f"Настройки экспортированы: {target}")
        except Exception as exc:
            QMessageBox.critical(self, t("msg.export_settings_failed.title"), str(exc))

    def _import_settings(self) -> None:
        source, _ = QFileDialog.getOpenFileName(
            self, t("menu.tools.import_settings"), "", "JSON-файлы (*.json)"
        )
        if not source:
            return
        try:
            self.config = Config.import_settings(Path(source))
            self.config.save()
            self._load_config_to_ui()
            self._append_log(f"Настройки импортированы: {source}")
        except Exception as exc:
            QMessageBox.critical(self, t("msg.import_settings_failed.title"), str(exc))

    def _reset_settings(self) -> None:
        reply = QMessageBox.question(self, t("msg.reset.title"), t("msg.reset.body"))
        if reply != QMessageBox.StandardButton.Yes:
            return
        try:
            SETTINGS_FILE.unlink(missing_ok=True)
        except Exception:
            pass
        self.config = Config()
        self._load_config_to_ui()
        self._append_log("Настройки сброшены к значениям по умолчанию.")

    def _show_history(self) -> None:
        HistoryDialog(load_export_history(), self).exec()

    def _repeat_history_export(self, entry: object) -> None:
        if not isinstance(entry, dict):
            return
        source_root = str(entry.get("source_root", "")).strip()
        if not source_root:
            return
        self.config.last_root = source_root
        self.config.export_profile = str(entry.get("profile", self.config.export_profile))
        self.config.safe_export_mode = str(
            entry.get("safe_export_mode", self.config.safe_export_mode)
        )
        self.config.diff_export_mode = str(
            entry.get("diff_export_mode", self.config.diff_export_mode)
        )
        self.config.save()
        self._load_config_to_ui()
        self._set_page(_PAGE_PROJECT)
        self._start(codex_package=False)

    def _refresh_analytics(self) -> None:
        if self.analytics_worker and self.analytics_worker.isRunning():
            return
        source_root = self._validate_source_root()
        if source_root is None:
            return
        self._save_config_from_ui()
        self.page_analytics.set_loading(True)
        self.analytics_worker = AnalyticsWorker(source_root, self.config, self)
        self.analytics_worker.finished_report.connect(self._on_analytics_ready)
        self.analytics_worker.failed.connect(self._on_analytics_failed)
        self.analytics_worker.start()

    def _on_analytics_ready(self, report: object) -> None:
        self.page_analytics.populate(report)

    def _on_analytics_failed(self, traceback_text: str) -> None:
        self._append_diagnostic(traceback_text)
        self.page_analytics.set_error(
            "Не удалось собрать аналитику. Подробности записаны в журнал."
        )

    def _apply_zoom(self, factor: float) -> None:
        from PySide6.QtGui import QFont

        _min_zoom = 0.7
        _max_zoom = 1.5
        _step = 0.1
        self._zoom_factor = max(_min_zoom, min(_max_zoom, round(factor / _step) * _step))
        base_pt = 9
        scaled_pt = max(7, round(base_pt * self._zoom_factor))
        app = QApplication.instance()
        f = QFont(app.font())
        f.setPointSize(scaled_pt)
        app.setFont(f)
        if self._zoom_in_action is not None:
            self._zoom_in_action.setEnabled(self._zoom_factor < _max_zoom - 0.01)
        if self._zoom_out_action is not None:
            self._zoom_out_action.setEnabled(self._zoom_factor > _min_zoom + 0.01)
        self.config.ui_zoom = self._zoom_factor
        self.config.save()

    def _zoom_in(self) -> None:
        self._apply_zoom(self._zoom_factor + 0.1)

    def _zoom_out(self) -> None:
        self._apply_zoom(self._zoom_factor - 0.1)

    def _zoom_reset(self) -> None:
        self._apply_zoom(1.0)

    def _show_help(self) -> None:
        HelpDialog(self).exec()

    def _show_about(self) -> None:
        QMessageBox.information(
            self,
            t("dialog.about.title").format(name=APP_NAME),
            t("dialog.about.body").format(name=APP_NAME, version=APP_VERSION),
        )

    def _open_profiles_json(self) -> None:
        self._open_path(ensure_user_profiles_file())

    def _open_desktop(self) -> None:
        self._open_path(desktop_path())

    def _open_last_result(self) -> None:
        if self.last_result_path and self.last_result_path.exists():
            self._open_path(self.last_result_path)
        else:
            QMessageBox.warning(self, t("msg.no_result.title"), t("msg.no_result.body"))

    def _open_path(self, path: Path) -> None:
        try:
            if os.name == "nt":
                os.startfile(str(path))
            elif sys.platform == "darwin":
                subprocess.Popen(["open", str(path)])
            else:
                subprocess.Popen(["xdg-open", str(path)])
        except Exception as exc:
            QMessageBox.critical(
                self,
                t("msg.open_error.title"),
                t("msg.open_error.body").format(path=path, exc=exc),
            )


def run_app() -> int:
    app = QApplication(sys.argv)
    app.setApplicationName(APP_NAME)
    app.setApplicationVersion(APP_VERSION)
    app.setWindowIcon(QIcon(str(asset_path("ICO.ico"))))
    startup_config = Config.load()

    # Apply saved language before first render (no signal — window not yet created)
    lang = getattr(startup_config, "language", "ru")
    get_i18n().init_language(lang)

    startup_theme = startup_config.normalized_theme()
    if startup_theme == "system":
        color_scheme = app.styleHints().colorScheme()
        startup_theme = "dark" if color_scheme == Qt.ColorScheme.Dark else "light"
    qss_name = "app_dark.qss" if startup_theme == "dark" else "app_light.qss"
    qss = read_text_resource(style_path(qss_name)) or read_text_resource(style_path())
    if qss:
        app.setStyleSheet(qss)
    initial_zoom = startup_config.normalized_ui_zoom()
    if initial_zoom != 1.0:
        from PySide6.QtGui import QFont

        f = QFont(app.font())
        f.setPointSize(max(7, round(9 * initial_zoom)))
        app.setFont(f)
    window = MainWindow()
    window.show()
    return app.exec()
