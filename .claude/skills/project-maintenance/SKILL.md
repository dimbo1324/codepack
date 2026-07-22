---
name: project-maintenance
description: Use for routine codepack upkeep — formatting, rule-module sync, mirror consistency, state-document updates, and explicitly requested publishing.
---

# Поддержка репозитория

Используйте автоматизацию проекта вместо ручных наборов команд.

## Быстрые пути

Синхронизация правил после правки модулей `.ai/` (обязательна):

```powershell
python dev_tools_scripts_runner.py sync-agents
```

Проверка, что точка входа Codex актуальна:

```powershell
python dev_tools_scripts_runner.py sync-agents --check
```

После этапа S0 те же операции доступны как `cargo xtask sync-agents`, плюс:

```powershell
cargo xtask fmt
cargo xtask gate --quick
cargo xtask doctor
```

## Правила

- `AGENTS.md` **генерируется** — руками не редактируется никогда. Правится модуль
  в `.ai/`, затем запускается синхронизация.
- Бюджет `AGENTS.md` — 30 KiB. Если сборка упёрлась в лимит: уплотнить модуль либо
  пометить ситуативный маркером `<!-- tier: extended -->` и добавить ему строку
  `> **Суть.**` с однострочным резюме.
- `.claude/agents|skills` и `.codex/agents|skills` — зеркала по именам. Изменение одной
  стороны требует эквивалентного изменения другой **в той же задаче**.
- `.claude/settings.json`: разрешения расширять можно, запреты удалять — только по
  явному решению владельца.

## Документы состояния

- Завершён этап → строка `**Status.**` под этапом в `ROADMAP.md` + статус в таблице §1.
- Изменилась форма системы → `docs/architecture/overview.md`.
- Решение владельца → `docs/decisions/open-questions.md`.
- Новый инвариант → `docs/architecture/invariants.md`.
- Задача закрыта → `task-checklist.md` с честными отметками `+`/`-`.

## Публикация

Пуш в `main` — только когда владелец явно попросил об этом в текущей задаче.
Маршрут по умолчанию: ветка → гейт → ff-слияние → отчёт. Красный гейт запрещает
публикацию.
