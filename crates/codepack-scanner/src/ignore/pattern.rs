//! Hand-rolled `fnmatch.translate`-equivalent glob matcher backed by `regex`.
//!
//! Deliberately not the `ignore`/`globset` crates — see the workspace `Cargo.toml`
//! comment on the `regex` dependency for the rationale. `*` matches any run of
//! characters including `/` (Python's `fnmatch` has no path-aware `**`; a literal
//! `**` is just two consecutive `*` translated independently, which is exactly what
//! legacy relies on for patterns like `**/node_modules/**`).

use regex::Regex;

#[derive(Debug, Clone)]
pub struct GlobPattern {
    raw: String,
    regex: Regex,
}

impl GlobPattern {
    pub fn new(pattern: &str) -> Self {
        let translated = translate(pattern);
        // `translate` only ever emits `regex::escape`d literals, `.`/`.*` for `?`/`*`,
        // and well-formed (possibly negated) bracket classes with a literal `\[`
        // fallback for an unterminated `[` — every code path produces syntactically
        // valid regex, so compilation cannot fail.
        let regex = Regex::new(&translated).expect("translate() always yields valid regex syntax");
        Self {
            raw: pattern.to_string(),
            regex,
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }
}

pub(crate) fn has_glob_metachars(pattern: &str) -> bool {
    pattern.contains(['*', '?', '['])
}

fn translate(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let n = chars.len();
    let mut out = String::from("(?s)^");
    let mut i = 0;

    while i < n {
        let c = chars[i];
        i += 1;
        match c {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            '[' => {
                let mut j = i;
                if j < n && (chars[j] == '!' || chars[j] == '^') {
                    j += 1;
                }
                if j < n && chars[j] == ']' {
                    j += 1;
                }
                while j < n && chars[j] != ']' {
                    j += 1;
                }
                if j >= n {
                    out.push_str("\\[");
                    continue;
                }
                let mut class = String::new();
                let mut k = i;
                if chars[k] == '!' {
                    class.push('^');
                    k += 1;
                } else if chars[k] == '^' {
                    class.push_str("\\^");
                    k += 1;
                }
                while k < j {
                    let ch = chars[k];
                    if ch == '\\' {
                        class.push_str("\\\\");
                    } else {
                        class.push(ch);
                    }
                    k += 1;
                }
                out.push('[');
                out.push_str(&class);
                out.push(']');
                i = j + 1;
            }
            other => out.push_str(&regex::escape(&other.to_string())),
        }
    }
    out.push('$');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_matches_any_run_of_characters() {
        let pattern = GlobPattern::new("*.log");
        assert!(pattern.matches("debug.log"));
        assert!(!pattern.matches("debug.logger"));
    }

    #[test]
    fn double_star_matches_across_separators() {
        let pattern = GlobPattern::new("**/node_modules/**");
        assert!(pattern.matches("frontend/node_modules/react/index.js"));
        assert!(!pattern.matches("frontend/src/index.js"));
    }

    #[test]
    fn question_mark_matches_single_character() {
        let pattern = GlobPattern::new("file?.txt");
        assert!(pattern.matches("file1.txt"));
        assert!(!pattern.matches("file12.txt"));
    }

    #[test]
    fn bracket_class_matches_listed_characters() {
        let pattern = GlobPattern::new("file[123].txt");
        assert!(pattern.matches("file1.txt"));
        assert!(!pattern.matches("file4.txt"));
    }

    #[test]
    fn negated_bracket_class_excludes_listed_characters() {
        let pattern = GlobPattern::new("file[!123].txt");
        assert!(pattern.matches("file4.txt"));
        assert!(!pattern.matches("file1.txt"));
    }

    #[test]
    fn unterminated_bracket_is_treated_as_literal() {
        let pattern = GlobPattern::new("weird[file.txt");
        assert!(pattern.matches("weird[file.txt"));
    }

    #[test]
    fn special_regex_characters_are_escaped() {
        let pattern = GlobPattern::new("a+b.txt");
        assert!(pattern.matches("a+b.txt"));
        assert!(!pattern.matches("aab.txt"));
    }

    #[test]
    fn empty_pattern_only_matches_empty_string() {
        let pattern = GlobPattern::new("");
        assert!(pattern.matches(""));
        assert!(!pattern.matches("x"));
    }

    #[test]
    fn has_glob_metachars_detects_wildcards() {
        assert!(has_glob_metachars("*.egg-info"));
        assert!(has_glob_metachars("file?.txt"));
        assert!(has_glob_metachars("[abc]"));
        assert!(!has_glob_metachars("plain-name"));
    }
}
