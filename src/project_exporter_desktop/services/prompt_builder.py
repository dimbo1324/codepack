from __future__ import annotations

from pathlib import Path

from ..utils.time_utils import human_now

PROMPT_GOALS: dict[str, str] = {
    "bug_hunt": "Find likely bugs and broken edge cases.",
    "security_review": "Review security, secret handling and unsafe patterns.",
    "architecture_review": "Review architecture, layering, modularity and coupling.",
    "refactor_plan": "Create a safe step-by-step refactoring plan.",
    "write_tests": "Identify missing tests and write a practical test plan.",
    "codex_task": "Prepare a Codex-ready implementation task prompt.",
}


def normalise_prompt_goals(goals: list[str] | None) -> list[str]:
    cleaned: list[str] = []
    for goal in goals or []:
        goal = goal.strip()
        if goal in PROMPT_GOALS and goal not in cleaned:
            cleaned.append(goal)
    return cleaned or ["architecture_review", "bug_hunt", "write_tests"]


def build_custom_prompt(project_name: str, goals: list[str] | None = None) -> str:
    selected = normalise_prompt_goals(goals)
    goal_lines = "\n".join(f"- {PROMPT_GOALS[key]}" for key in selected)


def write_custom_prompt(
    output_file: Path, project_name: str, goals: list[str] | None = None
) -> None:
    output_file.parent.mkdir(parents=True, exist_ok=True)
    output_file.write_text(build_custom_prompt(project_name, goals), encoding="utf-8", newline="\n")