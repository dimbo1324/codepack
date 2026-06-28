"""AI Prompt files report: writes ready-made prompt templates into the AI_PROMPTS/ folder.

PROMPTS maps output filename to Markdown prompt content; an empty dict means no prompt
files are generated, which is intentional while the prompt library is under construction.
"""

from __future__ import annotations

from pathlib import Path

from ...utils.time_utils import human_now

PROMPTS: dict[str, str] = {
}


def write_ai_prompt_files(_copied_root: Path, output_dir: Path) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    for filename, content in PROMPTS.items():
        header = f"<!-- Generated: {human_now()} -->\n\n"
        (output_dir / filename).write_text(header + content.strip() + "\n", encoding="utf-8")