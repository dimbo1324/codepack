# Текущее состояние архитектуры

> Этот документ описывает то, что **реально существует в коде**, а не то, что
> запланировано. План — в `ROADMAP.md`, замысел — в `BLUEPRINT.md`.
>
> Обновляется каждый раз, когда меняется форма системы: новый крейт, новый слой,
> новая операционная задача.

**Дата последней ревизии:** 2026-07-22
**Состояние:** этап S0 завершён — фундамент репозитория собирается, тестируется и
проверяется гейтом; продуктового кода (S1+) ещё нет.

## Что существует

| Область | Состояние |
|---|---|
| Cargo workspace (`Cargo.toml`, `resolver = "2"`) | **готов** |
| Крейты Rust (`crates/`) | **созданы как плейсхолдеры**: `codepack-core`, `-scanner`, `-security`, `-diff`, `-storage`, `-tokens`, `-reports`, `-archive`, `-engine` (все `lib.rs`), `codepack-cli` (`main.rs`, заглушка стадии S10) |
| `cargo xtask` (`crates/xtask`) | **готов**: `gate`, `fmt`, `lint`, `test`, `sync-agents [--check]`, `doctor` |
| `rust-toolchain.toml`, `rustfmt.toml`, workspace lints | **готовы** |
| `deny.toml` | **готов** (реальную проверку выполнит с первой зависимостью в S1) |
| Десктоп-приложение (`apps/desktop`) | не создано — этап S11 |
| CI (`.github/workflows/ci.yml`) | **готов**: матрица `ubuntu-latest` / `macos-latest` / `windows-latest`, запускает `cargo xtask gate` |
| Гейт качества (`cargo xtask gate`) | **готов и зелёный локально**: fmt, clippy `-D warnings`, тесты, `sync-agents --check` |
| Инфраструктура ИИ-агентов (`.ai/`, `.claude/`, `.codex/`) | **готова**; синхронизация `AGENTS.md` переведена с временного Python-скрипта на `cargo xtask sync-agents` |
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

Этап **S1 — Доменные типы и конфигурация (`codepack-core`)**. См. `ROADMAP.md` §2.
