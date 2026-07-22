# codepack — рабочие заметки для Claude Code

Этот файл — точка входа Claude Code. Все правила живут в общих модулях внутри `.ai/` —
единственном источнике правды для каждого ИИ-ассистента в репозитории. Codex читает те
же модули через сгенерированный `AGENTS.md`; править его руками нельзя (правьте модуль,
затем запустите `python dev_tools_scripts_runner.py sync-agents`).

Более поздние модули переопределяют более ранние; явная инструкция владельца в текущем
диалоге переопределяет всё.

## Быстрый старт в новой сессии

Проект разрабатывается почти полностью ИИ-агентами, сессии часто начинаются с нуля.
Перед любой работой выполните **ритуал ориентации** из `@.ai/project/13-progress-tracking.md`:
git-статус и лог → `ROADMAP.md` (первый этап без строки `**Status.**` — следующий) →
`docs/architecture/overview.md` → `task-checklist.md` → `docs/decisions/open-questions.md`.

Замысел продукта целиком описан в `BLUEPRINT.md`. Старая реализация на Python лежит
в `docs/__arch__/codepack-main.zip` и служит эталоном поведения.

## Универсальные правила (переносимы в любой проект)

- @.ai/universal/01-workflow.md
- @.ai/universal/02-task-checklist.md
- @.ai/universal/03-scope-and-code-style.md
- @.ai/universal/04-architecture-boundaries.md
- @.ai/universal/05-security-and-secrets.md
- @.ai/universal/06-quality-and-testing.md
- @.ai/universal/07-multi-assistant.md

## Правила проекта (codepack)

- @.ai/project/10-project-map.md
- @.ai/project/11-commands.md
- @.ai/project/12-domain-rules.md
- @.ai/project/13-progress-tracking.md
- @.ai/project/14-legacy-reference.md

## Рабочее пространство Claude Code

- `.claude/settings.json` — списки разрешённых и запрещённых команд.
- `.claude/agents/` — проектные субагенты (`codepack-*`) для делегирования;
  зеркалят `.codex/agents/` один в один.
- `.claude/skills/` — переиспользуемые процессы (`stage-episode`, `legacy-lookup`,
  `code-review`, `ci-fix`, `project-maintenance`); зеркалят `.codex/skills/`.
