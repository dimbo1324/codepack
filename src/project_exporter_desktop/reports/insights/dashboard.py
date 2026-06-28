from __future__ import annotations

import html
import json
import re
from pathlib import Path

from ...utils.time_utils import human_now


def _read_text(path: Path, limit: int = 30_000) -> str:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
        return text[:limit]
    except Exception:
        return ""


def _extract_overall_score(health_report: Path) -> str:
    text = _read_text(health_report, 5_000)
    match = re.search(r"Overall score:\s*\*\*(\d+/100)\*\*", text)
    return match.group(1) if match else "n/a"


def _extract_security_summary(security_json: Path) -> dict[str, int]:
    try:
        data = json.loads(security_json.read_text(encoding="utf-8"))
        summary = data.get("summary", {})
        return {str(k): int(v) for k, v in summary.items()}
    except Exception:
        return {}


def write_html_dashboard(reports_dir: Path, output_file: Path) -> None:
    health = reports_dir / "22_project_health_report.md"
    archive_plan = reports_dir / "27_archive_plan.md"
    export_plan = reports_dir / "28_export_plan.md"
    security_json = reports_dir / "06_security_scan.json"
    security_summary = _extract_security_summary(security_json)
    score = _extract_overall_score(health)

    cards = [
        ("Project health", score, "Heuristic maintainability/AI-readiness score"),
        (
            "Security findings",
            str(security_summary.get("total_findings", "n/a")),
            "JSON + SARIF are generated",
        ),
        (
            "Potential secrets",
            str(security_summary.get("potential_secrets", "n/a")),
            "Redacted secret-like lines",
        ),
        (
            "Archive strategy",
            "split" if "Split planned: `True`" in _read_text(archive_plan, 2000) else "single",
            "Logical ZIP planning",
        ),
    ]
    links = [
        ("Project Profile", "../00_project_profile.json"),
        ("Export Plan", "28_export_plan.md"),
        ("Archive Plan", "27_archive_plan.md"),
        ("Security Scan", "06_security_scan.txt"),
        ("Security JSON", "06_security_scan.json"),
        ("Security SARIF", "06_security_scan.sarif"),
        ("Architecture Map", "24_architecture_map.md"),
        ("Health Report", "22_project_health_report.md"),
        ("AI Context", "AI_CONTEXT/00_PROJECT_OVERVIEW.md"),
        ("Custom Prompt", "AI_PROMPTS/CUSTOM_PROMPT.md"),
    ]
    card_html = "\n".join(
        f'<section class="card"><div class="label">{html.escape(title)}</div><div class="value">{html.escape(value)}</div><p>{html.escape(desc)}</p></section>'
        for title, value, desc in cards
    )
    link_html = "\n".join(
        f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>' for label, href in links
    )
    export_plan_excerpt = html.escape(_read_text(export_plan, 6000))
    health_excerpt = html.escape(_read_text(health, 6000))
    output_file.write_text(
        encoding="utf-8",
        newline="\n",
    )