# Insight report generator: reads the copied project and writes one focused analysis artifact into reports/insights.

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

from ...constants import (
    SECRET_KEY_PATTERN,
    SECRET_PATTERNS,
    SENSITIVE_FILENAMES,
    SENSITIVE_SUFFIXES,
)
from ...utils.inventory import iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, redact_secrets, should_consider_text_file
from ...utils.time_utils import human_now

RISKY_CODE_PATTERNS: tuple[tuple[str, re.Pattern[str], str, str], ...] = (
    (
        "critical",
        "python-eval",
        re.compile(r"\beval\s*\("),
        "Python eval() can execute arbitrary code.",
    ),
    (
        "critical",
        "python-exec",
        re.compile(r"\bexec\s*\("),
        "Python exec() can execute arbitrary code.",
    ),
    (
        "high",
        "subprocess-shell-true",
        re.compile(r"subprocess\.[A-Za-z_]+\([^\n]*shell\s*=\s*True"),
        "subprocess with shell=True increases command-injection risk.",
    ),
    (
        "critical",
        "pickle-load",
        re.compile(r"\bpickle\.loads?\s*\("),
        "pickle can execute arbitrary code when loading untrusted data.",
    ),
    (
        "high",
        "unsafe-yaml-load",
        re.compile(r"yaml\.load\s*\("),
        "yaml.load can be unsafe without a safe loader.",
    ),
    (
        "high",
        "js-eval",
        re.compile(r"\beval\s*\("),
        "JavaScript eval() can execute arbitrary code.",
    ),
    (
        "medium",
        "inner-html",
        re.compile(r"\.innerHTML\s*="),
        "innerHTML assignment can create XSS risk with untrusted input.",
    ),
    (
        "medium",
        "document-write",
        re.compile(r"document\.write\s*\("),
        "document.write is usually unsafe and hard to control.",
    ),
    (
        "high",
        "local-storage-token",
        re.compile(
            r"localStorage\.[A-Za-z]*(?:setItem|getItem)\([^\n]*(token|jwt|secret|password)",
            re.IGNORECASE,
        ),
        "Storing sensitive auth data in localStorage can be risky.",
    ),
)

_PRIVATE_KEY_RE = re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")
_ASSIGNMENT_SECRET_RE = re.compile(
    r"(?i)\b(api[_-]?key|secret|token|password|pass|private[_-]?key|database[_-]?url|jwt[_-]?secret)\b\s*[:=]"
)
_SCANNER_CODE_HINTS = (
    "SECRET_PATTERN",
    "SECRET_KEY_PATTERN",
    "redact_secrets",
    "_REDACT_KEYWORDS",
    "_SCAN_KEYWORDS",
)


@dataclass(slots=True)
class SecretFinding:
    path: Path
    line_number: int
    confidence: str
    line: str


@dataclass(slots=True)
class RiskyCodeFinding:
    path: Path
    line_number: int
    severity: str
    code: str
    explanation: str
    line: str


def _secret_confidence(line: str) -> str | None:
    if any(hint in line for hint in _SCANNER_CODE_HINTS):
        return None
    if _PRIVATE_KEY_RE.search(line):
        return "critical"
    if any(pattern.search(line) for pattern in SECRET_PATTERNS):
        return "high"
    if _ASSIGNMENT_SECRET_RE.search(line):
        return "medium"
    if SECRET_KEY_PATTERN.search(line) and not line.strip().startswith(("#", "//", "*")):
        return "low"
    return None


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


def _collect_security_findings(
    copied_root: Path, max_bytes_per_file: int | None
) -> tuple[list[tuple[str, Path]], list[SecretFinding], list[RiskyCodeFinding]]:
    suspicious_files: list[tuple[str, Path]] = []
    suspicious_lines: list[SecretFinding] = []
    risky_code: list[RiskyCodeFinding] = []

    for path in iter_project_files(copied_root):
        name = path.name.lower()
        suffix = path.suffix.lower().lstrip(".")
        if name in SENSITIVE_FILENAMES or suffix in SENSITIVE_SUFFIXES or name.startswith(".env"):
            severity = (
                "critical"
                if name.startswith(".env") or suffix in {"key", "pem", "p12", "pfx"}
                else "high"
            )
            suspicious_files.append((severity, path))

        if not should_consider_text_file(path):
            continue
        try:
            if max_bytes_per_file is not None and path.stat().st_size > max_bytes_per_file:
                continue
        except Exception:
            continue

        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue
        for line_number, line in enumerate(text.splitlines(), start=1):
            confidence = _secret_confidence(line)
            if confidence is not None:
                suspicious_lines.append(
                    SecretFinding(path, line_number, confidence, redacted_line(line))
                )
            for severity, code, pattern, explanation in RISKY_CODE_PATTERNS:
                if pattern.search(line):
                    risky_code.append(
                        RiskyCodeFinding(
                            path,
                            line_number,
                            severity,
                            code,
                            explanation,
                            redact_secrets(line.strip()[:240]),
                        )
                    )

    order = {"critical": 0, "high": 1, "medium": 2, "low": 3}
    suspicious_files.sort(key=lambda item: (order.get(item[0], 9), str(item[1]).casefold()))
    suspicious_lines.sort(
        key=lambda item: (
            order.get(item.confidence, 9),
            str(item.path).casefold(),
            item.line_number,
        )
    )
    risky_code.sort(
        key=lambda item: (order.get(item.severity, 9), str(item.path).casefold(), item.line_number)
    )
    return suspicious_files, suspicious_lines, risky_code


