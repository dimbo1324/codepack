# Текущее состояние архитектуры

> Этот документ описывает то, что **реально существует в коде**, а не то, что
> запланировано. План — в `ROADMAP.md`, замысел — в `BLUEPRINT.md`.
>
> Обновляется каждый раз, когда меняется форма системы: новый крейт, новый слой,
> новая операционная задача.

**Дата последней ревизии:** 2026-07-23
**Состояние:** этапы S0, S1, S2, S3 и S4 завершены. `codepack-core`, `codepack-scanner`,
`codepack-security` и `codepack-diff` — крейты с реальной доменной логикой; остальные
крейты «Ядра» пока плейсхолдеры.

## Что существует

| Область | Состояние |
|---|---|
| Cargo workspace (`Cargo.toml`, `resolver = "2"`) | **готов** |
| `codepack-core` | **готов (S1)**: `Config` (26 полей legacy + `schema_version`), нормализация, миграция legacy-настроек, 5 AI-пресетов (данные), `AppPaths`, `CancellationToken`, `ProgressEvent`/`LogEvent`, 5 общих типов пайплайна (`ExportPaths`, `CopyStats`, `TextDumpStats`, `RiskPreviewReport`, `ArchiveBuildResult`). 50 юнит- + 6 интеграционных тестов (56 всего) |
| `codepack-scanner` | **готов (S2, в границах этапа — без safe-mode/diff-фильтрации, см. `ROADMAP.md` §2 Status)**: базовое+стековое игнорирование директорий (`walk.rs`, `IgnoredDirMatcher`, `walkdir` без следования симлинкам), `.exportignore`/кастомные правила (`ignore/`, ручной `fnmatch`-эквивалент на `regex`), детектор 12 стеков (`stack.rs`), классификация текст/бинарь (`classify.rs`), `build_export_plan()`/`write_export_plan_files()` (`plan/`, JSON+Markdown, порядок полей — контракт I5). 85 юнит- + 13 интеграционных тестов |
| `codepack-security` | **готов (S3)**: три safe-режима (`policy/`), редактирование секретов с безбэкреференсным переписыванием legacy-регекса (`redact.rs`), эвристический сканер v3 (`scan/`) — sensitive-файлы, secret-каскад (4 уровня уверенности), 9 risky-code правил, плюс новое из BLUEPRINT §B.1: 10 провайдер-сигнатур, энтропия Шеннона, `aho-corasick`-предфильтр (`patterns/`). Выходы `.txt`/`.json`/SARIF 2.1.0 (`scan/write/`). Корпус-baseline (I9): parity P=1.000/R=0.312/F1=0.476, full P=1.000/R=1.000/F1=1.000. 111 юнит- + 7 интеграционных тестов |
| `codepack-diff` | **готов (S4)**: 4 режима diff (`all`/`last_export`/`git_ref`/`uncommitted`) через `git2` (только чтение, никогда сеть), снапшот проекта с потоковым SHA-256 (`snapshot/`), `last_export` берёт предыдущий снапшот аргументом (без хранилища — это S5), отчёт `29_export_comparison_report.md`. Первая C-зависимость воркспейса (`libgit2-sys`, vendored, без `https`/`ssh`/`cred`-фич). 40 тестов |
| Остальные крейты Rust (`crates/`) | **плейсхолдеры**: `-storage`, `-tokens`, `-reports`, `-archive`, `-engine` (`lib.rs`), `codepack-cli` (`main.rs`, заглушка стадии S10) |
| `cargo xtask` (`crates/xtask`) | **готов**: `gate`, `fmt`, `lint`, `test`, `deny`, `sync-agents [--check]`, `doctor` |
| `rust-toolchain.toml`, `rustfmt.toml`, workspace lints | **готовы** |
| `deny.toml` | **активен с S1**: advisories/bans/licenses/sources все `ok`; собственные крейты исключены из license-проверки (`[licenses.private] ignore = true`); с S2 добавлен `[bans] allow-wildcard-paths = true` (внутриворкспейсные path-зависимости без semver-диапазона — не supply-chain риск) |
| Десктоп-приложение (`apps/desktop`) | не создано — этап S11 |
| CI (`.github/workflows/ci.yml`) | **готов**: матрица `ubuntu-latest` / `macos-latest` / `windows-latest`, устанавливает `cargo-deny` и запускает `cargo xtask gate` |
| Гейт качества (`cargo xtask gate`) | **зелёный локально и в CI**: fmt, clippy `-D warnings`, тесты, `cargo deny check`, `sync-agents --check` |
| Инфраструктура ИИ-агентов (`.ai/`, `.claude/`, `.codex/`) | **готова**; синхронизация `AGENTS.md` — `cargo xtask sync-agents` |
| Спецификация продукта (`BLUEPRINT.md`) | **готова** |
| План реализации (`ROADMAP.md`) | **готов** |
| Архив старой реализации | `docs/__arch__/codepack-main.zip` |

## Что было раньше

Предыдущая версия — Project Exporter Desktop 1.0.1: Python 3.11+, PySide6 (Qt),
около 13 400 строк, 22 тестовых модуля, только Windows, дистрибуция через PyInstaller
и Inno Setup. Полностью удалена из рабочего дерева; сохранена в архиве и подробно
описана в `BLUEPRINT.md` части A.

Причины переписывания: кроссплатформенность, производительность без ограничений GIL,
строгая типизация, замена плоских JSON на SQLite, усиление детектора секретов.

## Целевая форма системы

Описана в `BLUEPRINT.md` §C (архитектура Rust + Tauri) и §D (модель данных).
Порядок появления частей — в `ROADMAP.md` §1.

Кратко: слой крейтов `codepack-*` (ядро, независимое от UI) → `codepack-engine`
(оркестратор) → две точки входа: `codepack-cli` (headless) и `apps/desktop`
(Tauri + TypeScript).

## Следующий шаг

Этап **S5 — Хранилище SQLite (`codepack-storage`)**.
См. `ROADMAP.md` §2.
