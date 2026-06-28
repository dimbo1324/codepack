# PySide6 wizard page module: owns one visible screen and emits user actions back to MainWindow.

from __future__ import annotations

from pathlib import Path
from typing import Any

from PySide6.QtCore import Qt, Signal
from PySide6.QtWidgets import (
    QDialog,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QListWidget,
    QPushButton,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
    QWidget,
)

from ...i18n import t
from . import make_card, make_scroll_page


class SnapshotCompareDialog(QDialog):
    def __init__(self, older: dict[str, Any], newer: dict[str, Any], parent: QWidget | None = None):
        super().__init__(parent)
        self.setWindowTitle(t("snapshot.title"))
        self.setMinimumSize(760, 560)
        layout = QVBoxLayout(self)
        title = QLabel(t("snapshot.subtitle"))
        title.setObjectName("PageTitle")
        layout.addWidget(title)
        summary = QLabel(self._summary_text(older, newer))
        summary.setObjectName("PageHint")
        summary.setWordWrap(True)
        layout.addWidget(summary)
        list_widget = QListWidget()
        for line in self._diff_lines(older, newer):
            list_widget.addItem(line)
        layout.addWidget(list_widget, 1)
        close = QPushButton(t("snapshot.close"))
        close.clicked.connect(self.accept)
        row = QHBoxLayout()
        row.addStretch(1)
        row.addWidget(close)
        layout.addLayout(row)

    def _snap(self, entry: dict[str, Any]) -> dict[str, dict[str, Any]]:
        snapshot = entry.get("snapshot")
        if not isinstance(snapshot, dict):
            return {}
        return {
            str(path): dict(meta)
            for path, meta in snapshot.items()
            if isinstance(path, str) and isinstance(meta, dict)
        }

    def _summary_text(self, older: dict[str, Any], newer: dict[str, Any]) -> str:
        old = self._snap(older)
        new = self._snap(newer)
        added = [path for path in new if path not in old]
        deleted = [path for path in old if path not in new]
        modified = [
            path
            for path, meta in new.items()
            if path in old and old[path].get("sha256") != meta.get("sha256")
        ]
        old_loc = sum(int(meta.get("loc", 0)) for meta in old.values())
        new_loc = sum(int(meta.get("loc", 0)) for meta in new.values())
        return t("snapshot.summary").format(
            added=len(added),
            modified=len(modified),
            deleted=len(deleted),
            loc=new_loc - old_loc,
        )

    def _diff_lines(self, older: dict[str, Any], newer: dict[str, Any]) -> list[str]:
        old = self._snap(older)
        new = self._snap(newer)
        lines: list[str] = []
        for path in sorted(path for path in new if path not in old):
            lines.append(f"+ {path}")
        for path in sorted(
            path
            for path, meta in new.items()
            if path in old and old[path].get("sha256") != meta.get("sha256")
        ):
            lines.append(f"~ {path}")
        for path in sorted(path for path in old if path not in new):
            lines.append(f"- {path}")
        return lines or [t("snapshot.no_diff")]


