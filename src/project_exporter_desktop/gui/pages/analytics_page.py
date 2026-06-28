from __future__ import annotations

from typing import Any

from PySide6.QtCore import Qt
from PySide6.QtGui import QColor, QPainter
from PySide6.QtWidgets import (
    QFrame,
    QGridLayout,
    QHBoxLayout,
    QLabel,
    QListWidget,
    QPushButton,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
    QWidget,
)

from . import make_card, make_scroll_page


class _LanguageChart(QFrame):
    def __init__(self) -> None:
        super().__init__()
        self._items: list[tuple[str, int]] = []
        self.setMinimumHeight(180)

    def set_items(self, items: list[tuple[str, int]]) -> None:
        self._items = items[:8]
        self.update()

    def paintEvent(self, _event) -> None:  # noqa: ANN001, N802
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        rect = self.rect().adjusted(8, 8, -8, -8)
        if not self._items:
            painter.setPen(QColor("#64748b"))
            painter.drawText(rect, Qt.AlignmentFlag.AlignCenter, "Нет данных по языкам")
            return
        max_value = max(value for _name, value in self._items) or 1
        row_h = max(20, rect.height() // max(len(self._items), 1))
        colors = [
            QColor("#2563eb"),
            QColor("#16a34a"),
            QColor("#d97706"),
            QColor("#dc2626"),
            QColor("#7c3aed"),
            QColor("#0891b2"),
            QColor("#4b5563"),
            QColor("#be123c"),
        ]
        for index, (name, value) in enumerate(self._items):
            y = rect.top() + index * row_h
            label_w = min(150, max(96, rect.width() // 4))
            bar_x = rect.left() + label_w + 10
            bar_w = max(4, int((rect.width() - label_w - 80) * value / max_value))
            painter.setPen(QColor("#334155"))
            painter.drawText(rect.left(), y, label_w, row_h, Qt.AlignmentFlag.AlignVCenter, name)
            painter.setBrush(colors[index % len(colors)])
            painter.setPen(Qt.PenStyle.NoPen)
            painter.drawRoundedRect(bar_x, y + 5, bar_w, max(8, row_h - 10), 4, 4)
            painter.setPen(QColor("#64748b"))
            painter.drawText(
                bar_x + bar_w + 8,
                y,
                70,
                row_h,
                Qt.AlignmentFlag.AlignVCenter,
                f"{value:,}",
            )


class AnalyticsPage(QWidget):
    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        scroll, layout = make_scroll_page(
            "Аналитика",
            "Локальная сводка по стеку, языкам, зависимостям, Git и рискам проекта.",
        )

        top_row = QHBoxLayout()
        self.refresh_button = QPushButton("Обновить аналитику")
        top_row.addStretch(1)
        top_row.addWidget(self.refresh_button)
        layout.addLayout(top_row)

        overview_card, overview_layout = make_card()
        self.summary_grid = QGridLayout()
        overview_layout.addLayout(self.summary_grid)
        layout.addWidget(overview_card)

        chart_card, chart_layout = make_card()
        chart_title = QLabel("Языки и LOC")
        chart_title.setObjectName("PageTitle")
        chart_layout.addWidget(chart_title)
        self.chart = _LanguageChart()
        chart_layout.addWidget(self.chart)
        layout.addWidget(chart_card)

        deps_card, deps_layout = make_card()
        deps_title = QLabel("Зависимости")
        deps_title.setObjectName("PageTitle")
        deps_layout.addWidget(deps_title)
        self.deps_table = QTableWidget(0, 4)
        self.deps_table.setHorizontalHeaderLabels(["Менеджер", "Пакет", "Версия", "Предупреждение"])
        self.deps_table.setSortingEnabled(True)
        deps_layout.addWidget(self.deps_table)
        layout.addWidget(deps_card)

        git_card, git_layout = make_card()
        git_title = QLabel("Git-активность")
        git_title.setObjectName("PageTitle")
        git_layout.addWidget(git_title)
        self.git_label = QLabel("Git-сводка ещё не построена.")
        self.git_label.setObjectName("PageHint")
        self.git_label.setWordWrap(True)
        git_layout.addWidget(self.git_label)
        self.git_list = QListWidget()
        git_layout.addWidget(self.git_list)
        layout.addWidget(git_card)

        risks_card, risks_layout = make_card()
        risks_title = QLabel("Риски")
        risks_title.setObjectName("PageTitle")
        risks_layout.addWidget(risks_title)
        self.risks_list = QListWidget()
        risks_layout.addWidget(self.risks_list)
        layout.addWidget(risks_card)
        layout.addStretch(1)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.addWidget(scroll)
        self.set_loading(False)

    def set_loading(self, loading: bool) -> None:
        self.refresh_button.setEnabled(not loading)
        if loading:
            self._set_summary({"Статус": "Сбор аналитики..."})

    def set_error(self, text: str) -> None:
        self.set_loading(False)
        self._set_summary({"Ошибка": text})

    def populate(self, report: Any) -> None:
        self.set_loading(False)
        self._set_summary(
            {
                "Проект": getattr(report, "project_name", ""),
                "Стек": getattr(report, "stack", ""),
                "Файлов": f"{getattr(report, 'total_files', 0):,}",
                "LOC": f"{getattr(report, 'total_loc', 0):,}",
                "Размер": getattr(report, "total_size_human", ""),
            }
        )
        languages = getattr(report, "languages", [])
        self.chart.set_items([(getattr(item, "name", ""), getattr(item, "loc", 0)) for item in languages])
        self._populate_deps(getattr(report, "dependencies", []))
        branch = getattr(report, "git_branch", "") or "нет данных"
        self.git_label.setText(f"Ветка: {branch}. {getattr(report, 'git_status', '')}")
        self.git_list.clear()
        for commit in getattr(report, "git_commits", []):
            self.git_list.addItem(
                f"{getattr(commit, 'short_hash', '')}  {getattr(commit, 'date', '')}  "
                f"{getattr(commit, 'author', '')}: {getattr(commit, 'subject', '')}"
            )
        if self.git_list.count() == 0:
            self.git_list.addItem("Коммиты не найдены.")
        self.risks_list.clear()
        for risk in getattr(report, "risks", []):
            self.risks_list.addItem(
                f"[{getattr(risk, 'severity', '')}] {getattr(risk, 'path', '')}: "
                f"{getattr(risk, 'reason', '')}"
            )
        if self.risks_list.count() == 0:
            self.risks_list.addItem("Явные риски не найдены.")

    def _set_summary(self, values: dict[str, str]) -> None:
        while self.summary_grid.count():
            item = self.summary_grid.takeAt(0)
            widget = item.widget()
            if widget is not None:
                widget.deleteLater()
        for col, (label, value) in enumerate(values.items()):
            name = QLabel(label)
            name.setObjectName("PageHint")
            number = QLabel(str(value))
            number.setObjectName("PageTitle")
            self.summary_grid.addWidget(name, 0, col)
            self.summary_grid.addWidget(number, 1, col)

    def _populate_deps(self, dependencies: list[Any]) -> None:
        self.deps_table.setSortingEnabled(False)
        self.deps_table.setRowCount(len(dependencies))
        for row, item in enumerate(dependencies):
            values = [
                getattr(item, "manager", ""),
                getattr(item, "name", ""),
                getattr(item, "version", ""),
                getattr(item, "warning", ""),
            ]
            for col, value in enumerate(values):
                self.deps_table.setItem(row, col, QTableWidgetItem(str(value)))
        self.deps_table.setSortingEnabled(True)
        self.deps_table.resizeColumnsToContents()
