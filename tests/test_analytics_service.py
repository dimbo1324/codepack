from __future__ import annotations

import json
from pathlib import Path

from project_exporter_desktop.services.analytics_service import analyze_project


def test_analytics_collects_stack_languages_dependencies_and_risks(tmp_path: Path) -> None:
    (tmp_path / "package.json").write_text(
        json.dumps({"dependencies": {"left-pad": "*"}}, ensure_ascii=False),
        encoding="utf-8",
    )
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "main.py").write_text("print('one')\nprint('two')\n", encoding="utf-8")
    (tmp_path / ".env").write_text("API_KEY=secret", encoding="utf-8")

    report = analyze_project(tmp_path, frozenset())

    assert report.stack == "Node.js"
    assert report.total_files >= 3
    assert report.total_loc >= 2
    assert any(item.name == "Python" and item.loc >= 2 for item in report.languages)
    assert any(item.name == "left-pad" and item.warning for item in report.dependencies)
    assert any(item.path == ".env" for item in report.risks)