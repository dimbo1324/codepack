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
<html lang="ru">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Панель отчётов проекта</title>
<style>
:root {{
  color-scheme: light dark;
  --bg: #f6f7fb;
  --fg: #1f2937;
  --muted: #667085;
  --card: #ffffff;
  --border: #d9dee8;
  --accent: #0f766e;
}}
@media (prefers-color-scheme: dark) {{
  :root {{
    --bg: #111827;
    --fg: #e5e7eb;
    --muted: #a3aab8;
    --card: #182033;
    --border: #2b364d;
    --accent: #5eead4;
  }}
}}
* {{ box-sizing: border-box; }}
body {{
  margin: 0;
  padding: 32px;
  background: var(--bg);
  color: var(--fg);
  font-family: "Segoe UI", Arial, sans-serif;
  line-height: 1.5;
}}
main {{
  max-width: 1180px;
  margin: 0 auto;
}}
h1 {{
  margin: 0 0 6px;
  font-size: 30px;
}}
.meta {{
  margin: 0 0 24px;
  color: var(--muted);
}}
.cards {{
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 14px;
  margin-bottom: 24px;
}}
.card {{
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  background: var(--card);
}}
.label {{
  color: var(--muted);
  font-size: 13px;
}}
.value {{
  margin-top: 8px;
  font-size: 28px;
  font-weight: 700;
}}
.card p {{
  margin: 8px 0 0;
  color: var(--muted);
}}
.grid {{
  display: grid;
  grid-template-columns: minmax(220px, 320px) minmax(0, 1fr);
  gap: 20px;
}}
nav, section.panel {{
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 18px;
  background: var(--card);
}}
nav ul {{
  margin: 0;
  padding-left: 20px;
}}
nav li {{
  margin: 7px 0;
}}
a {{
  color: var(--accent);
}}
pre {{
  overflow: auto;
  padding: 14px;
  border-radius: 8px;
  background: rgba(127, 127, 127, 0.12);
  white-space: pre-wrap;
}}
@media (max-width: 760px) {{
  body {{ padding: 18px; }}
  .grid {{ grid-template-columns: 1fr; }}
}}
</style>
</head>
<body>
<main>
<h1>Панель отчётов проекта</h1>
<p class="meta">Сформировано: {html.escape(human_now())}</p>
<div class="cards">
{card_html}
</div>
<div class="grid">
<nav>
<h2>Отчёты</h2>
<ul>
{link_html}
</ul>
</nav>
<section class="panel">
<h2>План экспорта</h2>
<pre>{export_plan_excerpt or "Отчёт ещё не создан."}</pre>
<h2>Здоровье проекта</h2>
<pre>{health_excerpt or "Отчёт ещё не создан."}</pre>
</section>
</div>
</main>
</body>
</html>
""",
        encoding="utf-8",
        newline="\n",
    )
