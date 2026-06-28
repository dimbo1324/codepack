from __future__ import annotations

from typing import Any

from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QColor
from PySide6.QtWidgets import (
    QHBoxLayout,
    QHeaderView,
    QLabel,
    QPushButton,
    QTreeWidget,
    QTreeWidgetItem,
    QVBoxLayout,
    QWidget,
)

from ...utils.text_utils import format_bytes
from ...utils.token_counter import context_fit_rows, format_tokens
from . import make_card

_COLOR_INCLUDED = QColor("#4caf50")
_COLOR_EXCLUDED_HIGH = QColor("#f44336")
_COLOR_EXCLUDED_MEDIUM = QColor("#ff9800")
_COLOR_EXCLUDED_INFO = QColor("#9e9e9e")
_COLOR_OVERRIDE_INC = QColor("#00bcd4")
_COLOR_OVERRIDE_EXC = QColor("#e91e63")

_SEVERITY_COLOR = {
    "critical": _COLOR_EXCLUDED_HIGH,
    "high": _COLOR_EXCLUDED_HIGH,
    "medium": _COLOR_EXCLUDED_MEDIUM,
    "info": _COLOR_EXCLUDED_INFO,
    "low": _COLOR_EXCLUDED_INFO,
}

_COL_FILE = 0
_COL_SIZE = 1
_COL_STATUS = 2
_COL_REASON = 3


