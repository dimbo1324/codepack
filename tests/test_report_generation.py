from __future__ import annotations

from pathlib import Path

from project_exporter_desktop.reports.insights.ai_context_folder import write_ai_context_folder
from project_exporter_desktop.reports.insights.dashboard import write_html_dashboard
from project_exporter_desktop.services.prompt_builder import (
    build_custom_prompt,
    write_custom_prompt,
)


def test_custom_prompt_builder_returns_text(tmp_path: Path) -> None:
    prompt = build_custom_prompt("demo", ["bug_hunt"])
    assert "demo" in prompt
    assert "Найти вероятные ошибки" in prompt

    output = tmp_path / "CUSTOM_PROMPT.md"
    write_custom_prompt(output, "demo", ["bug_hunt"])
    assert output.read_text(encoding="utf-8") == prompt


def test_dashboard_writes_html_content(tmp_path: Path) -> None:
    reports = tmp_path / "reports"
    reports.mkdir()
    (reports / "22_project_health_report.md").write_text(
        "Overall score: **87/100**\n", encoding="utf-8"
    )
    (reports / "27_archive_plan.md").write_text("Split planned: `False`\n", encoding="utf-8")
    (reports / "28_export_plan.md").write_text("# План\n\n- ok\n", encoding="utf-8")
    (reports / "06_security_scan.json").write_text(
        '{"summary": {"total_findings": 2, "potential_secrets": 1}}', encoding="utf-8"
    )
    output = reports / "REPORT_DASHBOARD.html"

    write_html_dashboard(reports, output)

    html = output.read_text(encoding="utf-8")
    assert "<!doctype html>" in html
    assert "87/100" in html
    assert "Панель отчётов проекта" in html


def test_ai_context_folder_writes_codex_prompt(tmp_path: Path) -> None:
    copied = tmp_path / "copied"
    source = tmp_path / "source"
    copied.mkdir()
    source.mkdir()
    (copied / "main.py").write_text("print('ok')\n", encoding="utf-8")
    inventory = {
        "files": [copied / "main.py"],
        "dirs": [],
        "stack": {"backend": ["Python"]},
        "sizes": [(copied / "main.py", 12)],
        "total_size": 12,
    }
    output = tmp_path / "AI_CONTEXT"

    write_ai_context_folder(copied, source, output, inventory)

    prompt = (output / "09_PROMPT_FOR_CODEX.md").read_text(encoding="utf-8")
    assert "Промпт для анализа проекта" in prompt
