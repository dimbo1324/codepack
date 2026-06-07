from __future__ import annotations

import os
import re
import subprocess
import sys
import threading
import traceback
from datetime import datetime
from pathlib import Path
from queue import Empty, Queue

import tkinter as tk
from tkinter import filedialog, messagebox, scrolledtext, ttk

from ..config import Config
from ..constants import APP_NAME, APP_VERSION, SETTINGS_FILE
from ..services.exporter import ProjectExporter
from ..utils.path_utils import desktop_path, validate_source_root

class App:
    def __init__(self, master: tk.Tk):
        self.master = master
        self.config = Config.load()
        self.log_queue: Queue[str] = Queue()
        self.cancel_event = threading.Event()
        self.worker: threading.Thread | None = None
        self.last_result_path: Path | None = None  # zip or staging dir

        self._build_ui()
        self._load_config_to_ui()
        self._poll_logs()

    # -- UI construction ----------------------------------------------------

    def _build_ui(self) -> None:
        self.master.title(f"{APP_NAME} v{APP_VERSION}")
        self.master.geometry("960x760")
        self.master.minsize(880, 640)

        root = ttk.Frame(self.master, padding=14)
        root.pack(fill="both", expand=True)

        title = ttk.Label(root, text=APP_NAME, font=("Segoe UI", 16, "bold"))
        title.grid(row=0, column=0, columnspan=3, sticky="w")

        subtitle = ttk.Label(
            root,
            text=(
                "Один ZIP с копией проекта, manifest.json, INDEX.md "
                "и пакетом отчётов (структура, Git, текстовый дамп, аналитика)."
            ),
            foreground="gray",
        )
        subtitle.grid(row=1, column=0, columnspan=3, sticky="w", pady=(2, 14))

        ttk.Label(root, text="Корневая папка проекта:").grid(
            row=2, column=0, columnspan=3, sticky="w"
        )

        self.entry_root = ttk.Entry(root)
        self.entry_root.grid(row=3, column=0, sticky="we", padx=(0, 8))

        ttk.Button(root, text="Обзор", command=self._browse_root).grid(
            row=3, column=1, sticky="e"
        )
        ttk.Button(root, text="Открыть Desktop", command=self._open_desktop).grid(
            row=3, column=2, sticky="e", padx=(8, 0)
        )

        options = ttk.LabelFrame(root, text="Настройки", padding=10)
        options.grid(row=4, column=0, columnspan=3, sticky="we", pady=(14, 10))

        # Row: max text file size
        size_line = ttk.Frame(options)
        size_line.pack(anchor="w", fill="x")
        ttk.Label(size_line, text="Максимальный размер одного текстового файла:").pack(
            side="left"
        )
        self.var_max_mb = tk.StringVar()
        self.entry_max_mb = ttk.Entry(
            size_line, width=8, textvariable=self.var_max_mb, justify="right"
        )
        self.entry_max_mb.pack(side="left", padx=(8, 4))
        ttk.Label(size_line, text="МБ").pack(side="left")

        # Row: redact secrets
        self.var_redact = tk.BooleanVar(value=True)
        ttk.Checkbutton(
            options,
            text=(
                "Маскировать очевидные секреты в текстовом дампе "
                "(.env, TOKEN, PASSWORD, API_KEY и т.п.)"
            ),
            variable=self.var_redact,
        ).pack(anchor="w", pady=(8, 0))

        # Row: include project in zip
        self.var_include_project = tk.BooleanVar(value=True)
        ttk.Checkbutton(
            options,
            text="Включать копию проекта внутрь итогового ZIP (иначе — только отчёты)",
            variable=self.var_include_project,
        ).pack(anchor="w", pady=(4, 0))

        # Row: keep staging
        self.var_keep_staging = tk.BooleanVar(value=False)
        ttk.Checkbutton(
            options,
            text=(
                "Оставить распакованную папку рядом с ZIP "
                "(удобно для просмотра без распаковки)"
            ),
            variable=self.var_keep_staging,
        ).pack(anchor="w", pady=(4, 0))

        # Row: extra ignored dirs
        extras_line = ttk.Frame(options)
        extras_line.pack(anchor="w", fill="x", pady=(8, 0))
        ttk.Label(
            extras_line,
            text="Дополнительно исключить папки (через запятую):",
        ).pack(side="left")
        self.var_extra_ignored = tk.StringVar()
        self.entry_extra_ignored = ttk.Entry(
            extras_line, textvariable=self.var_extra_ignored
        )
        self.entry_extra_ignored.pack(side="left", fill="x", expand=True, padx=(8, 0))

        ttk.Label(
            options,
            text=(
                "Базовое исключение .git и node_modules сохраняется всегда — "
                "ваши значения только добавляются."
            ),
            foreground="gray",
        ).pack(anchor="w", pady=(4, 0))

        warning = ttk.Label(
            options,
            text=(
                "Важно: Git-команды read-only — не переключают ветку "
                "и не изменяют исходный проект."
            ),
            foreground="gray",
        )
        warning.pack(anchor="w", pady=(8, 0))

        # Action buttons
        actions = ttk.Frame(root)
        actions.grid(row=5, column=0, columnspan=3, sticky="we", pady=(8, 10))

        self.btn_start = ttk.Button(
            actions, text="▶ Создать экспорт", command=self._start
        )
        self.btn_start.pack(side="left")

        self.btn_cancel = ttk.Button(
            actions, text="Отмена", command=self._cancel, state="disabled"
        )
        self.btn_cancel.pack(side="left", padx=8)

        self.btn_open_result = ttk.Button(
            actions,
            text="Открыть результат",
            command=self._open_last_result,
            state="disabled",
        )
        self.btn_open_result.pack(side="left")

        ttk.Button(actions, text="Сброс настроек", command=self._reset_settings).pack(
            side="right"
        )

        # Progress + status
        progress_line = ttk.Frame(root)
        progress_line.grid(row=6, column=0, columnspan=3, sticky="we", pady=(4, 8))

        self.progress = ttk.Progressbar(progress_line, mode="indeterminate")
        self.progress.pack(side="left", fill="x", expand=True)

        self.lbl_status = ttk.Label(progress_line, text="Готов", width=18)
        self.lbl_status.pack(side="left", padx=(10, 0))

        # Log
        self.log = scrolledtext.ScrolledText(
            root, height=22, state="disabled", wrap="word", font=("Consolas", 9)
        )
        self.log.grid(row=7, column=0, columnspan=3, sticky="nsew")

        footer = ttk.Label(
            root,
            text=(
                "Результат создаётся на Desktop. По умолчанию это один файл "
                "вида {project}_export_{timestamp}.zip."
            ),
            foreground="gray",
        )
        footer.grid(row=8, column=0, columnspan=3, sticky="w", pady=(8, 0))

        root.columnconfigure(0, weight=1)
        root.rowconfigure(7, weight=1)

    # -- Config sync --------------------------------------------------------

    def _load_config_to_ui(self) -> None:
        self.entry_root.delete(0, "end")
        self.entry_root.insert(0, self.config.last_root)

        self.var_max_mb.set(str(self.config.max_text_file_mb))
        self.var_redact.set(self.config.redact_secrets)
        self.var_include_project.set(self.config.include_project_in_zip)
        self.var_keep_staging.set(self.config.keep_staging_folder)
        self.var_extra_ignored.set(", ".join(self.config.extra_ignored_dirs))

    def _save_config_from_ui(self) -> None:
        self.config.last_root = self.entry_root.get().strip() or str(Path.home())
        try:
            self.config.max_text_file_mb = max(1, int(self.var_max_mb.get().strip()))
        except Exception:
            self.config.max_text_file_mb = 5
            self.var_max_mb.set("5")

        self.config.redact_secrets = bool(self.var_redact.get())
        self.config.include_project_in_zip = bool(self.var_include_project.get())
        self.config.keep_staging_folder = bool(self.var_keep_staging.get())

        raw_extras = self.var_extra_ignored.get()
        extras: list[str] = []
        for token in re.split(r"[,;\n]", raw_extras):
            token = token.strip()
            if token and token not in extras:
                extras.append(token)
        self.config.extra_ignored_dirs = extras

        self.config.save()

    # -- Buttons ------------------------------------------------------------

    def _browse_root(self) -> None:
        initial = self.entry_root.get().strip() or str(Path.home())
        selected = filedialog.askdirectory(
            initialdir=initial, title="Выберите корневую папку проекта"
        )
        if selected:
            self.entry_root.delete(0, "end")
            self.entry_root.insert(0, selected)

    def _start(self) -> None:
        if self.worker and self.worker.is_alive():
            return

        try:
            source_root = validate_source_root(self.entry_root.get())
        except Exception as exc:
            messagebox.showerror("Ошибка", str(exc))
            return

        try:
            max(1, int(self.var_max_mb.get().strip()))
        except Exception:
            messagebox.showerror(
                "Ошибка", "Максимальный размер файла должен быть целым числом."
            )
            return

        self._save_config_from_ui()
        self.cancel_event.clear()
        self.last_result_path = None
        self.btn_open_result.config(state="disabled")
        self._set_running(True)
        self._log("Запуск экспорта...")

        exporter = ProjectExporter(
            source_root=source_root,
            config=self.config,
            log_queue=self.log_queue,
            cancel_event=self.cancel_event,
        )

        def target() -> None:
            try:
                paths = exporter.run()
                # Prefer the zip as the "result"; fall back to staging if kept.
                if paths.final_zip.exists():
                    self.last_result_path = paths.final_zip
                elif paths.staging_dir.exists():
                    self.last_result_path = paths.staging_dir

                if self.cancel_event.is_set():
                    location = self.last_result_path or paths.staging_dir
                    self.master.after(
                        0,
                        lambda: messagebox.showwarning(
                            "Остановлено",
                            f"Операция остановлена.\n{location}",
                        ),
                    )
                else:
                    self.master.after(
                        0, lambda: self.btn_open_result.config(state="normal")
                    )
                    location = self.last_result_path or paths.final_zip
                    self.master.after(
                        0,
                        lambda: messagebox.showinfo(
                            "Готово", f"Экспорт создан:\n{location}"
                        ),
                    )
            except Exception:
                error_text = traceback.format_exc()
                self.log_queue.put(error_text)
                self.master.after(0, lambda: messagebox.showerror("Ошибка", error_text))
            finally:
                self.master.after(0, lambda: self._set_running(False))

        self.worker = threading.Thread(target=target, daemon=True)
        self.worker.start()

    def _cancel(self) -> None:
        if self.worker and self.worker.is_alive():
            if messagebox.askyesno("Отмена", "Остановить текущую операцию?"):
                self.cancel_event.set()
                self._log("Запрошена остановка...")

    def _set_running(self, running: bool) -> None:
        if running:
            self.btn_start.config(state="disabled")
            self.btn_cancel.config(state="normal")
            self.lbl_status.config(text="Выполняется...")
            self.progress.start(15)
        else:
            self.btn_start.config(state="normal")
            self.btn_cancel.config(state="disabled")
            self.lbl_status.config(text="Готов")
            self.progress.stop()

    def _open_path(self, path: Path) -> None:
        try:
            if os.name == "nt":
                # On Windows, startfile opens files in their default app
                # and folders in Explorer.
                os.startfile(str(path))  # type: ignore[attr-defined]
            elif sys.platform == "darwin":
                # For files: open them; for folders: reveal in Finder.
                # `open` handles both.
                subprocess.Popen(["open", str(path)])
            else:
                # Linux: xdg-open handles both files and directories.
                subprocess.Popen(["xdg-open", str(path)])
        except Exception as exc:
            messagebox.showerror("Ошибка", f"Не удалось открыть путь:\n{path}\n\n{exc}")

    def _open_desktop(self) -> None:
        self._open_path(desktop_path())

    def _open_last_result(self) -> None:
        if self.last_result_path and self.last_result_path.exists():
            # Selecting the zip in Explorer is nicer than opening it directly,
            # but `os.startfile` on a .zip will open it in the archive viewer
            # which is also fine. We keep behaviour simple and consistent.
            self._open_path(self.last_result_path)
        else:
            messagebox.showwarning("Нет результата", "Итоговый файл пока не создан.")

    def _reset_settings(self) -> None:
        if not messagebox.askyesno("Сброс", "Сбросить сохранённые настройки?"):
            return

        try:
            SETTINGS_FILE.unlink(missing_ok=True)
        except Exception:
            pass

        self.config = Config()
        self._load_config_to_ui()
        self._log("Настройки сброшены.")

    def _log(self, message: str) -> None:
        self.log_queue.put(message)

    def _poll_logs(self) -> None:
        try:
            while True:
                message = self.log_queue.get_nowait()
                timestamp = datetime.now().strftime("%H:%M:%S")
                self.log.configure(state="normal")
                self.log.insert("end", f"[{timestamp}] {message}\n")
                self.log.see("end")
                self.log.configure(state="disabled")
        except Empty:
            pass

        self.master.after(150, self._poll_logs)
