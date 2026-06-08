from __future__ import annotations

import os
import re
import subprocess
import sys
import threading
import traceback
from dataclasses import replace
from datetime import datetime
from pathlib import Path
from queue import Empty, Queue

import tkinter as tk
from tkinter import filedialog, messagebox, scrolledtext, ttk

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
from ..services.exporter import ProjectExporter
from ..services.git_diff import resolve_diff_selection
from ..services.risk_preview import build_pre_export_risk_preview, format_risk_preview_for_user
from ..utils.path_utils import desktop_path, validate_source_root


class App:
    def __init__(self, master: tk.Tk):
        self.master = master
        self.config = Config.load()
        self.profile_catalog = load_profile_catalog()
        self.log_queue: Queue[str] = Queue()
        self.cancel_event = threading.Event()
        self.worker: threading.Thread | None = None
        self.last_result_path: Path | None = None

        self._build_ui()
        self._load_config_to_ui()
        self._poll_logs()

    # -- UI construction ----------------------------------------------------

    def _try_set_window_icon(self) -> None:
        try:
            icon_path = Path(__file__).resolve().parents[3] / "assets" / "ICO.ico"
            if icon_path.exists():
                self.master.iconbitmap(str(icon_path))
        except Exception:
            pass

    def _sync_profile_hint(self) -> None:
        profile = self.var_export_profile.get().strip()
        self.lbl_profile_hint.config(text=self.profile_catalog.get(profile, EXPORT_PROFILES.get(profile, "")))

    def _sync_safe_hint(self) -> None:
        mode = self.var_safe_mode.get().strip()
        self.lbl_safe_hint.config(text=SAFE_EXPORT_MODES.get(mode, ""))

    def _sync_diff_hint(self) -> None:
        mode = self.var_diff_mode.get().strip()
        self.lbl_diff_hint.config(text=DIFF_EXPORT_MODES.get(mode, ""))
        enabled = mode in {"changed_since_ref", "between_refs"}
        target_enabled = mode == "between_refs"
        self.entry_diff_base.configure(state="normal" if enabled else "disabled")
        self.entry_diff_target.configure(state="normal" if target_enabled else "disabled")

    def _sync_text_limit_state(self) -> None:
        self.entry_max_mb.configure(state="normal" if self.var_text_limit_enabled.get() else "disabled")

    def _build_ui(self) -> None:
        self.master.title(f"{APP_NAME} v{APP_VERSION}")
        self.master.geometry("1080x900")
        self.master.minsize(980, 760)
        self._try_set_window_icon()

        root = ttk.Frame(self.master, padding=14)
        root.pack(fill="both", expand=True)

        ttk.Label(root, text=APP_NAME, font=("Segoe UI", 16, "bold")).grid(
            row=0, column=0, columnspan=3, sticky="w"
        )
        ttk.Label(
            root,
            text=(
                "Безопасный экспорт проекта для ChatGPT/Codex: копия проекта, отчёты, "
                "AI_CONTEXT, AI_PROMPTS, health score и логическое разбиение ZIP > 512 МБ."
            ),
            foreground="gray",
        ).grid(row=1, column=0, columnspan=3, sticky="w", pady=(2, 14))

        ttk.Label(root, text="Корневая папка проекта:").grid(row=2, column=0, columnspan=3, sticky="w")
        self.entry_root = ttk.Entry(root)
        self.entry_root.grid(row=3, column=0, sticky="we", padx=(0, 8))
        ttk.Button(root, text="Обзор", command=self._browse_root).grid(row=3, column=1, sticky="e")
        ttk.Button(root, text="Открыть Desktop", command=self._open_desktop).grid(row=3, column=2, sticky="e", padx=(8, 0))

        options = ttk.LabelFrame(root, text="Настройки экспорта", padding=10)
        options.grid(row=4, column=0, columnspan=3, sticky="we", pady=(14, 10))

        # Profile row
        profile_line = ttk.Frame(options)
        profile_line.pack(anchor="w", fill="x")
        ttk.Label(profile_line, text="Профиль экспорта:").pack(side="left")
        self.var_export_profile = tk.StringVar()
        self.combo_export_profile = ttk.Combobox(
            profile_line,
            textvariable=self.var_export_profile,
            values=list(self.profile_catalog.keys()),
            state="readonly",
            width=18,
        )
        self.combo_export_profile.pack(side="left", padx=(8, 10))
        self.lbl_profile_hint = ttk.Label(profile_line, text="", foreground="gray")
        self.lbl_profile_hint.pack(side="left", fill="x", expand=True)
        self.combo_export_profile.bind("<<ComboboxSelected>>", lambda _event: self._sync_profile_hint())
        ttk.Button(profile_line, text="Profiles JSON", command=self._open_profiles_json).pack(side="right")

        # Safe export row
        safe_line = ttk.Frame(options)
        safe_line.pack(anchor="w", fill="x", pady=(8, 0))
        ttk.Label(safe_line, text="Safe Export mode:").pack(side="left")
        self.var_safe_mode = tk.StringVar()
        self.combo_safe_mode = ttk.Combobox(
            safe_line,
            textvariable=self.var_safe_mode,
            values=list(SAFE_EXPORT_MODES.keys()),
            state="readonly",
            width=14,
        )
        self.combo_safe_mode.pack(side="left", padx=(8, 10))
        self.lbl_safe_hint = ttk.Label(safe_line, text="", foreground="gray")
        self.lbl_safe_hint.pack(side="left", fill="x", expand=True)
        self.combo_safe_mode.bind("<<ComboboxSelected>>", lambda _event: self._sync_safe_hint())

        # Text-size row
        size_line = ttk.Frame(options)
        size_line.pack(anchor="w", fill="x", pady=(8, 0))
        self.var_text_limit_enabled = tk.BooleanVar(value=False)
        ttk.Checkbutton(
            size_line,
            text="Ограничить размер одного текстового файла:",
            variable=self.var_text_limit_enabled,
            command=self._sync_text_limit_state,
        ).pack(side="left")
        self.var_max_mb = tk.StringVar()
        self.entry_max_mb = ttk.Entry(size_line, width=8, textvariable=self.var_max_mb, justify="right")
        self.entry_max_mb.pack(side="left", padx=(8, 4))
        ttk.Label(size_line, text="МБ (по умолчанию без ограничения)").pack(side="left")

        # Archive row
        archive_line = ttk.Frame(options)
        archive_line.pack(anchor="w", fill="x", pady=(8, 0))
        ttk.Label(archive_line, text="Лимит одного ZIP-архива:").pack(side="left")
        self.var_zip_part_mb = tk.StringVar()
        self.entry_zip_part_mb = ttk.Entry(archive_line, width=8, textvariable=self.var_zip_part_mb, justify="right")
        self.entry_zip_part_mb.pack(side="left", padx=(8, 4))
        ttk.Label(archive_line, text="МБ; если итоговый ZIP больше — будет создана папка с логическими частями").pack(side="left")

        # Diff row
        diff_line = ttk.Frame(options)
        diff_line.pack(anchor="w", fill="x", pady=(8, 0))
        ttk.Label(diff_line, text="Diff Export:").pack(side="left")
        self.var_diff_mode = tk.StringVar()
        self.combo_diff_mode = ttk.Combobox(
            diff_line,
            textvariable=self.var_diff_mode,
            values=list(DIFF_EXPORT_MODES.keys()),
            state="readonly",
            width=20,
        )
        self.combo_diff_mode.pack(side="left", padx=(8, 10))
        self.lbl_diff_hint = ttk.Label(diff_line, text="", foreground="gray")
        self.lbl_diff_hint.pack(side="left", fill="x", expand=True)
        self.combo_diff_mode.bind("<<ComboboxSelected>>", lambda _event: self._sync_diff_hint())

        refs_line = ttk.Frame(options)
        refs_line.pack(anchor="w", fill="x", pady=(4, 0))
        ttk.Label(refs_line, text="Base ref:").pack(side="left")
        self.var_diff_base = tk.StringVar()
        self.entry_diff_base = ttk.Entry(refs_line, width=18, textvariable=self.var_diff_base)
        self.entry_diff_base.pack(side="left", padx=(8, 14))
        ttk.Label(refs_line, text="Target ref:").pack(side="left")
        self.var_diff_target = tk.StringVar()
        self.entry_diff_target = ttk.Entry(refs_line, width=18, textvariable=self.var_diff_target)
        self.entry_diff_target.pack(side="left", padx=(8, 0))

        # Checkboxes
        self.var_redact = tk.BooleanVar(value=True)
        ttk.Checkbutton(
            options,
            text="Маскировать очевидные секреты в текстовом дампе и Git-отчётах",
            variable=self.var_redact,
        ).pack(anchor="w", pady=(8, 0))

        self.var_include_git_patch = tk.BooleanVar(value=False)
        ttk.Checkbutton(
            options,
            text="Включать полный patch последнего Git-коммита (выключено по умолчанию из-за риска утечки секретов)",
            variable=self.var_include_git_patch,
        ).pack(anchor="w", pady=(4, 0))

        self.var_include_project = tk.BooleanVar(value=True)
        ttk.Checkbutton(
            options,
            text="Включать копию проекта внутрь итогового ZIP (иначе — только отчёты)",
            variable=self.var_include_project,
        ).pack(anchor="w", pady=(4, 0))

        self.var_keep_staging = tk.BooleanVar(value=False)
        ttk.Checkbutton(
            options,
            text="Оставить распакованную staging-папку рядом с результатом",
            variable=self.var_keep_staging,
        ).pack(anchor="w", pady=(4, 0))

        extras_line = ttk.Frame(options)
        extras_line.pack(anchor="w", fill="x", pady=(8, 0))
        ttk.Label(extras_line, text="Дополнительно исключить папки (через запятую):").pack(side="left")
        self.var_extra_ignored = tk.StringVar()
        self.entry_extra_ignored = ttk.Entry(extras_line, textvariable=self.var_extra_ignored)
        self.entry_extra_ignored.pack(side="left", fill="x", expand=True, padx=(8, 0))

        ttk.Label(
            options,
            text="Базовые исключения (.git, node_modules, .venv, build/dist/cache) сохраняются всегда. Git-команды read-only.",
            foreground="gray",
        ).pack(anchor="w", pady=(8, 0))

        actions = ttk.Frame(root)
        actions.grid(row=5, column=0, columnspan=3, sticky="we", pady=(8, 10))

        self.btn_start = ttk.Button(actions, text="▶ Создать экспорт", command=lambda: self._start(False))
        self.btn_start.pack(side="left")

        self.btn_codex = ttk.Button(actions, text="⚡ Codex Package", command=lambda: self._start(True))
        self.btn_codex.pack(side="left", padx=(8, 0))

        self.btn_cancel = ttk.Button(actions, text="Отмена", command=self._cancel, state="disabled")
        self.btn_cancel.pack(side="left", padx=8)

        self.btn_open_result = ttk.Button(actions, text="Открыть результат", command=self._open_last_result, state="disabled")
        self.btn_open_result.pack(side="left")

        ttk.Button(actions, text="История", command=self._show_history).pack(side="left", padx=8)
        ttk.Button(actions, text="Сброс настроек", command=self._reset_settings).pack(side="right")

        progress_line = ttk.Frame(root)
        progress_line.grid(row=6, column=0, columnspan=3, sticky="we", pady=(4, 8))
        self.progress = ttk.Progressbar(progress_line, mode="indeterminate")
        self.progress.pack(side="left", fill="x", expand=True)
        self.lbl_status = ttk.Label(progress_line, text="Готов", width=18)
        self.lbl_status.pack(side="left", padx=(10, 0))

        self.log = scrolledtext.ScrolledText(root, height=22, state="disabled", wrap="word", font=("Consolas", 9))
        self.log.grid(row=7, column=0, columnspan=3, sticky="nsew")

        ttk.Label(
            root,
            text="Результат создаётся на Desktop: один ZIP либо папка *_archives, если требуется разбиение.",
            foreground="gray",
        ).grid(row=8, column=0, columnspan=3, sticky="w", pady=(8, 0))

        root.columnconfigure(0, weight=1)
        root.rowconfigure(7, weight=1)

    # -- Config sync --------------------------------------------------------

    def _load_config_to_ui(self) -> None:
        self.entry_root.delete(0, "end")
        self.entry_root.insert(0, self.config.last_root)
        self.var_text_limit_enabled.set(self.config.text_file_size_limit_enabled)
        self.var_max_mb.set(str(self.config.max_text_file_mb))
        self.var_zip_part_mb.set(str(self.config.zip_part_limit_mb or MAX_ARCHIVE_PART_MB))
        self.var_redact.set(self.config.redact_secrets)
        self.var_include_git_patch.set(self.config.include_git_patch)
        self.var_include_project.set(self.config.include_project_in_zip)
        self.var_keep_staging.set(self.config.keep_staging_folder)
        self.var_extra_ignored.set(", ".join(self.config.extra_ignored_dirs))
        self.var_export_profile.set(self.config.normalized_export_profile())
        self.var_safe_mode.set(self.config.normalized_safe_export_mode())
        self.var_diff_mode.set(self.config.normalized_diff_export_mode())
        self.var_diff_base.set(self.config.diff_base_ref)
        self.var_diff_target.set(self.config.diff_target_ref)
        self._sync_profile_hint()
        self._sync_safe_hint()
        self._sync_diff_hint()
        self._sync_text_limit_state()

    def _save_config_from_ui(self) -> None:
        self.config = self._config_from_ui(save=True)
        self.config.save()

    def _config_from_ui(self, save: bool = False) -> Config:
        cfg = self.config if save else replace(self.config)
        cfg.last_root = self.entry_root.get().strip() or str(Path.home())
        cfg.text_file_size_limit_enabled = bool(self.var_text_limit_enabled.get())
        try:
            cfg.max_text_file_mb = max(1, int(self.var_max_mb.get().strip()))
        except Exception:
            cfg.max_text_file_mb = 5
            self.var_max_mb.set("5")
        try:
            cfg.zip_part_limit_mb = max(1, int(self.var_zip_part_mb.get().strip()))
        except Exception:
            cfg.zip_part_limit_mb = MAX_ARCHIVE_PART_MB
            self.var_zip_part_mb.set(str(MAX_ARCHIVE_PART_MB))

        cfg.redact_secrets = bool(self.var_redact.get())
        cfg.include_git_patch = bool(self.var_include_git_patch.get())
        cfg.include_project_in_zip = bool(self.var_include_project.get())
        cfg.keep_staging_folder = bool(self.var_keep_staging.get())
        cfg.export_profile = self.var_export_profile.get().strip()
        cfg.safe_export_mode = self.var_safe_mode.get().strip()
        cfg.diff_export_mode = self.var_diff_mode.get().strip()
        cfg.diff_base_ref = self.var_diff_base.get().strip() or "HEAD"
        cfg.diff_target_ref = self.var_diff_target.get().strip()

        extras: list[str] = []
        for token in re.split(r"[,;\n]", self.var_extra_ignored.get()):
            token = token.strip()
            if token and token not in extras:
                extras.append(token)
        cfg.extra_ignored_dirs = extras
        selected_profile_key = cfg.export_profile
        cfg = apply_custom_profile_if_needed(selected_profile_key, cfg)
        if cfg.safe_export_mode not in SAFE_EXPORT_MODES:
            cfg.safe_export_mode = "safe"
        if cfg.diff_export_mode not in DIFF_EXPORT_MODES:
            cfg.diff_export_mode = "all"
        return cfg

    # -- Buttons ------------------------------------------------------------

    def _browse_root(self) -> None:
        initial = self.entry_root.get().strip() or str(Path.home())
        selected = filedialog.askdirectory(initialdir=initial, title="Выберите корневую папку проекта")
        if selected:
            self.entry_root.delete(0, "end")
            self.entry_root.insert(0, selected)

    def _validate_before_start(self) -> Path | None:
        try:
            source_root = validate_source_root(self.entry_root.get())
        except Exception as exc:
            messagebox.showerror("Ошибка", str(exc))
            return None
        if self.var_text_limit_enabled.get():
            try:
                max(1, int(self.var_max_mb.get().strip()))
            except Exception:
                messagebox.showerror("Ошибка", "Максимальный размер файла должен быть целым числом.")
                return None
        try:
            max(1, int(self.var_zip_part_mb.get().strip()))
        except Exception:
            messagebox.showerror("Ошибка", "Лимит ZIP-архива должен быть целым числом.")
            return None
        return source_root

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
            zip_part_limit_mb=MAX_ARCHIVE_PART_MB,
        )

    def _confirm_risk_preview(self, source_root: Path, cfg: Config) -> bool:
        diff_selection = resolve_diff_selection(source_root, cfg.normalized_diff_export_mode(), cfg.diff_base_ref, cfg.diff_target_ref)
        report = build_pre_export_risk_preview(source_root, cfg.effective_ignored_dirs(), diff_selection)
        preview_text = format_risk_preview_for_user(report, cfg.normalized_safe_export_mode())
        return messagebox.askyesno("Pre-export risk preview", preview_text)

    def _start(self, codex_package: bool = False) -> None:
        if self.worker and self.worker.is_alive():
            return

        source_root = self._validate_before_start()
        if source_root is None:
            return

        self._save_config_from_ui()
        run_config = self._codex_config(self.config) if codex_package else replace(self.config)

        if not self._confirm_risk_preview(source_root, run_config):
            self._log("Экспорт отменён на этапе pre-export preview.")
            return

        self.cancel_event.clear()
        self.last_result_path = None
        self.btn_open_result.config(state="disabled")
        self._set_running(True)
        self._log(
            f"Запуск экспорта... profile={run_config.normalized_export_profile()}, "
            f"safe={run_config.normalized_safe_export_mode()}, diff={run_config.normalized_diff_export_mode()}"
        )

        exporter = ProjectExporter(source_root=source_root, config=run_config, log_queue=self.log_queue, cancel_event=self.cancel_event)

        def target() -> None:
            try:
                paths = exporter.run()
                archive_result = exporter.archive_result
                if archive_result and archive_result.primary_result and archive_result.primary_result.exists():
                    self.last_result_path = archive_result.primary_result
                elif paths.final_zip.exists():
                    self.last_result_path = paths.final_zip
                elif paths.archive_set_dir.exists():
                    self.last_result_path = paths.archive_set_dir
                elif paths.staging_dir.exists():
                    self.last_result_path = paths.staging_dir

                if self.cancel_event.is_set():
                    location = self.last_result_path or paths.staging_dir
                    self.master.after(0, lambda: messagebox.showwarning("Остановлено", f"Операция остановлена.\n{location}"))
                else:
                    self.master.after(0, lambda: self.btn_open_result.config(state="normal"))
                    location = self.last_result_path or paths.final_zip
                    self.master.after(0, lambda: messagebox.showinfo("Готово", f"Экспорт создан:\n{location}"))
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
        state = "disabled" if running else "normal"
        self.btn_start.config(state=state)
        self.btn_codex.config(state=state)
        self.btn_cancel.config(state="normal" if running else "disabled")
        self.lbl_status.config(text="Выполняется..." if running else "Готов")
        if running:
            self.progress.start(15)
        else:
            self.progress.stop()

    def _open_path(self, path: Path) -> None:
        try:
            if os.name == "nt":
                os.startfile(str(path))  # type: ignore[attr-defined]
            elif sys.platform == "darwin":
                subprocess.Popen(["open", str(path)])
            else:
                subprocess.Popen(["xdg-open", str(path)])
        except Exception as exc:
            messagebox.showerror("Ошибка", f"Не удалось открыть путь:\n{path}\n\n{exc}")

    def _open_desktop(self) -> None:
        self._open_path(desktop_path())

    def _open_last_result(self) -> None:
        if self.last_result_path and self.last_result_path.exists():
            self._open_path(self.last_result_path)
        else:
            messagebox.showwarning("Нет результата", "Итоговый файл или папка пока не созданы.")

    def _open_profiles_json(self) -> None:
        self._open_path(ensure_user_profiles_file())

    def _show_history(self) -> None:
        history = load_export_history()[:10]
        if not history:
            messagebox.showinfo("История", "История экспортов пока пуста.")
            return
        lines: list[str] = []
        for item in history:
            lines.append(
                f"{item.get('generated_at', '')}\n"
                f"Project: {item.get('project_name', '')}\n"
                f"Profile: {item.get('profile', '')}; Safe: {item.get('safe_export_mode', '')}; Split: {item.get('split_archives', False)}\n"
                f"Result: {item.get('result', '')}\n"
            )
        messagebox.showinfo("Последние экспорты", "\n".join(lines))

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
