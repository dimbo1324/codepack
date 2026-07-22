from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parents[2]
MODULES_DIR = ROOT_DIR / ".ai"
AGENTS_MD = ROOT_DIR / "AGENTS.md"

# Codex reads at most 32 KiB of project instructions; stay below with headroom.
SIZE_LIMIT_KIB = 30.0

# A module may opt out of being inlined with a marker on one of its first lines:
#     <!-- tier: extended -->
# Extended modules are listed as an on-demand index instead of being embedded.
TIER_MARKER = re.compile(r"<!--\s*tier:\s*(inline|extended)\s*-->", re.IGNORECASE)

BANNER = """\
<!--
СГЕНЕРИРОВАННЫЙ ФАЙЛ — НЕ РЕДАКТИРОВАТЬ.
Источник правды: .ai/universal/*.md и .ai/project/*.md.
Отредактируйте модуль, затем выполните: python dev_tools_scripts_runner.py sync-agents
-->

# codepack — рабочие заметки для Codex

Этот файл — точка входа Codex. Он собран из общих модулей правил в `.ai/`, поэтому
Codex и Claude Code всегда следуют одинаковым правилам. Более поздние секции
переопределяют более ранние; явная инструкция владельца в текущем диалоге
переопределяет всё.
"""


def module_files() -> list[Path]:
    files: list[Path] = []
    for group in ("universal", "project"):
        group_dir = MODULES_DIR / group
        if not group_dir.is_dir():
            print(f"ERROR: missing module directory: {group_dir}", file=sys.stderr)
            raise SystemExit(2)
        files.extend(sorted(group_dir.glob("*.md")))
    if not files:
        print("ERROR: no rule modules found under .ai/", file=sys.stderr)
        raise SystemExit(2)
    return files


def module_tier(body: str) -> str:
    match = TIER_MARKER.search("\n".join(body.splitlines()[:5]))
    return match.group(1).lower() if match else "inline"


def module_title(body: str) -> str:
    for line in body.splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return "(без заголовка)"


def module_digest(body: str) -> str:
    """The `> **Суть.** ...` line an extended module carries so its essence stays inline."""
    match = re.search(r"^>\s*\*\*Суть\.\*\*\s*(.+)$", body, re.MULTILINE)
    return match.group(1).strip() if match else ""


def render() -> str:
    parts = [BANNER]
    extended: list[tuple[str, str, str]] = []

    for path in module_files():
        body = path.read_text(encoding="utf-8").strip()
        rel = path.relative_to(ROOT_DIR).as_posix()
        if module_tier(body) == "extended":
            extended.append((rel, module_title(body), module_digest(body)))
            continue
        parts.append(f"<!-- module: {rel} -->\n\n{body}")

    if extended:
        lines = [
            "<!-- module index: extended -->",
            "",
            "# Модули, подключаемые по требованию",
            "",
            "Эти правила действуют наравне с остальными, но не встроены целиком из-за",
            "бюджета размера инструкций. Ниже — их суть; когда задача касается модуля,",
            "прочитайте файл целиком. Это обязанность, а не рекомендация.",
            "",
        ]
        for rel, title, digest in extended:
            lines.append(f"## {title}")
            lines.append("")
            lines.append(f"Файл: `{rel}`")
            if digest:
                lines.append("")
                lines.append(digest)
            lines.append("")
        parts.append("\n".join(lines).rstrip())

    return "\n\n---\n\n".join(parts) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Assemble AGENTS.md from the .ai/ rule modules.")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)

    content = render()
    size_kib = len(content.encode("utf-8")) / 1024
    if size_kib > SIZE_LIMIT_KIB:
        print(
            f"ERROR: assembled AGENTS.md is {size_kib:.1f} KiB; the budget is "
            f"{SIZE_LIMIT_KIB:.0f} KiB. Tighten a module or mark a situational one "
            "with <!-- tier: extended -->.",
            file=sys.stderr,
        )
        return 2

    current = AGENTS_MD.read_text(encoding="utf-8") if AGENTS_MD.exists() else ""
    if args.check:
        if current != content:
            print(
                "AGENTS.md is out of sync with .ai/ modules. "
                "Run: python dev_tools_scripts_runner.py sync-agents",
                file=sys.stderr,
            )
            return 1
        print(f"AGENTS.md is in sync with .ai/ modules ({size_kib:.1f} KiB).")
        return 0

    if current == content:
        print(f"AGENTS.md already up to date ({size_kib:.1f} KiB).")
        return 0
    AGENTS_MD.write_text(content, encoding="utf-8", newline="\n")
    print(f"AGENTS.md regenerated ({size_kib:.1f} KiB of {SIZE_LIMIT_KIB:.0f} KiB budget).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
