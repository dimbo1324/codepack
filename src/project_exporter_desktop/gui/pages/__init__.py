from __future__ import annotations

from PySide6.QtWidgets import (
    QComboBox,
    QFrame,
    QHBoxLayout,
    QLabel,
    QScrollArea,
    QVBoxLayout,
    QWidget,
)


def make_scroll_page(title_text: str, hint_text: str) -> tuple[QScrollArea, QVBoxLayout]:
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


def make_card() -> tuple[QFrame, QVBoxLayout]:
    frame = QFrame()
    frame.setObjectName("Card")
    layout = QVBoxLayout(frame)
    layout.setContentsMargins(18, 18, 18, 18)
    layout.setSpacing(12)
    return frame, layout


def wrap_layout(layout: QHBoxLayout | QVBoxLayout) -> QWidget:
    widget = QWidget()
    widget.setLayout(layout)
    return widget


def set_combo_value(combo: QComboBox, value: str) -> None:
    index = combo.findText(value)
    if index >= 0:
        combo.setCurrentIndex(index)