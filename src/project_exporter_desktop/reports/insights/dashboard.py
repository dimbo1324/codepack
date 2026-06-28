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
        f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Project Export Dashboard</title>
<style>
:root {{ color-scheme: dark light; font-family: Inter, Segoe UI, Arial, sans-serif; }}
body {{ margin: 0; padding: 32px; background: #101214; color: #ece7dc; }}
header {{ max-width: 1120px; margin: 0 auto 24px; }}
h1 {{ margin: 0 0 8px; font-size: 32px; }}
.muted {{ color: #a9aa9f; }}
.grid {{ max-width: 1120px; margin: 0 auto; display: grid; grid-template-columns: repeat(auto-fit,minmax(220px,1fr)); gap: 14px; }}
.card {{ border: 1px solid #3a3d37; border-radius: 16px; padding: 18px; background: #171a1d; box-shadow: 0 10px 30px rgba(0,0,0,.2); }}
.label {{ color: #a9aa9f; font-size: 13px; text-transform: uppercase; letter-spacing: .08em; }}
.value {{ font-size: 34px; font-weight: 800; margin: 8px 0; }}
.panel {{ max-width: 1120px; margin: 18px auto; border: 1px solid #3a3d37; border-radius: 16px; padding: 18px; background: #171a1d; }}
a {{ color: #b8d49b; }}
pre {{ overflow: auto; white-space: pre-wrap; background: #0c0e10; padding: 14px; border-radius: 12px; }}
@media (prefers-color-scheme: light) {{ body {{ background:#f7f4ed; color:#181a1d; }} .card,.panel {{ background:#fffaf0; border-color:#d7d0c2; }} pre {{ background:#f0eadf; }} }}
</style>
</head>
<body>
<header>
<h1>Project Export Dashboard</h1>
<p class="muted">Generated: {html.escape(human_now())}. Open this file locally after export to navigate the package quickly.</p>
</header>
<main>
<div class="grid">{card_html}</div>
<section class="panel"><h2>Quick links</h2><ul>{link_html}</ul></section>
<section class="panel"><h2>Export plan excerpt</h2><pre>{export_plan_excerpt}</pre></section>
<section class="panel"><h2>Health report excerpt</h2><pre>{health_excerpt}</pre></section>
</main>
</body>
</html>
""",
        encoding="utf-8",
        newline="\n",
    )