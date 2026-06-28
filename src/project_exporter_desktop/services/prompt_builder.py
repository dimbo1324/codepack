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
    return f"""# Custom AI Project Review Prompt

Generated: {human_now()}
Project: {project_name}

You are a senior software architect and code reviewer. Analyse the exported project package.

Use these files first:
- PROJECT_PROFILE.json
- INDEX.md
- reports/insights/REPORT_DASHBOARD.html
- reports/insights/22_project_health_report.md
- reports/insights/24_architecture_map.md
- reports/insights/AI_CONTEXT/

## Goals

{goal_lines}

## Output format

1. Executive summary.
2. Critical issues first.
3. Concrete file-level recommendations.
4. Safe implementation plan split into small steps.
5. Tests/checks that must pass before merging.

Do not invent files that are not present in the export. If information is missing, say exactly what is missing.
"""


def write_custom_prompt(
    output_file: Path, project_name: str, goals: list[str] | None = None
) -> None:
    output_file.parent.mkdir(parents=True, exist_ok=True)
    output_file.write_text(build_custom_prompt(project_name, goals), encoding="utf-8", newline="\n")