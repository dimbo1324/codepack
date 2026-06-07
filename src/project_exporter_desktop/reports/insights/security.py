from __future__ import annotations

import re
from pathlib import Path

from ...constants import SECRET_KEY_PATTERN, SECRET_PATTERNS, SENSITIVE_FILENAMES, SENSITIVE_SUFFIXES
from ...utils.inventory import iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, redact_secrets, should_consider_text_file
from ...utils.time_utils import human_now

RISKY_CODE_PATTERNS: tuple[tuple[str, re.Pattern[str], str], ...] = (
    ("python-eval", re.compile(r"\beval\s*\("), "Python eval() can execute arbitrary code."),
    ("python-exec", re.compile(r"\bexec\s*\("), "Python exec() can execute arbitrary code."),
    ("subprocess-shell-true", re.compile(r"subprocess\.[A-Za-z_]+\([^\n]*shell\s*=\s*True"), "subprocess with shell=True increases command-injection risk."),
    ("pickle-load", re.compile(r"\bpickle\.loads?\s*\("), "pickle can execute arbitrary code when loading untrusted data."),
    ("unsafe-yaml-load", re.compile(r"yaml\.load\s*\("), "yaml.load can be unsafe without a safe loader."),
    ("js-eval", re.compile(r"\beval\s*\("), "JavaScript eval() can execute arbitrary code."),
    ("inner-html", re.compile(r"\.innerHTML\s*="), "innerHTML assignment can create XSS risk with untrusted input."),
    ("document-write", re.compile(r"document\.write\s*\("), "document.write is usually unsafe and hard to control."),
    ("local-storage-token", re.compile(r"localStorage\.[A-Za-z]*(?:setItem|getItem)\([^\n]*(token|jwt|secret|password)", re.IGNORECASE), "Storing sensitive auth data in localStorage can be risky."),
)


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
    risky_code: list[tuple[Path, int, str, str, str]] = []

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
            for code, pattern, explanation in RISKY_CODE_PATTERNS:
                if pattern.search(line):
                    risky_code.append((path, line_number, code, explanation, line.strip()[:240]))

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Enhanced Security Scan ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(
            "This is a heuristic scan, not a professional secret scanner or SAST engine. Values are redacted.\n"
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

        out.write("\n--- Risky code patterns ---\n")
        if risky_code:
            for path, line_number, code, explanation, line in risky_code[:500]:
                out.write(f"{rel_display(path, copied_root)}:{line_number}: [{code}] {explanation}\n")
                out.write(f"    {redact_secrets(line)}\n")
            if len(risky_code) > 500:
                out.write(f"... and {len(risky_code) - 500:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Recommended actions before sharing ---\n")
        out.write("- Review all .env-like files.\n")
        out.write(
            "- Rotate any secret that may have been committed or exported by mistake.\n"
        )
        out.write("- Prefer .env.example with placeholder values for shared exports.\n")
        out.write("- Manually review any risky code pattern before treating it as a vulnerability.\n")
