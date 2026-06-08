from __future__ import annotations

import tkinter as tk
from tkinter import ttk

from .constants import APP_NAME, APP_VERSION
from .ui.app_window import App


def main() -> None:
    root = tk.Tk()

    try:
        style = ttk.Style()
        if "clam" in style.theme_names():
            style.theme_use("clam")
    except Exception:
        pass

    app = App(root)
    app._log(f"{APP_NAME} v{APP_VERSION} — готов к работе.")
    app._log("Выберите корневую папку проекта и нажмите «Создать экспорт».")
    root.mainloop()
