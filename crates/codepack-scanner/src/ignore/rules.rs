//! `should_skip_dir`/`should_skip_file`, ported from legacy `ExportIgnoreRules` with
//! its exact precedence. Both are pure, standalone predicates over a relative path —
//! they do not know whether the caller already pruned an ancestor directory, which is
//! why `should_skip_file` re-derives its own ancestor-directory-rule check (legacy
//! relies on this for the "always-include a file without including its whole excluded
//! parent directory" case — see the module tests).

use std::path::Path;

use super::ExportIgnoreRules;

impl ExportIgnoreRules {
    /// Returns `(skip, reason)`. `reason` is empty when `skip` is `false`.
    pub fn should_skip_dir(&self, relative_dir: &Path) -> (bool, String) {
        let rel = normalize_rel(relative_dir);
        if rel.is_empty()
            || self.is_inside_always_included_dir(&rel)
            || self.has_always_included_file_descendant(&rel)
        {
            return (false, String::new());
        }
        let name = basename_lower(relative_dir);
        match self.excluded_dir_match(&rel, &name) {
            Some(rule) => (true, format!(".exportignore/custom directory rule: {rule}")),
            None => (false, String::new()),
        }
    }

    /// Returns `(skip, reason)`. `reason` is empty when `skip` is `false`.
    pub fn should_skip_file(&self, relative_path: &Path) -> (bool, String) {
        let rel = normalize_rel(relative_path);
        let name = basename_lower(relative_path);
        let parent_rel = normalize_rel(relative_path.parent().unwrap_or(Path::new("")));

        if self.is_always_included_file(&rel, &name)
            || self.is_inside_always_included_dir(&parent_rel)
        {
            return (false, String::new());
        }

        let suffix = extension_lower(relative_path);
        if !suffix.is_empty() && self.excluded_extensions.contains(&suffix) {
            return (true, format!("custom extension rule: .{suffix}"));
        }

        for pattern in &self.excluded_files {
            if pattern.matcher.matches(&rel) || pattern.matcher.matches(&name) {
                return (
                    true,
                    format!(".exportignore/custom file rule: {}", pattern.raw),
                );
            }
        }

        // Ancestor walk, deepest-first (mirrors Python's `Path.parents`), independent
        // of whether `should_skip_dir` would have pruned that ancestor: an
        // always-included file's own directory can survive `should_skip_dir` while a
        // *sibling* file inside that same surviving directory is still excluded here.
        let dir_components: Vec<&str> = if parent_rel.is_empty() {
            Vec::new()
        } else {
            parent_rel.split('/').collect()
        };
        for len in (1..=dir_components.len()).rev() {
            let ancestor = dir_components[..len].join("/");
            let ancestor_name = dir_components[len - 1];
            if let Some(rule) = self.excluded_dir_match(&ancestor, ancestor_name) {
                return (true, format!(".exportignore/custom directory rule: {rule}"));
            }
        }

        (false, String::new())
    }

    fn is_always_included_file(&self, rel: &str, name: &str) -> bool {
        self.always_include_files
            .iter()
            .any(|pattern| pattern.matcher.matches(rel) || pattern.matcher.matches(name))
    }

    fn is_inside_always_included_dir(&self, rel: &str) -> bool {
        if rel.is_empty() {
            return false;
        }
        let parts: Vec<&str> = rel.split('/').collect();
        self.always_include_dirs.iter().any(|include_dir| {
            include_dir == rel
                || rel.starts_with(&format!("{include_dir}/"))
                || parts.contains(&include_dir.as_str())
        })
    }

    fn has_always_included_file_descendant(&self, rel: &str) -> bool {
        if rel.is_empty() {
            return !self.always_include_files.is_empty();
        }
        let prefix = format!("{rel}/");
        self.always_include_files
            .iter()
            .any(|pattern| pattern.raw == rel || pattern.raw.starts_with(&prefix))
    }

    fn excluded_dir_match(&self, rel: &str, name: &str) -> Option<&str> {
        self.excluded_dirs
            .iter()
            .find(|rule| {
                rule.raw == name
                    || rule.raw == rel
                    || rel.starts_with(&format!("{}/", rule.raw))
                    || rule.matcher.matches(rel)
            })
            .map(|rule| rule.raw.as_str())
    }
}

/// A lone `.` (or an empty path) collapses to the empty string, representing "the
/// project root" — the intended meaning of legacy's `if not rel:` root check. Python's
/// own `str(Path('.'))` renders as `"."`, not `""`, which would make that check
/// unreachable through a `Path`-typed argument; collapsing here implements the
/// documented intent (root is never skipped) rather than the accidental literal.
pub(super) fn normalize_rel(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let trimmed = s.trim_matches('/');
    if trimmed == "." {
        String::new()
    } else {
        trimmed.to_lowercase()
    }
}

