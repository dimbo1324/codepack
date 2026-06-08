from __future__ import annotations

from pathlib import Path

from ...utils.time_utils import human_now

PROMPTS: dict[str, str] = {
    "01_project_overview_prompt.md": """# Project overview prompt

You are a senior software architect. Analyse this exported project package.

Use these files first:
- `PROJECT_PROFILE.json`
- `INDEX.md`
- `manifest.json`
- `reports/insights/12_ai_context_pack.md`
- `reports/insights/AI_CONTEXT/`

Tasks:
1. Explain what the project does.
2. Identify the main entry points and execution flow.
3. Map the architecture by layers and responsibilities.
4. List the top risks and the most valuable next improvements.
5. Be explicit when information is missing or uncertain.
""",
    "02_codex_refactor_prompt.md": """# Codex refactoring prompt

You are Codex working as a senior engineer in this repository.

Before changing code:
1. Read `PROJECT_PROFILE.json`, `manifest.json`, `INDEX.md` and `reports/insights/24_architecture_map.md`.
2. Create a new task branch.
3. Make a minimal, well-tested change.
4. Run all available checks and tests.
5. Merge into `main` only after checks pass.
6. Finish with `main` as the final branch.

Task:
Refactor the highest-impact maintainability issue without changing user-visible behaviour. Preserve security boundaries and update tests/docs when needed.
""",
    "03_security_review_prompt.md": """# Security review prompt

You are a senior application security reviewer.

Focus on:
- secret handling and redaction;
- unsafe file export or archive packaging paths;
- subprocess usage;
- symlink/path traversal issues;
- accidental disclosure through logs, Git reports and text dumps;
- unsafe defaults.

Use `reports/insights/06_security_scan.txt`, `manifest.json`, and the source code. Produce a prioritised risk register with concrete fixes.
""",
    "04_bug_hunt_prompt.md": """# Bug hunt prompt

You are a senior QA engineer and Python developer.

Find likely bugs in edge cases:
- empty projects;
- huge files;
- unreadable files;
- cancelled exports;
- Windows path quirks;
- non-Git projects;
- Unicode paths;
- duplicate archive names;
- disabled text-size limit.

For every bug, propose a small reproducible test and a safe patch.
""",
    "05_architecture_review_prompt.md": """# Architecture review prompt

You are a principal software architect.

Review the project architecture against:
- separation of UI, services, reports and utilities;
- testability;
- coupling and dependency direction;
- long-term extensibility;
- removal of dead code;
- production-readiness.

Use `reports/insights/15_architecture_report.md`, `24_architecture_map.md`, and `17_code_quality_report.md`.
""",
}


def write_ai_prompt_files(_copied_root: Path, output_dir: Path) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    for filename, content in PROMPTS.items():
        header = f"<!-- Generated: {human_now()} -->\n\n"
        (output_dir / filename).write_text(header + content.strip() + "\n", encoding="utf-8")