def _write_security_json(
    output_file: Path,
    copied_root: Path,
    suspicious_files: list[tuple[str, Path]],
    suspicious_lines: list[SecretFinding],
    risky_code: list[RiskyCodeFinding],
) -> None:
    findings: list[dict[str, object]] = []
    for severity, path in suspicious_files:
        findings.append(
            {
                "type": "sensitive_file",
                "severity": severity,
                "confidence": "high",
                "file": rel_display(path, copied_root),
                "line": None,
                "rule": "sensitive_filename",
                "message": "Sensitive-looking filename or suffix.",
            }
        )
    for item in suspicious_lines:
        findings.append(
            {
                "type": "potential_secret",
                "severity": item.confidence,
                "confidence": item.confidence,
                "file": rel_display(item.path, copied_root),
                "line": item.line_number,
                "rule": "secret_like_line",
                "message": item.line,
            }
        )
    for item in risky_code:
        findings.append(
            {
                "type": "risky_code",
                "severity": item.severity,
                "confidence": "medium",
                "file": rel_display(item.path, copied_root),
                "line": item.line_number,
                "rule": item.code,
                "message": item.explanation,
            }
        )
    data = {
        "schema_version": "1.0",
        "generated_at": human_now(),
        "summary": {
            "sensitive_files": len(suspicious_files),
            "potential_secrets": len(suspicious_lines),
            "risky_code": len(risky_code),
            "total_findings": len(findings),
        },
        "findings": findings,
    }
    output_file.write_text(
        json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8", newline="\n"
    )


def _write_security_sarif(json_file: Path, sarif_file: Path) -> None:
    data = json.loads(json_file.read_text(encoding="utf-8"))
    rules: dict[str, dict[str, object]] = {}
    results: list[dict[str, object]] = []
    for finding in data.get("findings", []):
        rule_id = str(finding.get("rule", "unknown"))
        rules.setdefault(
            rule_id,
            {
                "id": rule_id,
                "name": rule_id,
                "shortDescription": {"text": str(finding.get("message", rule_id))[:120]},
            },
        )
        file_path = str(finding.get("file", "")).lstrip(".\\/").replace("\\", "/")
        location: dict[str, object] = {"artifactLocation": {"uri": file_path}}
        if finding.get("line"):
            location["region"] = {"startLine": int(finding["line"])}
        results.append(
            {
                "ruleId": rule_id,
                "level": "error" if finding.get("severity") in {"critical", "high"} else "warning",
                "message": {"text": str(finding.get("message", ""))},
                "locations": [{"physicalLocation": location}],
            }
        )
    sarif = {
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "Project Exporter Security Scanner",
                        "rules": list(rules.values()),
                    }
                },
                "results": results,
            }
        ],
    }
    sarif_file.write_text(
        json.dumps(sarif, ensure_ascii=False, indent=2), encoding="utf-8", newline="\n"
    )


def write_security_scan_report(
    copied_root: Path, output_file: Path, max_bytes_per_file: int | None
) -> None:
    suspicious_files, suspicious_lines, risky_code = _collect_security_findings(
        copied_root, max_bytes_per_file
    )
    json_file = output_file.with_suffix(".json")
    sarif_file = output_file.with_suffix(".sarif")
    _write_security_json(json_file, copied_root, suspicious_files, suspicious_lines, risky_code)
    _write_security_sarif(json_file, sarif_file)

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Enhanced Security Scan v3 ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(
            "This is a heuristic scanner, not a professional secret scanner or SAST engine. Values are redacted.\n"
        )
        out.write(
            "Machine-readable outputs are written next to this report: 06_security_scan.json and 06_security_scan.sarif.\n"
        )
        out.write("Findings are grouped by confidence/severity to reduce noise.\n")
        out.write("=" * 100 + "\n\n")

        out.write("--- Sensitive-looking files ---\n")
        if suspicious_files:
            for severity, path in suspicious_files[:300]:
                out.write(f"[{severity}] {rel_display(path, copied_root)}\n")
            if len(suspicious_files) > 300:
                out.write(f"... and {len(suspicious_files) - 300:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Potential secret-like lines ---\n")
        if suspicious_lines:
            for item in suspicious_lines[:500]:
                out.write(
                    f"[{item.confidence}] {rel_display(item.path, copied_root)}:{item.line_number}: {item.line}\n"
                )
            if len(suspicious_lines) > 500:
                out.write(f"... and {len(suspicious_lines) - 500:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Risky code patterns ---\n")
        if risky_code:
            for item in risky_code[:500]:
                out.write(
                    f"[{item.severity}] {rel_display(item.path, copied_root)}:{item.line_number}: [{item.code}] {item.explanation}\n"
                )
                out.write(f"    {item.line}\n")
            if len(risky_code) > 500:
                out.write(f"... and {len(risky_code) - 500:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Recommended actions before sharing ---\n")
        out.write("- Keep Safe Export mode enabled for external AI/code-review handoffs.\n")
        out.write("- Review all critical/high findings before sharing the archive.\n")
        out.write("- Rotate any secret that may have been committed or exported by mistake.\n")
        out.write("- Prefer `.env.example` with placeholder values for shared exports.\n")
        out.write(
            "- Manually review risky code findings before treating them as vulnerabilities.\n"
        )
