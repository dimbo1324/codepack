from __future__ import annotations

import re
from collections import Counter
from pathlib import Path
from typing import Any

from ...utils.inventory import write_key_value_lines
from ...utils.path_utils import rel_display
from ...utils.text_utils import format_bytes
from ...utils.time_utils import human_now

def write_project_summary_report(
    copied_root: Path,
    source_root: Path,
    output_file: Path,
    inventory: dict[str, Any],
) -> None:
    files: list[Path] = inventory["files"]
    dirs: list[Path] = inventory["dirs"]
    sizes: list[tuple[Path, int]] = inventory["sizes"]
    stack: dict[str, list[str]] = inventory["stack"]

    readmes = [p for p in files if p.name.lower().startswith("readme")]
    licenses = [p for p in files if p.name.lower().startswith("license")]
    env_files = [p for p in files if p.name.lower().startswith(".env")]
    test_files = [
        p
        for p in files
        if re.search(
            r"(?i)(^|[._/-])(test|spec)([._/-]|$)", str(p.relative_to(copied_root))
        )
    ]
    ci_files = list((copied_root / ".github" / "workflows").glob("*.yml")) + list(
        (copied_root / ".github" / "workflows").glob("*.yaml")
    )
    docker_files = [p for p in files if p.name.lower().startswith("dockerfile")]
    compose_files = [
        p
        for p in files
        if p.name.lower()
        in {"docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"}
    ]

    largest = sorted(sizes, key=lambda item: item[1], reverse=True)[:15]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Project Summary ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("=" * 100 + "\n\n")
        write_key_value_lines(
            out,
            {
                "Source root": str(source_root),
                "Copied project root": str(copied_root),
                "Project name": copied_root.name,
                "Total files": f"{len(files):,}",
                "Total folders": f"{len(dirs):,}",
                "Total copied size": format_bytes(int(inventory["total_size"])),
                "README present": "yes" if readmes else "no",
                "LICENSE present": "yes" if licenses else "no",
                "Tests detected": (
                    f"yes ({len(test_files):,} files)" if test_files else "no"
                ),
                "Docker detected": "yes" if docker_files or compose_files else "no",
                "CI/CD detected": (
                    f"yes ({len(ci_files):,} GitHub Actions workflows)"
                    if ci_files
                    else "no"
                ),
                ".env-like files": f"{len(env_files):,}",
            },
        )

        out.write("\n--- Detected stack ---\n")
        for group, values in stack.items():
            out.write(f"{group}: {', '.join(values) if values else 'not detected'}\n")

        out.write("\n--- Detected languages by file count ---\n")
        language_count: Counter[str] = inventory["language_count"]
        if language_count:
            for language, count in language_count.most_common(30):
                size = inventory["language_size"][language]
                out.write(
                    f"{language:<28} {count:>8,} files   {format_bytes(size):>12}\n"
                )
        else:
            out.write("No known language extensions detected.\n")

        out.write("\n--- Largest files ---\n")
        for path, size in largest:
            out.write(f"{format_bytes(size):>12}  {rel_display(path, copied_root)}\n")

        out.write("\n--- Useful next checks ---\n")
        if not readmes:
            out.write("- Add or update README with setup/run instructions.\n")
        if not licenses:
            out.write("- Add LICENSE if this project will be shared externally.\n")
        if env_files:
            out.write("- Review .env-like files before sharing the export.\n")
        if not test_files:
            out.write(
                "- No obvious test files found; consider adding smoke/unit tests.\n"
            )
        if not ci_files:
            out.write(
                "- No GitHub Actions workflow detected; consider adding CI for checks.\n"
            )
