# Task Checklist

**Task:** Этап **S11 — Tauri-оболочка и UI** (`ROADMAP.md` §3). Первый этап,
создающий `apps/desktop/src-tauri` (не существовал) и реальный `apps/desktop/ui`
(до сих пор был только записью в карте проекта, каталога не было).
**Date:** 2026-07-25
**Branch:** feat/s11-tauri-shell-and-ui

## Preparation

- [+] Ритуал ориентации (git log, ROADMAP §1/§3, overview.md, task-checklist.md,
      open-questions.md, invariants.md)
- [+] Подтверждено: `apps/` не существует вовсе — ни `ui`, ни `src-tauri`. Оба
      создаются с нуля в этой задаче
- [+] Окружение: Node v22.22, pnpm 9.12 — есть; `cargo tauri` CLI — нет (используем
      `@tauri-apps/cli` через pnpm, стандартный путь для JS-проектов, без
      глобальной установки)
- [+] Фреймворк фронтенда: **Svelte + Vite + TypeScript**, а не React —
      BLUEPRINT §C.5 называет его первой рекомендацией («лёгкий и предсказуемый»),
      и он меньше по рантайму/зависимостям, что соответствует
      `.ai/universal/03-scope-and-code-style.md` (минимум зависимостей)
- [+] Архитектурное решение: `apps/desktop/src-tauri` — обычный бинарный крейт в
      cargo-воркспейсе, зовёт `codepack-engine`/`-core`/`-storage`/`-security`
      напрямую (BLUEPRINT §C.2: TAURI ↔ C_ENG), а не через `codepack-cli` —
      Tauri-команды это второй, не производный, потребитель ядра
- [+] Многие поля `Config` для этого этапа уже существуют с S1 (`theme`,
      `ui_zoom`, `language`, `watch_enabled`, `watch_clipboard_auto_update`) —
      переиспользуются, не создаются заново

## Реализация — backend (`apps/desktop/src-tauri`)

- [ ] Cargo-крейт добавлен в воркспейс; `tauri.conf.json`, окно, минимальный
      `main.rs`
- [ ] Команды: выбор проекта (диалог, без прямого доступа UI к ФС), `preview`,
      `scan`, `export` (поток прогресса через Tauri events), `cancel_export`,
      `history`, настройки, каталог пресетов/профилей
- [ ] Мост прогресса/лога: `codepack_core::progress_channel` → `app.emit`
- [ ] Отмена: `CancellationToken` на активный прогон, staging подчищается
- [ ] Watch-режим (`notify`), опциональное авто-обновление буфера обмена
- [ ] Системный трей (нативный API Tauri v2)
- [ ] Масштаб UI (`Config::ui_zoom`)

## Реализация — frontend (`apps/desktop/ui`)

- [ ] Vite + Svelte + TypeScript, `pnpm-workspace.yaml`, пакет `@codepack/ui`
- [ ] i18n RU/EN без перезапуска
- [ ] Тема светлая/тёмная/системная, синхронизирована с `Config::theme`
- [ ] Типизированный клиент над `invoke()`
- [ ] Страницы-мастер: Проект → Настройки → Безопасность → Предпросмотр →
      Журнал → Результат → История → Аналитика
- [ ] Дерево предпросмотра: included/excluded/warning, ручные переопределения
- [ ] Страница журнала: живые события прогресса/лога
- [ ] Страница результата: путь(и) архива, число critical-находок, открыть папку
- [ ] Изоляция: фронтенд не имеет прямого доступа к ФС

## Verification

- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace` — не ниже baseline 820
- [ ] `pnpm --filter @codepack/ui typecheck`, `pnpm --filter @codepack/ui lint`
- [ ] Тесты Tauri-команд (вызов функций напрямую)
- [ ] E2E-smoke: выбрать проект → предпросмотр → экспорт → открыть результат
- [ ] Отмена длинного экспорта из UI не оставляет мусора — отдельный тест
- [ ] Независимое ревью диффа
- [-] `cargo deny check` — недоступен в песочнице, проверяется в CI
- [-] Реальный `tauri build` (нативный инсталлятор) — это S14, не требуется
      критериями готовности S11

## Completion

- [ ] `ROADMAP.md` — статус S11 + таблица §1
- [ ] `docs/architecture/overview.md` обновлён
- [ ] `.ai/project/10-project-map.md`/`11-commands.md` актуализированы
- [ ] Чек-лист заполнен `+`/`-`, финальный отчёт владельцу

## Next task

Этап **S12 — Человеко-ориентированные результаты** (`ROADMAP.md` §4).
