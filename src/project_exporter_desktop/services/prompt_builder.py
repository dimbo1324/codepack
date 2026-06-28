# Builds the CUSTOM_PROMPT.md AI instruction file included in every export bundle.
# Goal keys are validated against PROMPT_GOALS and default to a sensible review set.

from __future__ import annotations

from pathlib import Path

PROMPT_GOALS: dict[str, str] = {
    "bug_hunt": "Найти вероятные ошибки, сломанные крайние случаи и логические несоответствия.",
    "security_review": "Проверить безопасность, обработку секретов и опасные паттерны.",
    "architecture_review": "Оценить архитектуру, слои, модульность и связность.",
    "refactor_plan": "Составить безопасный пошаговый план рефакторинга.",
    "write_tests": "Выявить недостающие тесты и предложить практичный тест-план.",
    "codex_task": "Подготовить задачу для Codex с понятными критериями реализации.",
}


def normalise_prompt_goals(goals: list[str] | None) -> list[str]:
    cleaned: list[str] = []
    for goal in goals or []:
        goal = goal.strip()
        if goal in PROMPT_GOALS and goal not in cleaned:
            cleaned.append(goal)
    return cleaned or ["architecture_review", "bug_hunt", "write_tests"]  # default goals when none are specified or all are invalid


def build_custom_prompt(project_name: str, goals: list[str] | None = None) -> str:
    selected = normalise_prompt_goals(goals)
    goal_lines = "\n".join(f"- {PROMPT_GOALS[key]}" for key in selected)
    return (
        f"# Промпт для анализа проекта: {project_name}\n\n"
        "Ты работаешь с экспортом проекта. Сначала изучи структуру, manifest, профиль проекта, "
        "план экспорта и отчёты безопасности. Затем выполни задачи ниже.\n\n"
        "## Цели\n\n"
        f"{goal_lines}\n\n"
        "## Формат ответа\n\n"
        "- Сначала перечисли критичные проблемы с файлами и причинами.\n"
        "- Затем предложи небольшие улучшения с низким риском внедрения.\n"
        "- Отдельно отметь тесты, которые нужно добавить или обновить.\n"
        "- Не переписывай проект целиком без необходимости.\n"
    )


def write_custom_prompt(
    output_file: Path, project_name: str, goals: list[str] | None = None
) -> None:
    output_file.parent.mkdir(parents=True, exist_ok=True)
    output_file.write_text(build_custom_prompt(project_name, goals), encoding="utf-8", newline="\n")
