"""Asserts a `codepack export --json` report is well-formed and describes a real bundle.

Used by the CI step that satisfies stage S10's acceptance criterion. Kept as a file
rather than inlined in the workflow so it can be run by hand when the CI step fails,
and so the shell quoting stays trivial.
"""

import json
import pathlib
import sys


def main(report_path: str) -> int:
    report = json.loads(pathlib.Path(report_path).read_text(encoding="utf-8"))

    problems = []
    if report.get("schema_version") != 1:
        problems.append(f"schema_version: expected 1, got {report.get('schema_version')!r}")
    if report.get("command") != "export":
        problems.append(f"command: expected 'export', got {report.get('command')!r}")
    if report.get("successful") is not True:
        problems.append(f"successful: expected true, got {report.get('successful')!r}")

    archive = report.get("result_path")
    if not archive:
        problems.append("result_path: no archive was reported")
    elif not pathlib.Path(archive).is_file():
        problems.append(f"result_path: {archive} does not exist")

    if problems:
        print("headless export report is not valid:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        print(json.dumps(report, indent=2)[:2000], file=sys.stderr)
        return 1

    print(f"headless export ok: {report['files_copied']} file(s) -> {archive}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: check_export_report.py <export.json>", file=sys.stderr)
        raise SystemExit(2)
    raise SystemExit(main(sys.argv[1]))
