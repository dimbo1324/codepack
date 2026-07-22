# Чек-лист задачи

**Задача:** Этап **S0 — Фундамент репозитория и гейт качества** (`ROADMAP.md` §2).
Задача была начата в предыдущей сессии (Cargo workspace и крейты-плейсхолдеры уже
существовали), но не завершена: код ни разу не проходил `cargo fmt`, `cargo xtask
sync-agents --check` падал (бюджет `AGENTS.md` превышен), CI отсутствовал.
**Дата:** 2026-07-22
**Ветка:** feat/s0-repo-foundation-quality-gate

## Подготовка

- [+] Ритуал ориентации: `git status`/`git log`, `ROADMAP.md` §1 (S0 без строки
      `**Status.**` — первый в очереди), `docs/architecture/overview.md`,
      `task-checklist.md`, `docs/decisions/open-questions.md`
- [+] Инвентаризация уже существующей части S0: workspace, 10 крейтов-плейсхолдеров,
      `crates/xtask` с командами `gate/fmt/lint/test/sync-agents/doctor`,
      `rust-toolchain.toml`, `rustfmt.toml`, `deny.toml`, `.gitignore`, `.editorconfig`
- [+] Выявлены незавершённые места: код не отформатирован, `sync-agents --check`
      падает (31.7 KiB > 30 KiB), `.github/workflows` отсутствует, временный
      Python-скрипт синхронизации не удалён, `docs/architecture/overview.md` устарел

## Реализация

- [+] `cargo xtask fmt` — привести весь существующий Rust-код к `rustfmt.toml`
- [+] Вернуть `AGENTS.md` под бюджет 30 KiB: пометить `.ai/universal/08-rules-
      evolution.md` и `.ai/project/14-legacy-reference.md` как `tier: extended` с
      однострочной `> **Essence.**` (обнаружен и исправлен баг: essence не должен
      переноситься на несколько строк — парсер читает только первую)
- [+] Удалить `dev_tools_scripts_runner.py` и `scripts/dev_tools/sync_agents_md.py`;
      обновить ссылки на них в `.ai/README.md` и `.claude/settings.json`
- [+] Записать изменение модулей в `.ai/CHANGELOG.md`
- [+] Добавить `.github/workflows/ci.yml` — матрица `ubuntu-latest`/`macos-latest`/
      `windows-latest`, шаг `cargo xtask gate`

## Проверка

- [+] `cargo xtask gate` зелёный локально (fmt, clippy -D warnings, тесты, sync-agents
      --check; 3/3 юнит-теста xtask проходят)
- [+] `cargo xtask sync-agents --check` проходит (25.7 KiB из 30 KiB)
- [-] Гейт в CI на трёх ОС — воркфлоу добавлен, но не подтверждён живым прогоном:
      подтвердится при первом push ветки/PR в `origin`
- [+] `docs/architecture/overview.md` обновлён под фактическое состояние кода
- [+] `ROADMAP.md`: строка `**Status.**` под S0 и статус в таблице §1 обновлены

## Завершение

- [+] Коммиты изменений (раздельно: чек-лист, код и правила, CI)
- [ ] Fast-forward merge в `main` (после явного согласия владельца на публикацию)
- [+] Финальный отчёт владельцу

---

## Следующая задача

Этап **S1 — Доменные типы и конфигурация (`codepack-core`)** (`ROADMAP.md` §2).
Начать с ритуала ориентации из `.ai/project/13-progress-tracking.md`.
