from __future__ import annotations

import os
import re
import subprocess
import sys
import threading
from dataclasses import replace
from datetime import datetime
from pathlib import Path

from PySide6.QtCore import Qt
from PySide6.QtGui import QAction, QIcon
from PySide6.QtWidgets import (
    QApplication,
    QCheckBox,
    QComboBox,
    QDialog,
    QFileDialog,
    QFormLayout,
    QFrame,
    QGridLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMainWindow,
    QMessageBox,
    QPlainTextEdit,
    QPushButton,
    QScrollArea,
    QSpinBox,
    QStackedWidget,
    QVBoxLayout,
    QWidget,
)

from ..config import Config
from ..constants import (
    APP_NAME,
    APP_VERSION,
    DIFF_EXPORT_MODES,
    EXPORT_PROFILES,
    MAX_ARCHIVE_PART_MB,
    SAFE_EXPORT_MODES,
    SETTINGS_FILE,
)
from ..services.export_history import load_export_history
from ..services.export_profiles import apply_custom_profile_if_needed, ensure_user_profiles_file, load_profile_catalog
from ..utils.path_utils import desktop_path, validate_source_root
from .dialogs import ExportPlanDialog, HistoryDialog, PromptGoalsDialog, RulesDialog
from .logging import append_app_log, app_log_file
from .resources import asset_path, read_text_resource, style_path
from .workers import ExportWorker, PlanPreviewWorker


EXPORTIGNORE_TEMPLATE = """# Project Exporter rules
# Similar to .gitignore. Use !pattern to explicitly include custom-ignored files.

node_modules/
.git/
dist/
build/
*.log
*.zip
*.rar
*.7z
*.db
*.sqlite
.env*
private/
large-assets/

# Examples:
# !README.md
# !docs/
"""