class HistoryPage(QWidget):
    open_result_requested = Signal(object)
    repeat_export_requested = Signal(object)

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._history: list[dict[str, Any]] = []
        scroll, layout, self._page_title, self._page_hint = make_scroll_page(
            t("history.page_title"),
            t("history.page_hint"),
        )

        card, card_layout = make_card()
        toolbar = QHBoxLayout()
        self.search_edit = QLineEdit()
        self.search_edit.setPlaceholderText(t("history.search_placeholder"))
        self.search_edit.textChanged.connect(self._apply_filter)
        self._refresh_btn = QPushButton(t("history.btn_refresh"))
        self._refresh_btn.clicked.connect(self._apply_filter)
        self.open_button = QPushButton(t("history.btn_open"))
        self.open_button.clicked.connect(self._open_selected)
        self.repeat_button = QPushButton(t("history.btn_repeat"))
        self.repeat_button.clicked.connect(self._repeat_selected)
        self.compare_button = QPushButton(t("history.btn_compare"))
        self.compare_button.clicked.connect(self._compare_selected)
        toolbar.addWidget(self.search_edit, 1)
        toolbar.addWidget(self._refresh_btn)
        toolbar.addWidget(self.open_button)
        toolbar.addWidget(self.repeat_button)
        toolbar.addWidget(self.compare_button)
        card_layout.addLayout(toolbar)

        self.table = QTableWidget(0, 7)
        self._update_table_headers()
        self.table.setSelectionBehavior(QTableWidget.SelectionBehavior.SelectRows)
        self.table.setSelectionMode(QTableWidget.SelectionMode.ExtendedSelection)
        self.table.setSortingEnabled(True)
        card_layout.addWidget(self.table, 1)
        layout.addWidget(card, 1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)

    def _update_table_headers(self) -> None:
        self.table.setHorizontalHeaderLabels(
            [
                t("history.col_date"),
                t("history.col_project"),
                t("history.col_profile"),
                t("history.col_files"),
                t("history.col_tokens"),
                t("history.col_status"),
                t("history.col_result"),
            ]
        )

    def retranslate(self) -> None:
        self._page_title.setText(t("history.page_title"))
        self._page_hint.setText(t("history.page_hint"))
        self.search_edit.setPlaceholderText(t("history.search_placeholder"))
        self._refresh_btn.setText(t("history.btn_refresh"))
        self.open_button.setText(t("history.btn_open"))
        self.repeat_button.setText(t("history.btn_repeat"))
        self.compare_button.setText(t("history.btn_compare"))
        self._update_table_headers()
        self._apply_filter()

    def set_history(self, history: list[dict[str, Any]]) -> None:
        self._history = history
        self._apply_filter()

    def _entry_text(self, entry: dict[str, Any]) -> str:
        return " ".join(
            str(entry.get(key, ""))
            for key in ["generated_at", "project_name", "source_root", "result", "profile"]
        ).casefold()

    def _apply_filter(self) -> None:
        query = self.search_edit.text().strip().casefold()
        rows = [entry for entry in self._history if not query or query in self._entry_text(entry)]
        self.table.setSortingEnabled(False)
        self.table.setRowCount(len(rows))
        for row, entry in enumerate(rows):
            self._set_row(row, entry)
        self.table.setSortingEnabled(True)
        self.table.resizeColumnsToContents()

    def _set_row(self, row: int, entry: dict[str, Any]) -> None:
        copy_stats = entry.get("copy_stats") if isinstance(entry.get("copy_stats"), dict) else {}
        files = copy_stats.get("files_copied", "")
        status = (
            t("history.status_cancelled") if entry.get("cancelled") else t("history.status_done")
        )
        values = [
            entry.get("generated_at", ""),
            entry.get("project_name", ""),
            entry.get("profile", ""),
            files,
            entry.get("tokens", ""),
            status,
            entry.get("result", ""),
        ]
        for col, value in enumerate(values):
            item = QTableWidgetItem(str(value))
            item.setData(Qt.ItemDataRole.UserRole, entry)
            self.table.setItem(row, col, item)

    def _selected_entries(self) -> list[dict[str, Any]]:
        entries: list[dict[str, Any]] = []
        for index in self.table.selectionModel().selectedRows():
            item = self.table.item(index.row(), 0)
            entry = item.data(Qt.ItemDataRole.UserRole) if item is not None else None
            if isinstance(entry, dict):
                entries.append(entry)
        return entries

    def _open_selected(self) -> None:
        entries = self._selected_entries()
        if entries:
            self.open_result_requested.emit(Path(str(entries[0].get("result", ""))))

    def _repeat_selected(self) -> None:
        entries = self._selected_entries()
        if entries:
            self.repeat_export_requested.emit(entries[0])

    def _compare_selected(self) -> None:
        entries = self._selected_entries()
        if len(entries) < 2:
            return
        older, newer = sorted(entries[:2], key=lambda item: str(item.get("generated_at", "")))
        SnapshotCompareDialog(older, newer, self).exec()
