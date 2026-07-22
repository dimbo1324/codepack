# Текущее состояние архитектуры

> Этот документ описывает то, что **реально существует в коде**, а не то, что
> запланировано. План — в `ROADMAP.md`, замысел — в `BLUEPRINT.md`.
>
> Обновляется каждый раз, когда меняется форма системы: новый крейт, новый слой,
> новая операционная задача.

**Дата последней ревизии:** 2026-07-22
**Состояние:** этапы S0 и S1 завершены. `codepack-core` — единственный крейт с
реальной доменной логикой; остальные крейты «Ядра» пока плейсхолдеры.

## Что существует

| Область | Состояние |
|---|---|
| Cargo workspace (`Cargo.toml`, `resolver = "2"`) | **готов** |
| `codepack-core` | **готов (S1)**: `Config` (26 полей legacy + `schema_version`), нормализация, миграция legacy-настроек, 5 AI-пресетов (данные), `AppPaths`, `CancellationToken`, `ProgressEvent`/`LogEvent`, 5 общих типов пайплайна (`ExportPaths`, `CopyStats`, `TextDumpStats`, `RiskPreviewReport`, `ArchiveBuildResult`). 50 юнит- + 6 интеграционных тестов (56 всего) |
| Остальные крейты Rust (`crates/`) | **плейсхолдеры**: `-scanner`, `-security`, `-diff`, `-storage`, `-tokens`, `-reports`, `-archive`, `-engine` (`lib.rs`), `codepack-cli` (`main.rs`, заглушка стадии S10) |
| `cargo xtask` (`crates/xtask`) | **готов**: `gate`, `fmt`, `lint`, `test`, `deny`, `sync-agents [--check]`, `doctor` |
| `rust-toolchain.toml`, `rustfmt.toml`, workspace lints | **готовы** |
| `deny.toml` | **активен с S1**: advisories/bans/licenses/sources все `ok`; собственные крейты исключены из license-проверки (`[licenses.private] ignore = true`) — у них ещё нет решения о лицензии, это отдельный вопрос |
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

Этап **S2 — Сканер: обход, ignore, детект стека (`codepack-scanner`)**.
См. `ROADMAP.md` §2.