class MainWindow(QMainWindow):
    def __init__(self) -> None:
        super().__init__()
        self.config = Config.load()
        self.profile_catalog = load_profile_catalog()
        self.cancel_event = threading.Event()
        self.preview_worker: PlanPreviewWorker | None = None
        self.export_worker: ExportWorker | None = None
        self.pending_source_root: Path | None = None
        self.pending_config: Config | None = None
        self.last_result_path: Path | None = None
        self.log_file = app_log_file()
        self.nav_buttons: list[QPushButton] = []

        self.setWindowTitle(f"{APP_NAME} v{APP_VERSION}")
        self.resize(1280, 840)
        self.setMinimumSize(1120, 720)
        self._apply_icon()
        self._build_menu()
        self._build_ui()
        self._load_config_to_ui()
        self._append_log(f"{APP_NAME} v{APP_VERSION} is ready.")
        self._append_log("Select a project folder and create an export package.")

    # -- Construction -----------------------------------------------------

    def _apply_icon(self) -> None:
        icon = asset_path("ICO.ico")
        if icon.exists():
            self.setWindowIcon(QIcon(str(icon)))

    def _build_menu(self) -> None:
        file_menu = self.menuBar().addMenu("File")
        open_desktop = QAction("Open Desktop", self)
        open_desktop.triggered.connect(self._open_desktop)
        file_menu.addAction(open_desktop)
        open_result = QAction("Open last result", self)
        open_result.triggered.connect(self._open_last_result)
        file_menu.addAction(open_result)
        file_menu.addSeparator()
        exit_action = QAction("Exit", self)
        exit_action.triggered.connect(self.close)
        file_menu.addAction(exit_action)

        tools_menu = self.menuBar().addMenu("Tools")
        tools_menu.addAction("Edit rules", self._edit_rules)
        tools_menu.addAction("Prompt Builder", self._edit_prompt_goals)
        tools_menu.addAction("Create .exportignore template", self._create_exportignore_template)
        tools_menu.addSeparator()
        tools_menu.addAction("Export settings", self._export_settings)
        tools_menu.addAction("Import settings", self._import_settings)
        tools_menu.addAction("Reset settings", self._reset_settings)
        tools_menu.addSeparator()
        tools_menu.addAction("Recent exports", self._show_history)

    def _build_ui(self) -> None:
        central = QWidget()
        root = QHBoxLayout(central)
        root.setContentsMargins(0, 0, 0, 0)
        root.setSpacing(0)

        sidebar = self._build_sidebar()
        root.addWidget(sidebar)

        content_host = QWidget()
        content_layout = QVBoxLayout(content_host)
        content_layout.setContentsMargins(24, 22, 24, 22)
        content_layout.setSpacing(16)

        self.stack = QStackedWidget()
        self.page_project = self._build_project_page()
        self.page_settings = self._build_settings_page()
        self.page_security = self._build_security_page()
        self.page_run = self._build_run_page()
        self.page_summary = self._build_summary_page()
        for page in [
            self.page_project,
            self.page_settings,
            self.page_security,
            self.page_run,
            self.page_summary,
        ]:
            self.stack.addWidget(page)
        content_layout.addWidget(self.stack, 1)

        bottom = QHBoxLayout()
        self.status_label = QLabel("Ready")
        self.status_label.setObjectName("PageHint")
        bottom.addWidget(self.status_label, 1)
        self.open_result_button = QPushButton("Open result")
        self.open_result_button.setEnabled(False)
        self.open_result_button.clicked.connect(self._open_last_result)
        bottom.addWidget(self.open_result_button)
        self.cancel_button = QPushButton("Cancel")
        self.cancel_button.setObjectName("DangerButton")
        self.cancel_button.setEnabled(False)
        self.cancel_button.clicked.connect(self._cancel_export)
        bottom.addWidget(self.cancel_button)
        self.codex_button = QPushButton("Codex Package")
        self.codex_button.clicked.connect(lambda: self._start(codex_package=True))
        bottom.addWidget(self.codex_button)
        self.start_button = QPushButton("Create export")
        self.start_button.setObjectName("PrimaryButton")
        self.start_button.clicked.connect(lambda: self._start(codex_package=False))
        bottom.addWidget(self.start_button)
        content_layout.addLayout(bottom)

        root.addWidget(content_host, 1)
        self.setCentralWidget(central)

    def _build_sidebar(self) -> QFrame:
        sidebar = QFrame()
        sidebar.setObjectName("Sidebar")
        sidebar.setFixedWidth(250)
        layout = QVBoxLayout(sidebar)
        layout.setContentsMargins(18, 24, 18, 18)
        layout.setSpacing(10)

        title = QLabel("Project\nExporter")
        title.setObjectName("SidebarTitle")
        subtitle = QLabel("AI-ready safe handoff")
        subtitle.setObjectName("SidebarSubtitle")
        layout.addWidget(title)
        layout.addWidget(subtitle)
        layout.addSpacing(20)

        for index, text in enumerate([
            "1  Project",
            "2  Export settings",
            "3  Safety & filters",
            "4  Run log",
            "5  Result summary",
        ]):
            button = QPushButton(text)
            button.setObjectName("NavButton")
            button.setCursor(Qt.CursorShape.PointingHandCursor)
            button.clicked.connect(lambda _checked=False, i=index: self._set_page(i))
            self.nav_buttons.append(button)
            layout.addWidget(button)
        layout.addStretch(1)

        desktop_button = QPushButton("Open Desktop")
        desktop_button.setObjectName("NavButton")
        desktop_button.clicked.connect(self._open_desktop)
        layout.addWidget(desktop_button)
        self._set_page(0)
        return sidebar

    def _make_scroll_page(self, title_text: str, hint_text: str) -> tuple[QWidget, QVBoxLayout]:
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setFrameShape(QFrame.Shape.NoFrame)
        body = QWidget()
        layout = QVBoxLayout(body)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(14)
        title = QLabel(title_text)
        title.setObjectName("PageTitle")
        hint = QLabel(hint_text)
        hint.setObjectName("PageHint")
        hint.setWordWrap(True)
        layout.addWidget(title)
        layout.addWidget(hint)
        scroll.setWidget(body)
        return scroll, layout

    def _card(self) -> tuple[QFrame, QVBoxLayout]:
        frame = QFrame()
        frame.setObjectName("Card")
        layout = QVBoxLayout(frame)
        layout.setContentsMargins(18, 18, 18, 18)
        layout.setSpacing(12)
        return frame, layout

    def _build_project_page(self) -> QWidget:
        page, layout = self._make_scroll_page(
            "Select project",
            "Choose the source project folder. The original project is read-only during normal export.",
        )
        card, card_layout = self._card()
        form = QGridLayout()
        form.setColumnStretch(0, 1)
        self.root_edit = QLineEdit()
        self.root_edit.setPlaceholderText(r"C:\Users\you\Desktop\my-project")
        browse = QPushButton("Browse")
        browse.clicked.connect(self._browse_root)
        form.addWidget(QLabel("Project root"), 0, 0, 1, 2)
        form.addWidget(self.root_edit, 1, 0)
        form.addWidget(browse, 1, 1)
        card_layout.addLayout(form)

        self.project_hint = QLabel(
            "Safe defaults exclude .git, node_modules, virtual environments, caches, build artefacts and obvious secrets."
        )
        self.project_hint.setObjectName("PageHint")
        self.project_hint.setWordWrap(True)
        card_layout.addWidget(self.project_hint)
        layout.addWidget(card)
        layout.addStretch(1)
        return page

    def _build_settings_page(self) -> QWidget:
        page, layout = self._make_scroll_page(
            "Export settings",
            "Control profile, text dump limits, archive size and Git/incremental file selection.",
        )
        card, card_layout = self._card()
        form = QFormLayout()
        form.setLabelAlignment(Qt.AlignmentFlag.AlignLeft)

        self.profile_combo = QComboBox()
        self.profile_combo.addItems(list(self.profile_catalog.keys()))
        self.profile_combo.currentTextChanged.connect(self._sync_profile_hint)
        self.profile_hint = QLabel("")
        self.profile_hint.setObjectName("PageHint")
        profile_block = QVBoxLayout()
        profile_block.addWidget(self.profile_combo)
        profile_block.addWidget(self.profile_hint)
        form.addRow("Export profile", self._wrap(profile_block))

        self.text_limit_checkbox = QCheckBox("Limit one text file in text dump")
        self.text_limit_checkbox.toggled.connect(self._sync_text_limit_state)
        self.max_text_mb_spin = QSpinBox()
        self.max_text_mb_spin.setRange(1, 4096)
        self.max_text_mb_spin.setSuffix(" MB")
        text_limit_row = QHBoxLayout()
        text_limit_row.addWidget(self.text_limit_checkbox)
        text_limit_row.addWidget(self.max_text_mb_spin)
        text_limit_row.addStretch(1)
        form.addRow("Text dump", self._wrap(text_limit_row))

        self.zip_limit_spin = QSpinBox()
        self.zip_limit_spin.setRange(1, 102400)
        self.zip_limit_spin.setSuffix(" MB")
        form.addRow("ZIP part limit", self.zip_limit_spin)

        self.diff_combo = QComboBox()
        self.diff_combo.addItems(list(DIFF_EXPORT_MODES.keys()))
        self.diff_combo.currentTextChanged.connect(self._sync_diff_hint)
        self.diff_hint = QLabel("")
        self.diff_hint.setObjectName("PageHint")
        diff_block = QVBoxLayout()
        diff_block.addWidget(self.diff_combo)
        diff_block.addWidget(self.diff_hint)
        form.addRow("Git diff export", self._wrap(diff_block))

        refs_row = QHBoxLayout()
        self.diff_base_edit = QLineEdit()
        self.diff_base_edit.setPlaceholderText("HEAD")
        self.diff_target_edit = QLineEdit()
        self.diff_target_edit.setPlaceholderText("target ref")
        refs_row.addWidget(QLabel("Base"))
        refs_row.addWidget(self.diff_base_edit)
        refs_row.addWidget(QLabel("Target"))
        refs_row.addWidget(self.diff_target_edit)
        form.addRow("Refs", self._wrap(refs_row))

        self.incremental_checkbox = QCheckBox("Export only files added/modified since previous successful baseline")
        form.addRow("Incremental", self.incremental_checkbox)

        card_layout.addLayout(form)
        layout.addWidget(card)
        layout.addStretch(1)
        return page

    def _build_security_page(self) -> QWidget:
        page, layout = self._make_scroll_page(
            "Safety and filtering",
            "Configure safe export policy, redaction, Git patch inclusion and custom include/exclude rules.",
        )
        card, card_layout = self._card()
        form = QFormLayout()

        self.safe_mode_combo = QComboBox()
        self.safe_mode_combo.addItems(list(SAFE_EXPORT_MODES.keys()))
        self.safe_mode_combo.currentTextChanged.connect(self._sync_safe_hint)
        self.safe_hint = QLabel("")
        self.safe_hint.setObjectName("PageHint")
        safe_block = QVBoxLayout()
        safe_block.addWidget(self.safe_mode_combo)
        safe_block.addWidget(self.safe_hint)
        form.addRow("Safe Export mode", self._wrap(safe_block))

        self.redact_checkbox = QCheckBox("Redact obvious secrets in text and Git reports")
        self.git_patch_checkbox = QCheckBox("Include full Git patch; disabled by default because patches may contain secrets")
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
        for text, handler in [
            ("Edit include/exclude rules", self._edit_rules),
            ("Prompt Builder", self._edit_prompt_goals),
            ("Create .exportignore", self._create_exportignore_template),
            ("Profiles JSON", self._open_profiles_json),
        ]:
            button = QPushButton(text)
            button.clicked.connect(handler)
            actions.addWidget(button)
        actions.addStretch(1)
        card_layout.addLayout(actions)

        settings_actions = QHBoxLayout()
        for text, handler in [
            ("Export settings", self._export_settings),
            ("Import settings", self._import_settings),
            ("Reset settings", self._reset_settings),
            ("Recent exports", self._show_history),
        ]:
            button = QPushButton(text)
            button.clicked.connect(handler)
            settings_actions.addWidget(button)
        settings_actions.addStretch(1)
        card_layout.addLayout(settings_actions)

        layout.addWidget(card)
        layout.addStretch(1)
        return page

    def _build_run_page(self) -> QWidget:
        page, layout = self._make_scroll_page(
            "Run log",
            "The export runs in a worker thread. Progress, current stage and diagnostic messages appear here.",
        )
        card, card_layout = self._card()
        self.progress_bar = QProgressBarCompat()
        card_layout.addWidget(self.progress_bar)
        self.stage_label = QLabel("Idle")
        self.stage_label.setObjectName("PageHint")
        self.current_item_label = QLabel("")
        self.current_item_label.setObjectName("PageHint")
        self.current_item_label.setWordWrap(True)
        card_layout.addWidget(self.stage_label)
        card_layout.addWidget(self.current_item_label)
        self.log_view = QPlainTextEdit()
        self.log_view.setReadOnly(True)
        self.log_view.setMinimumHeight(420)
        card_layout.addWidget(self.log_view, 1)
        layout.addWidget(card, 1)
        return page

    def _build_summary_page(self) -> QWidget:
        page, layout = self._make_scroll_page(
            "Result summary",
            "After completion, this page shows the final status and where the archive was created.",
        )
        card, card_layout = self._card()
        self.summary_status = QLabel("No export has been created yet.")
        self.summary_status.setWordWrap(True)
        self.summary_path = QLineEdit()
        self.summary_path.setReadOnly(True)
        self.summary_path.setPlaceholderText("Result path will appear here")
        open_row = QHBoxLayout()
        open_button = QPushButton("Open result")
        open_button.clicked.connect(self._open_last_result)
        open_desktop = QPushButton("Open Desktop")
        open_desktop.clicked.connect(self._open_desktop)
        open_row.addWidget(open_button)
        open_row.addWidget(open_desktop)
        open_row.addStretch(1)
        card_layout.addWidget(self.summary_status)
        card_layout.addWidget(QLabel("Result path"))
        card_layout.addWidget(self.summary_path)
        card_layout.addLayout(open_row)
        layout.addWidget(card)
        layout.addStretch(1)
        return page

    @staticmethod
    def _wrap(layout: QHBoxLayout | QVBoxLayout) -> QWidget:
        widget = QWidget()
        widget.setLayout(layout)
        return widget

    # -- UI state ----------------------------------------------------------

    def _set_page(self, index: int) -> None:
        if hasattr(self, "stack"):
            self.stack.setCurrentIndex(index)
        for i, button in enumerate(self.nav_buttons):
            button.setProperty("active", i == index)
            button.style().unpolish(button)
            button.style().polish(button)

    def _sync_profile_hint(self) -> None:
        profile = self.profile_combo.currentText().strip()
        self.profile_hint.setText(self.profile_catalog.get(profile, EXPORT_PROFILES.get(profile, "")))

    def _sync_safe_hint(self) -> None:
        mode = self.safe_mode_combo.currentText().strip()
        self.safe_hint.setText(SAFE_EXPORT_MODES.get(mode, ""))

    def _sync_diff_hint(self) -> None:
        mode = self.diff_combo.currentText().strip()
        self.diff_hint.setText(DIFF_EXPORT_MODES.get(mode, ""))
        refs_enabled = mode in {"changed_since_ref", "between_refs"}
        target_enabled = mode == "between_refs"
        self.diff_base_edit.setEnabled(refs_enabled)
        self.diff_target_edit.setEnabled(target_enabled)

    def _sync_text_limit_state(self) -> None:
        self.max_text_mb_spin.setEnabled(self.text_limit_checkbox.isChecked())

    def _set_running(self, running: bool, preview: bool = False) -> None:
        self.start_button.setEnabled(not running)
        self.codex_button.setEnabled(not running)
        self.cancel_button.setEnabled(running and not preview)
        self.open_result_button.setEnabled(bool(self.last_result_path and self.last_result_path.exists()))
        self.status_label.setText("Building Export Plan..." if preview else ("Export is running..." if running else "Ready"))

    # -- Config ------------------------------------------------------------

    def _load_config_to_ui(self) -> None:
        self.root_edit.setText(self.config.last_root)
        self.text_limit_checkbox.setChecked(self.config.text_file_size_limit_enabled)
        self.max_text_mb_spin.setValue(max(1, int(self.config.max_text_file_mb)))
        self.zip_limit_spin.setValue(max(1, int(self.config.zip_part_limit_mb or MAX_ARCHIVE_PART_MB)))
        self.redact_checkbox.setChecked(self.config.redact_secrets)
        self.git_patch_checkbox.setChecked(self.config.include_git_patch)
        self.include_project_checkbox.setChecked(self.config.include_project_in_zip)
        self.keep_staging_checkbox.setChecked(self.config.keep_staging_folder)
        self.extra_ignored_edit.setText(", ".join(self.config.extra_ignored_dirs))
        self._set_combo_value(self.profile_combo, self.config.normalized_export_profile())
        self._set_combo_value(self.safe_mode_combo, self.config.normalized_safe_export_mode())
        self._set_combo_value(self.diff_combo, self.config.normalized_diff_export_mode())
        self.diff_base_edit.setText(self.config.diff_base_ref)
        self.diff_target_edit.setText(self.config.diff_target_ref)
        self.incremental_checkbox.setChecked(self.config.incremental_export_enabled)
        self._sync_profile_hint()
        self._sync_safe_hint()
        self._sync_diff_hint()
        self._sync_text_limit_state()

    @staticmethod
    def _set_combo_value(combo: QComboBox, value: str) -> None:
        index = combo.findText(value)
        if index >= 0:
            combo.setCurrentIndex(index)

    def _config_from_ui(self, save: bool = False) -> Config:
        cfg = self.config if save else replace(self.config)
        cfg.last_root = self.root_edit.text().strip() or str(Path.home())
        cfg.text_file_size_limit_enabled = self.text_limit_checkbox.isChecked()
        cfg.max_text_file_mb = max(1, self.max_text_mb_spin.value())
        cfg.zip_part_limit_mb = max(1, self.zip_limit_spin.value())
        cfg.redact_secrets = self.redact_checkbox.isChecked()
        cfg.include_git_patch = self.git_patch_checkbox.isChecked()
        cfg.include_project_in_zip = self.include_project_checkbox.isChecked()
        cfg.keep_staging_folder = self.keep_staging_checkbox.isChecked()
        cfg.export_profile = self.profile_combo.currentText().strip()
        cfg.safe_export_mode = self.safe_mode_combo.currentText().strip()
        cfg.diff_export_mode = self.diff_combo.currentText().strip()
        cfg.diff_base_ref = self.diff_base_edit.text().strip() or "HEAD"
        cfg.diff_target_ref = self.diff_target_edit.text().strip()
        cfg.incremental_export_enabled = self.incremental_checkbox.isChecked()

        extras: list[str] = []
        for token in re.split(r"[,;\n]", self.extra_ignored_edit.text()):
            value = token.strip()
            if value and value not in extras:
                extras.append(value)
        cfg.extra_ignored_dirs = extras

        selected_profile_key = cfg.export_profile
        cfg = apply_custom_profile_if_needed(selected_profile_key, cfg)
        if cfg.safe_export_mode not in SAFE_EXPORT_MODES:
            cfg.safe_export_mode = "safe"
        if cfg.diff_export_mode not in DIFF_EXPORT_MODES:
            cfg.diff_export_mode = "all"
        return cfg

    def _save_config_from_ui(self) -> None:
        self.config = self._config_from_ui(save=True)
        self.config.save()

    def _codex_config(self, base: Config) -> Config:
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

    # -- Actions -----------------------------------------------------------

    def _browse_root(self) -> None:
        initial = self.root_edit.text().strip() or str(Path.home())
        selected = QFileDialog.getExistingDirectory(self, "Select project root", initial)
        if selected:
            self.root_edit.setText(selected)

    def _validate_source_root(self) -> Path | None:
        try:
            return validate_source_root(self.root_edit.text())
        except Exception as exc:
            QMessageBox.critical(self, "Invalid project root", str(exc))
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
        self._set_page(3)
        self._append_log(
            f"Building Export Plan... profile={run_config.normalized_export_profile()}, "
            f"safe={run_config.normalized_safe_export_mode()}, diff={run_config.normalized_diff_export_mode()}"
        )
        self._set_running(True, preview=True)
        self.preview_worker = PlanPreviewWorker(source_root, run_config, self)
        self.preview_worker.finished_preview.connect(self._on_preview_ready)
        self.preview_worker.failed.connect(self._on_preview_failed)
        self.preview_worker.start()

    def _on_preview_ready(self, preview_text: str) -> None:
        self._set_running(False)
        dialog = ExportPlanDialog(preview_text, self)
        if dialog.exec() != QDialog.DialogCode.Accepted:
            self._append_log("Export cancelled at Export Plan confirmation stage.")
            return
        if self.pending_source_root is None or self.pending_config is None:
            QMessageBox.critical(self, "Export error", "Internal state was lost before export start.")
            return
        self._run_export(self.pending_source_root, self.pending_config)

    def _on_preview_failed(self, traceback_text: str) -> None:
        self._set_running(False)
        self._append_diagnostic(traceback_text)
        QMessageBox.critical(
            self,
            "Export Plan failed",
            f"Could not build the Export Plan. Technical details were written to:\n{self.log_file}",
        )

    def _run_export(self, source_root: Path, config: Config) -> None:
        self.cancel_event.clear()
        self.last_result_path = None
        self.open_result_button.setEnabled(False)
        self.summary_status.setText("Export is running...")
        self.summary_path.clear()
        self.progress_bar.setValue(0)
        self.stage_label.setText("Starting export...")
        self.current_item_label.clear()
        self._set_page(3)
        self._set_running(True)
        self._append_log("Starting export worker...")
        self.export_worker = ExportWorker(source_root, config, self.cancel_event, self)
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
            "Cancel export",
            "Stop the current export operation? A partial result may remain if enough data was already produced.",
        )
        if reply == QMessageBox.StandardButton.Yes:
            self.cancel_event.set()
            self._append_log("Cancellation requested...")

    def _on_export_finished(self, result: object) -> None:
        data = result if isinstance(result, dict) else {}
        result_path = data.get("result_path")
        if isinstance(result_path, Path):
            self.last_result_path = result_path
        cancelled = bool(data.get("cancelled"))
        self._set_running(False)
        self.open_result_button.setEnabled(bool(self.last_result_path and self.last_result_path.exists()))
        self._set_page(4)
        if cancelled:
            self.summary_status.setText("Export stopped by user. Partial output may have been created.")
            QMessageBox.warning(self, "Stopped", "Export was stopped. Review the result and run log.")
        else:
            self.summary_status.setText("Export completed successfully.")
            QMessageBox.information(self, "Export complete", "Project export was created successfully.")
        if self.last_result_path:
            self.summary_path.setText(str(self.last_result_path))
        self.status_label.setText("Ready")

    def _on_export_failed(self, traceback_text: str) -> None:
        self._set_running(False)
        self._append_diagnostic(traceback_text)
        self._set_page(4)
        self.summary_status.setText(f"Export failed. Technical details were written to {self.log_file}.")
        QMessageBox.critical(self, "Export failed", f"Export failed. Technical details were written to:\n{self.log_file}")

    def _handle_progress(self, percent: int, stage: str, current: str) -> None:
        self.progress_bar.setValue(max(0, min(100, int(percent))))
        self.stage_label.setText(f"{percent}% — {stage}")
        self.current_item_label.setText(current)

    def _append_log(self, message: str) -> None:
        append_app_log(message)
        timestamp = datetime.now().strftime("%H:%M:%S")
        self.log_view.appendPlainText(f"[{timestamp}] {message}")
        self.log_view.verticalScrollBar().setValue(self.log_view.verticalScrollBar().maximum())

    def _append_diagnostic(self, traceback_text: str) -> None:
        append_app_log("Technical traceback follows:")
        append_app_log(traceback_text)
        self._append_log(f"Technical error details were written to: {self.log_file}")

    # -- Tools -------------------------------------------------------------

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
        self._append_log("Custom include/exclude rules saved.")

    def _edit_prompt_goals(self) -> None:
        dialog = PromptGoalsDialog(self.config.prompt_goals, self)
        if dialog.exec() != QDialog.DialogCode.Accepted:
            return
        self.config.prompt_goals = dialog.goals()
        self.config.save()
        self._append_log("Prompt Builder goals saved.")

    def _create_exportignore_template(self) -> None:
        source_root = self._validate_source_root()
        if source_root is None:
            return
        target = source_root / ".exportignore"
        if target.exists():
            reply = QMessageBox.question(
                self,
                ".exportignore exists",
                ".exportignore already exists. Overwrite it with the template?",
            )
            if reply != QMessageBox.StandardButton.Yes:
                return
        try:
            target.write_text(EXPORTIGNORE_TEMPLATE, encoding="utf-8", newline="\n")
            self._append_log(f"Created .exportignore: {target}")
            self._open_path(target)
        except Exception as exc:
            QMessageBox.critical(self, "Write error", f"Could not create .exportignore:\n{exc}")

    def _export_settings(self) -> None:
        self._save_config_from_ui()
        target, _ = QFileDialog.getSaveFileName(
            self,
            "Export settings",
            "project_exporter_settings.json",
            "JSON files (*.json)",
        )
        if not target:
            return
        try:
            Config.export_settings(Path(target), self.config)
            self._append_log(f"Settings exported: {target}")
        except Exception as exc:
            QMessageBox.critical(self, "Export settings failed", str(exc))

    def _import_settings(self) -> None:
        source, _ = QFileDialog.getOpenFileName(self, "Import settings", "", "JSON files (*.json)")
        if not source:
            return
        try:
            self.config = Config.import_settings(Path(source))
            self.config.save()
            self._load_config_to_ui()
            self._append_log(f"Settings imported: {source}")
        except Exception as exc:
            QMessageBox.critical(self, "Import settings failed", str(exc))

    def _reset_settings(self) -> None:
        reply = QMessageBox.question(self, "Reset settings", "Reset saved settings to safe defaults?")
        if reply != QMessageBox.StandardButton.Yes:
            return
        try:
            SETTINGS_FILE.unlink(missing_ok=True)
        except Exception:
            pass
        self.config = Config()
        self._load_config_to_ui()
        self._append_log("Settings reset to defaults.")

    def _show_history(self) -> None:
        HistoryDialog(load_export_history(), self).exec()

    def _open_profiles_json(self) -> None:
        self._open_path(ensure_user_profiles_file())

    def _open_desktop(self) -> None:
        self._open_path(desktop_path())

    def _open_last_result(self) -> None:
        if self.last_result_path and self.last_result_path.exists():
            self._open_path(self.last_result_path)
        else:
            QMessageBox.warning(self, "No result", "No generated result exists yet.")

    def _open_path(self, path: Path) -> None:
        try:
            if os.name == "nt":
                os.startfile(str(path))  # type: ignore[attr-defined]
            elif sys.platform == "darwin":
                subprocess.Popen(["open", str(path)])
            else:
                subprocess.Popen(["xdg-open", str(path)])
        except Exception as exc:
            QMessageBox.critical(self, "Open path failed", f"Could not open:\n{path}\n\n{exc}")


class QProgressBarCompat(QWidget):
    """Small wrapper around QProgressBar to keep construction imports compact."""

    def __init__(self) -> None:
        from PySide6.QtWidgets import QProgressBar

        super().__init__()
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        self.bar = QProgressBar()
        self.bar.setRange(0, 100)
        self.bar.setTextVisible(True)
        layout.addWidget(self.bar)

    def setValue(self, value: int) -> None:  # noqa: N802 - Qt-compatible name
        self.bar.setValue(value)


def run_app() -> int:
    app = QApplication(sys.argv)
    app.setApplicationName(APP_NAME)
    app.setApplicationVersion(APP_VERSION)
    app.setWindowIcon(QIcon(str(asset_path("ICO.ico"))))
    qss = read_text_resource(style_path())
    if qss:
        app.setStyleSheet(qss)
    window = MainWindow()
    window.show()
    return app.exec()

