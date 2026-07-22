---
name: stage-episode
description: Use to plan and execute one ROADMAP stage (S0–S14) end to end — orientation, scoping, parity-first implementation, verification, and status update.
---

# Выполнение этапа дорожной карты

Один этап `ROADMAP.md` — одна задача. Не смешивайте два этапа в одной задаче и не
обгоняйте порядок S0→S14 без решения владельца.

## 1. Ориентация (обязательно, без пропусков)

```powershell
git status --short --branch
git log --oneline -15
```

Затем прочитайте по порядку: `ROADMAP.md` §1 и строки `**Status.**` (первый этап без
строки — ваш), `docs/architecture/overview.md`, `task-checklist.md`,
`docs/decisions/open-questions.md`.

Если в `task-checklist.md` остались незакрытые пункты `[ ]` от прошлой сессии —
сначала разберитесь с ними.

## 2. Планирование

Для крупного или незнакомого этапа поднимите субагента `codepack-stage-planner`.

Определите: границы этапа, требуемый паритет со старой версией, какие 🎯-возможности
входят, риски для инвариантов, критерии приёмки.

Заполните `task-checklist.md` пунктами `[ ]` по секциям (подготовка / реализация /
проверка / завершение) и **закоммитьте до начала кода**.

## 3. Ветка

```powershell
git checkout main
git pull --ff-only origin main
git checkout -b feat/s<N>-краткое-описание
```

## 4. Реализация — паритет прежде новизны

Сначала воспроизведите поведение старой версии, затем добавляйте новое. Источник
фактов — `BLUEPRINT.md`; при необходимости буквальной точности используйте скилл
`legacy-lookup`.

Делегируйте профильным субагентам: `codepack-core-engine`, `codepack-security`,
`codepack-reports`, `codepack-desktop-ui`.

## 5. Проверка

```powershell
cargo xtask gate
```

До этапа S0 доступна только проверка синхронизации правил:
`python dev_tools_scripts_runner.py sync-agents --check`.

Перед финализацией прогоните ревью субагентом `codepack-quality-reviewer`.

## 6. Завершение

- Отметьте пункты чек-листа `+`/`-` честно.
- Добавьте строку `**Status.**` под этапом в `ROADMAP.md` и обновите таблицу §1.
- Обновите `docs/architecture/overview.md`, если изменилась форма системы.
- Слейте `--ff-only` в `main` только при зелёном гейте.
- Напишите финальный отчёт: что сделано, что проверено, что не сделано.
