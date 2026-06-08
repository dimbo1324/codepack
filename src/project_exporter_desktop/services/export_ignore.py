from __future__ import annotations

import fnmatch
from dataclasses import dataclass, field
from pathlib import Path


def _clean_rule(line: str) -> str:
    line = line.strip()
    if not line or line.startswith('#'):
        return ''
    # Inline comments are treated as comments only when separated by whitespace.
    if ' #' in line:
        line = line.split(' #', 1)[0].strip()
    return line.replace('\\', '/')


def _normalise_path(path: Path | str) -> str:
    text = str(path).replace('\\', '/').strip('/')
    return text.casefold()


@dataclass(slots=True)
class ExportIgnoreRules:
    """Rules loaded from .exportignore and GUI settings.

    The syntax intentionally mirrors the common subset of .gitignore:
    - blank lines and # comments are ignored;
    - trailing slash means directory rule;
    - wildcard rules use fnmatch;
    - leading ! creates an explicit include override for custom ignore rules.

    Built-in safety exclusions are deliberately applied outside this class so an
    "always include" rule cannot silently leak secrets in Safe Export mode.
    """

    excluded_dirs: set[str] = field(default_factory=set)
    excluded_files: list[str] = field(default_factory=list)
    excluded_extensions: set[str] = field(default_factory=set)
    always_include_files: list[str] = field(default_factory=list)
    always_include_dirs: set[str] = field(default_factory=set)
    source_file: Path | None = None
    loaded_rules: list[str] = field(default_factory=list)

    @classmethod
    def from_project_and_config(
        cls,
        source_root: Path,
        excluded_files: list[str] | None = None,
        excluded_extensions: list[str] | None = None,
        always_include_files: list[str] | None = None,
        always_include_dirs: list[str] | None = None,
    ) -> ExportIgnoreRules:
        rules = cls()
        ignore_file = source_root / '.exportignore'
        if ignore_file.exists() and ignore_file.is_file():
            rules.source_file = ignore_file
            try:
                rules._load_lines(ignore_file.read_text(encoding='utf-8', errors='replace').splitlines())
            except Exception:
                # A malformed/locked .exportignore must not break the export.
                pass

        for item in excluded_files or []:
            rules.add_file_rule(item)
        for item in excluded_extensions or []:
            rules.add_extension_rule(item)
        for item in always_include_files or []:
            rules.add_always_include_file(item)
        for item in always_include_dirs or []:
            rules.add_always_include_dir(item)
        return rules

    def _load_lines(self, lines: list[str]) -> None:
        for raw_line in lines:
            rule = _clean_rule(raw_line)
            if not rule:
                continue
            self.loaded_rules.append(rule)
            include = rule.startswith('!')
            if include:
                rule = rule[1:].strip()
            if not rule:
                continue
            if rule.endswith('/'):
                target = rule.rstrip('/')
                if include:
                    self.add_always_include_dir(target)
                else:
                    self.add_dir_rule(target)
                continue
            if include:
                self.add_always_include_file(rule)
            else:
                self.add_file_rule(rule)

    def add_dir_rule(self, value: str) -> None:
        value = _normalise_path(value)
        if value:
            self.excluded_dirs.add(value)

    def add_file_rule(self, value: str) -> None:
        value = value.strip().replace('\\', '/')
        if value:
            self.excluded_files.append(value.casefold())

    def add_extension_rule(self, value: str) -> None:
        value = value.strip().lstrip('.').casefold()
        if value:
            self.excluded_extensions.add(value)

    def add_always_include_file(self, value: str) -> None:
        value = value.strip().replace('\\', '/')
        if value:
            self.always_include_files.append(value.casefold())

    def add_always_include_dir(self, value: str) -> None:
        value = _normalise_path(value)
        if value:
            self.always_include_dirs.add(value)

    def is_always_included_file(self, relative_path: Path) -> bool:
        rel = _normalise_path(relative_path)
        name = relative_path.name.casefold()
        return any(
            fnmatch.fnmatch(rel, pattern) or fnmatch.fnmatch(name, pattern)
            for pattern in self.always_include_files
        )

    def is_inside_always_included_dir(self, relative_dir_or_file: Path) -> bool:
        rel = _normalise_path(relative_dir_or_file)
        parts = rel.split('/') if rel else []
        return any(
            include_dir == rel
            or rel.startswith(include_dir + '/')
            or include_dir in parts
            for include_dir in self.always_include_dirs
        )

    def should_skip_dir(self, relative_dir: Path) -> tuple[bool, str]:
        rel = _normalise_path(relative_dir)
        if not rel or self.is_inside_always_included_dir(relative_dir):
            return False, ''
        name = relative_dir.name.casefold()
        for rule in self.excluded_dirs:
            if rule == name or rule == rel or rel.startswith(rule + '/') or fnmatch.fnmatch(rel, rule):
                return True, f'.exportignore/custom directory rule: {rule}'
        return False, ''

    def should_skip_file(self, relative_path: Path) -> tuple[bool, str]:
        if self.is_always_included_file(relative_path) or self.is_inside_always_included_dir(relative_path.parent):
            return False, ''
        rel = _normalise_path(relative_path)
        name = relative_path.name.casefold()
        suffix = relative_path.suffix.casefold().lstrip('.')
        if suffix and suffix in self.excluded_extensions:
            return True, f'custom extension rule: .{suffix}'
        for pattern in self.excluded_files:
            if fnmatch.fnmatch(rel, pattern) or fnmatch.fnmatch(name, pattern):
                return True, f'.exportignore/custom file rule: {pattern}'
        return False, ''

    def to_dict(self) -> dict[str, object]:
        return {
            'source_file': str(self.source_file) if self.source_file else None,
            'loaded_rules': self.loaded_rules,
            'excluded_dirs': sorted(self.excluded_dirs),
            'excluded_files': list(self.excluded_files),
            'excluded_extensions': sorted(self.excluded_extensions),
            'always_include_files': list(self.always_include_files),
            'always_include_dirs': sorted(self.always_include_dirs),
        }