class PreviewPage(QWidget):

    export_confirmed = Signal(object)
    export_cancelled = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)

        self._overrides: dict[str, bool] = {}
        self._plan: Any = None

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(14)

        title = QLabel("Предпросмотр экспорта")
        title.setObjectName("PageTitle")
        outer.addWidget(title)

        hint = QLabel(
            "Просмотрите список файлов, которые войдут в экспорт. "
            "Двойной клик на строке — переключить решение вручную. "
            "Голубой = принудительно включён, розовый = принудительно исключён."
        )
        hint.setObjectName("PageHint")
        hint.setWordWrap(True)
        outer.addWidget(hint)

        stats_card, stats_layout = make_card()
        stats_layout.setSpacing(6)

        self._stats_label = QLabel("Ожидание плана экспорта...")
        self._stats_label.setObjectName("PageTitle")
        stats_layout.addWidget(self._stats_label)

        self._token_label = QLabel("")
        self._token_label.setObjectName("PageHint")
        self._token_label.setWordWrap(True)
        stats_layout.addWidget(self._token_label)

        self._model_rows: list[QLabel] = []
        model_frame = QHBoxLayout()
        model_frame.setSpacing(16)
        for _ in range(4):
            lbl = QLabel("")
            lbl.setObjectName("PageHint")
            self._model_rows.append(lbl)
            model_frame.addWidget(lbl)
        model_frame.addStretch(1)
        stats_layout.addLayout(model_frame)

        outer.addWidget(stats_card)

        tree_card, tree_layout = make_card()

        toolbar = QHBoxLayout()
        reset_btn = QPushButton("Сбросить переопределения")
        reset_btn.clicked.connect(self._reset_overrides)
        toolbar.addWidget(reset_btn)
        toolbar.addStretch(1)
        inc_legend = QLabel("■ Включён")
        inc_legend.setStyleSheet(f"color: {_COLOR_INCLUDED.name()}")
        toolbar.addWidget(inc_legend)
        exc_legend = QLabel("■ Исключён")
        exc_legend.setStyleSheet(f"color: {_COLOR_EXCLUDED_HIGH.name()}")
        toolbar.addWidget(exc_legend)
        tree_layout.addLayout(toolbar)

        self._tree = QTreeWidget()
        self._tree.setColumnCount(4)
        self._tree.setHeaderLabels(["Файл", "Размер", "Статус", "Причина"])
        self._tree.header().setSectionResizeMode(0, QHeaderView.ResizeMode.Stretch)
        self._tree.header().setSectionResizeMode(1, QHeaderView.ResizeMode.ResizeToContents)
        self._tree.header().setSectionResizeMode(2, QHeaderView.ResizeMode.ResizeToContents)
        self._tree.header().setSectionResizeMode(3, QHeaderView.ResizeMode.ResizeToContents)
        self._tree.setAlternatingRowColors(True)
        self._tree.setSortingEnabled(True)
        self._tree.setRootIsDecorated(False)
        self._tree.itemDoubleClicked.connect(self._on_item_double_clicked)
        tree_layout.addWidget(self._tree)

        outer.addWidget(tree_card, 1)

        btn_row = QHBoxLayout()
        btn_row.addStretch(1)
        cancel_btn = QPushButton("Назад к настройкам")
        cancel_btn.clicked.connect(self.export_cancelled.emit)
        btn_row.addWidget(cancel_btn)
        self._confirm_btn = QPushButton("Начать экспорт  →")
        self._confirm_btn.setObjectName("PrimaryButton")
        self._confirm_btn.setEnabled(False)
        self._confirm_btn.clicked.connect(self._on_confirm)
        btn_row.addWidget(self._confirm_btn)
        outer.addLayout(btn_row)


    def populate(self, plan: Any) -> None:
        self._plan = plan
        self._overrides.clear()
        self._tree.setSortingEnabled(False)
        self._tree.clear()

        included = getattr(plan, "included_files", [])
        excluded = getattr(plan, "excluded_files", [])

        for pf in included:
            item = self._make_item(pf, included=True)
            self._tree.addTopLevelItem(item)

        for pf in excluded:
            item = self._make_item(pf, included=False)
            self._tree.addTopLevelItem(item)

        self._tree.setSortingEnabled(True)
        self._tree.sortByColumn(2, Qt.SortOrder.AscendingOrder)

        self._refresh_stats()
        self._confirm_btn.setEnabled(True)

    def reset(self) -> None:
        self._plan = None
        self._overrides.clear()
        self._tree.clear()
        self._stats_label.setText("Строится план экспорта...")
        self._token_label.setText("")
        for lbl in self._model_rows:
            lbl.setText("")
        self._confirm_btn.setEnabled(False)

    def get_overrides(self) -> dict[str, bool]:
        return dict(self._overrides)


    def _make_item(self, pf: Any, *, included: bool) -> QTreeWidgetItem:
        rel = getattr(pf, "relative_path", "")
        size = getattr(pf, "size", 0)
        severity = getattr(pf, "severity", "info")
        reason = getattr(pf, "reason", "")

        status_text = "Включён" if included else "Исключён"
        item = QTreeWidgetItem([rel, format_bytes(size), status_text, reason])
        item.setData(0, Qt.ItemDataRole.UserRole, rel)
        item.setData(2, Qt.ItemDataRole.UserRole, included)

        color = _COLOR_INCLUDED if included else _SEVERITY_COLOR.get(severity, _COLOR_EXCLUDED_INFO)
        for col in range(4):
            item.setForeground(col, color)

        item.setTextAlignment(1, Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
        return item

    def _on_item_double_clicked(self, item: QTreeWidgetItem, _col: int) -> None:
        rel = item.data(0, Qt.ItemDataRole.UserRole)
        original_included: bool = item.data(2, Qt.ItemDataRole.UserRole)

        if rel in self._overrides:
            del self._overrides[rel]
            currently_included = original_included
            color = (
                _COLOR_INCLUDED
                if original_included
                else _SEVERITY_COLOR.get(
                    getattr(self._plan_file(rel), "severity", "info"), _COLOR_EXCLUDED_INFO
                )
            )
        else:
            new_decision = not original_included
            self._overrides[rel] = new_decision
            currently_included = new_decision
            color = _COLOR_OVERRIDE_INC if new_decision else _COLOR_OVERRIDE_EXC

        status_text = "Включён ✎" if currently_included else "Исключён ✎"
        if rel not in self._overrides:
            status_text = "Включён" if original_included else "Исключён"

        item.setText(2, status_text)
        for col in range(4):
            item.setForeground(col, color)

        self._refresh_stats()

    def _plan_file(self, rel: str) -> Any:
        if self._plan is None:
            return None
        for pf in getattr(self._plan, "included_files", []):
            if getattr(pf, "relative_path", "") == rel:
                return pf
        for pf in getattr(self._plan, "excluded_files", []):
            if getattr(pf, "relative_path", "") == rel:
                return pf
        return None

    def _reset_overrides(self) -> None:
        self._overrides.clear()
        if self._plan is not None:
            self.populate(self._plan)

    def _refresh_stats(self) -> None:
        if self._plan is None:
            return

        included_files = getattr(self._plan, "included_files", [])
        excluded_files = getattr(self._plan, "excluded_files", [])

        inc_set = {getattr(pf, "relative_path", "") for pf in included_files}
        exc_set = {getattr(pf, "relative_path", "") for pf in excluded_files}
        for rel, decision in self._overrides.items():
            if decision:
                inc_set.add(rel)
                exc_set.discard(rel)
            else:
                exc_set.add(rel)
                inc_set.discard(rel)

        all_files = {
            getattr(pf, "relative_path", ""): getattr(pf, "size", 0)
            for pf in included_files + excluded_files
        }
        inc_bytes = sum(all_files.get(r, 0) for r in inc_set)
        n_inc = len(inc_set)
        n_exc = len(exc_set)
        overrides_count = len(self._overrides)

        override_note = f"  │  Переопределений: {overrides_count}" if overrides_count else ""
        self._stats_label.setText(
            f"Включено: {n_inc:,} файл(ов)  │  ~{format_bytes(inc_bytes)}"
            f"  │  Исключено: {n_exc:,}{override_note}"
        )

        tokens = 0
        if inc_bytes > 0:
            from ...utils.token_counter import estimate_tokens
            tokens = estimate_tokens(inc_bytes)
            self._token_label.setText(
                f"Приблизительно токенов в экспорте: ~{format_tokens(tokens)}"
            )
        else:
            self._token_label.setText("")

        rows = context_fit_rows(inc_bytes)
        for i, lbl in enumerate(self._model_rows):
            if i < len(rows):
                name, tok, limit, fits = rows[i]
                pct = int(min(tok / limit * 100, 999)) if limit else 0
                icon = "✓" if fits else "✗"
                color = "#4caf50" if fits else "#f44336"
                lbl.setText(f"{icon} {name}")
                lbl.setStyleSheet(f"color: {color}")
                lbl.setToolTip(
                    f"{name}: {format_tokens(tok)} / {format_tokens(limit)} токенов ({pct}%)"
                )
            else:
                lbl.setText("")

    def _on_confirm(self) -> None:
        self.export_confirmed.emit(dict(self._overrides))