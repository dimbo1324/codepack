from __future__ import annotations

from pathlib import Path

from ...constants import SECRET_KEY_PATTERN, SECRET_PATTERNS, SENSITIVE_FILENAMES, SENSITIVE_SUFFIXES
from ...utils.inventory import iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, redact_secrets, should_consider_text_file
from ...utils.time_utils import human_now

def redacted_line(line: str) -> str:
    line = redact_secrets(line)
    if SECRET_KEY_PATTERN.search(line):
        if "=" in line:
            key = line.split("=", 1)[0].strip()
            return f"{key}=<REDACTED>"
        if ":" in line:
            key = line.split(":", 1)[0].strip()
            return f"{key}: <REDACTED>"
        return "<REDACTED_SECRET_LINE>"
    return line.strip()

def write_security_scan_report(
    copied_root: Path, output_file: Path, max_bytes_per_file: int
) -> None:
    suspicious_files: list[Path] = []
    suspicious_lines: list[tuple[Path, int, str]] = []

    for path in iter_project_files(copied_root):
        name = path.name.lower()
        suffix = path.suffix.lower().lstrip(".")
        if (
            name in SENSITIVE_FILENAMES
            or suffix in SENSITIVE_SUFFIXES
            or name.startswith(".env")
        ):
            suspicious_files.append(path)

        if not should_consider_text_file(path):
            continue
        try:
            if path.stat().st_size > max_bytes_per_file:
                continue
        except Exception:
            continue

        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue
        for line_number, line in enumerate(text.splitlines(), start=1):
            if SECRET_KEY_PATTERN.search(line) or any(
                pattern.search(line) for pattern in SECRET_PATTERNS
            ):
                suspicious_lines.append((path, line_number, redacted_line(line)))

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Basic Security Scan ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(
            "This is a heuristic scan, not a professional secret scanner. Values are redacted.\n"
        )
        out.write("=" * 100 + "\n\n")

        out.write("--- Sensitive-looking files ---\n")
        if suspicious_files:
            for path in sorted(suspicious_files, key=lambda p: str(p).lower())[:300]:
                out.write(f"{rel_display(path, copied_root)}\n")
            if len(suspicious_files) > 300:
                out.write(f"... and {len(suspicious_files) - 300:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Potential secret-like lines ---\n")
        if suspicious_lines:
            for path, line_number, line in suspicious_lines[:500]:
                out.write(f"{rel_display(path, copied_root)}:{line_number}: {line}\n")
            if len(suspicious_lines) > 500:
                out.write(f"... and {len(suspicious_lines) - 500:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Recommended actions before sharing ---\n")
        out.write("- Review all .env-like files.\n")
        out.write(
            "- Rotate any secret that may have been committed or exported by mistake.\n"
        )
        out.write("- Prefer .env.example with placeholder values for shared exports.\n")