fn basename_lower(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn extension_lower(path: &Path) -> String {
    path.extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::super::ScanOptions;
    use super::*;

    fn rules_from(content: &str) -> ExportIgnoreRules {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".exportignore"), content).unwrap();
        ExportIgnoreRules::from_project_and_config(dir.path(), &ScanOptions::default())
    }

    #[test]
    fn dir_file_extension_and_negation_all_work_together() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".exportignore"),
            "private/\n*.log\n*.db\n!important.log\n!docs/\n",
        )
        .unwrap();
        let opts = ScanOptions {
            custom_excluded_extensions: vec!["sqlite".to_string()],
            ..ScanOptions::default()
        };
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &opts);

        assert!(rules.should_skip_dir(Path::new("private")).0);
        assert!(rules.should_skip_file(Path::new("logs/app.log")).0);
        assert!(!rules.should_skip_file(Path::new("important.log")).0);
        assert!(rules.should_skip_file(Path::new("data/main.sqlite")).0);
        assert!(!rules.should_skip_dir(Path::new("docs")).0);
    }

    #[test]
    fn always_include_file_keeps_parent_directory_traversable_but_not_its_siblings() {
        let rules = rules_from("private/\n!private/keep/allowed.txt\n");

        assert!(!rules.should_skip_dir(Path::new("private")).0);
        assert!(!rules.should_skip_dir(Path::new("private/keep")).0);
        assert!(rules.should_skip_dir(Path::new("private/drop")).0);
        assert!(
            !rules
                .should_skip_file(Path::new("private/keep/allowed.txt"))
                .0
        );
        // Always-include-vs-exclusion precedence test 1: an unrelated sibling file in
        // the same surviving directory is still excluded via the file-level ancestor
        // check — always-include rescues only what it names, not the whole directory.
        assert!(
            rules
                .should_skip_file(Path::new("private/keep/other.txt"))
                .0
        );
    }

    #[test]
    fn inline_comments_are_removed_only_after_whitespace() {
        let rules = rules_from("logs/*.log # generated logs\nhash#file.txt\n");
        assert!(rules.should_skip_file(Path::new("logs/app.log")).0);
        assert!(rules.should_skip_file(Path::new("hash#file.txt")).0);
        assert!(!rules.should_skip_file(Path::new("hash")).0);
    }

    #[test]
    fn empty_project_has_no_rules() {
        let rules = rules_from("");
        assert!(!rules.should_skip_file(Path::new("any/file.py")).0);
    }

    #[test]
    fn negation_overrides_glob_exclusion() {
        // Always-include-vs-exclusion precedence test 2 (the positive case): an
        // explicit `!` always-include rescues a file from a `.exportignore` glob rule.
        let rules = rules_from("*.log\n!important.log\n");
        assert!(rules.should_skip_file(Path::new("debug.log")).0);
        assert!(!rules.should_skip_file(Path::new("important.log")).0);
    }

    #[test]
    fn directory_rule_excludes_its_files_but_not_siblings() {
        let rules = rules_from("build/\n");
        assert!(rules.should_skip_file(Path::new("build/output.o")).0);
        assert!(!rules.should_skip_file(Path::new("src/main.py")).0);
        assert!(rules.should_skip_dir(Path::new("build")).0);
    }

    #[test]
    fn double_star_file_glob_matches_nested_paths() {
        let rules = rules_from("**/node_modules/**\n");
        assert!(
            rules
                .should_skip_file(Path::new("frontend/node_modules/react/index.js"))
                .0
        );
        assert!(!rules.should_skip_file(Path::new("frontend/src/index.js")).0);
    }

    #[test]
    fn programmatic_always_include_overrides_programmatic_exclusion() {
        let mut rules = rules_from("*.secret\n");
        rules.add_always_include_file("shared.secret");
        assert!(!rules.should_skip_file(Path::new("shared.secret")).0);
        assert!(rules.should_skip_file(Path::new("other.secret")).0);
    }

    #[test]
    fn conflicting_always_include_wins_over_file_rule() {
        let mut rules = ExportIgnoreRules::default();
        rules.add_file_rule("config.yml");
        rules.add_always_include_file("config.yml");
        assert!(!rules.should_skip_file(Path::new("config.yml")).0);
    }

    #[test]
    fn path_with_backslash_is_normalized() {
        let mut rules = ExportIgnoreRules::default();
        rules.add_file_rule("src\\utils\\helper.py");
        assert!(rules.should_skip_file(Path::new("src/utils/helper.py")).0);
    }

    #[test]
    fn unicode_filenames_are_matched() {
        let rules = rules_from("секрет.txt\n");
        assert!(rules.should_skip_file(Path::new("секрет.txt")).0);
        assert!(!rules.should_skip_file(Path::new("публичный.txt")).0);
    }

    #[test]
    fn always_include_dir_protects_children() {
        let mut rules = ExportIgnoreRules::default();
        rules.add_dir_rule("internal");
        rules.add_always_include_dir("internal/public");
        assert!(!rules.should_skip_dir(Path::new("internal/public")).0);
    }

    #[test]
    fn extension_rule_without_dot_in_config_still_matches() {
        let dir = tempfile::tempdir().unwrap();
        let opts = ScanOptions {
            custom_excluded_extensions: vec!["pyc".to_string()],
            ..ScanOptions::default()
        };
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &opts);
        assert!(
            rules
                .should_skip_file(Path::new("cache/__pycache__/module.pyc"))
                .0
        );
    }

    #[test]
    fn unknown_extension_is_not_skipped() {
        let rules = ExportIgnoreRules::default();
        assert!(!rules.should_skip_file(Path::new("file.xyz123unknown")).0);
    }
}
