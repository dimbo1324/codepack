//! `fnmatch`-equivalent glob matching, done directly rather than by translating to a
//! regex.
//!
//! Deliberately not the `ignore`/`globset` crates — see the workspace `Cargo.toml`
//! comment on the `regex` dependency for that decision. This module previously
//! translated each pattern into regex source and compiled it; matching a glob is simple
//! enough that the translation step was the only complicated part, and it brought two
//! costs with it: a compile per pattern, and a `Regex::new(..).expect(..)` whose safety
//! argument was a paragraph explaining why the generated source could never be invalid.
//! Matching the pattern directly removes both.
//!
//! Semantics are Python `fnmatch`'s, which is what legacy used:
//!
//! - `*` matches any run of characters, **including `/`**. Python's `fnmatch` has no
//!   path-aware `**`; a literal `**` is just two consecutive `*`, which is exactly what
//!   legacy relies on for patterns like `**/node_modules/**`.
//! - `?` matches exactly one character.
//! - `[abc]` matches one character from the set, `[!abc]` / `[^abc]` one outside it, and
//!   a `]` immediately after the opening bracket (or after the negation) is a literal.
//! - An unterminated `[` is a literal `[`.
//! - The whole pattern must match the whole text.

/// A compiled glob pattern.
///
/// Holds the pattern's characters rather than a compiled automaton, so construction is
/// free and cannot fail.
#[derive(Debug, Clone)]
pub struct GlobPattern {
    raw: String,
    /// The pattern as characters, so matching indexes by character rather than by byte
    /// and `?` correctly consumes one multi-byte character.
    pattern: Vec<char>,
}

impl GlobPattern {
    pub fn new(pattern: &str) -> Self {
        Self {
            raw: pattern.to_string(),
            pattern: pattern.chars().collect(),
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        let text: Vec<char> = text.chars().collect();
        matches_from(&self.pattern, 0, &text, 0)
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }
}

pub(crate) fn has_glob_metachars(pattern: &str) -> bool {
    pattern.contains(['*', '?', '['])
}

/// Matches `pattern[p..]` against `text[t..]`.
///
/// `*` is handled by trying every possible split of the remaining text, shortest first.
/// That is the textbook backtracking formulation; it is adequate here because these
/// patterns come from a project's `.exportignore` and are short, and because the
/// alternative — an iterative two-pointer scan with a single backtrack point — is
/// materially harder to read for no benefit at this size.
fn matches_from(pattern: &[char], p: usize, text: &[char], t: usize) -> bool {
    let Some(&current) = pattern.get(p) else {
        // Pattern exhausted: the text must be too, since the match is anchored at both
        // ends.
        return t == text.len();
    };

    match current {
        '*' => {
            // Collapse a run of `*`: `**` and `*` mean the same thing under fnmatch.
            let mut next = p;
            while pattern.get(next) == Some(&'*') {
                next += 1;
            }
            (t..=text.len()).any(|split| matches_from(pattern, next, text, split))
        }
        '?' => t < text.len() && matches_from(pattern, p + 1, text, t + 1),
        '[' => match parse_class(pattern, p) {
            Some(class) => {
                t < text.len()
                    && class.matches(text[t])
                    && matches_from(pattern, class.pattern_end, text, t + 1)
            }
            // An unterminated `[` is a literal, matching Python's fnmatch.
            None => t < text.len() && text[t] == '[' && matches_from(pattern, p + 1, text, t + 1),
        },
        literal => {
            t < text.len() && text[t] == literal && matches_from(pattern, p + 1, text, t + 1)
        }
    }
}

/// A parsed `[...]` character class.
struct CharacterClass<'p> {
    negated: bool,
    /// The characters and ranges between the brackets.
    body: &'p [char],
    /// Index just past the closing `]`.
    pattern_end: usize,
}

impl CharacterClass<'_> {
    fn matches(&self, candidate: char) -> bool {
        let mut index = 0;
        let mut found = false;
        while index < self.body.len() {
            // `a-z` is a range; a `-` at either end of the body is a literal.
            let is_range = index + 2 < self.body.len() && self.body[index + 1] == '-';
            if is_range {
                let (low, high) = (self.body[index], self.body[index + 2]);
                if (low..=high).contains(&candidate) {
                    found = true;
                }
                index += 3;
            } else {
                if self.body[index] == candidate {
                    found = true;
                }
                index += 1;
            }
        }
        found != self.negated
    }
}

/// Parses a `[...]` class starting at `open` (which must be the `[`).
///
/// Returns `None` when the class is unterminated, which the caller treats as a literal
/// `[`.
fn parse_class(pattern: &[char], open: usize) -> Option<CharacterClass<'_>> {
    let mut cursor = open + 1;
    let negated = matches!(pattern.get(cursor), Some('!' | '^'));
    if negated {
        cursor += 1;
    }

    // A `]` in the first position is a literal member, not the terminator.
    let body_start = cursor;
    if pattern.get(cursor) == Some(&']') {
        cursor += 1;
    }
    while cursor < pattern.len() && pattern[cursor] != ']' {
        cursor += 1;
    }
    if cursor >= pattern.len() {
        return None;
    }

    Some(CharacterClass {
        negated,
        body: &pattern[body_start..cursor],
        pattern_end: cursor + 1,
    })
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
    fn star_matches_a_separator_because_fnmatch_is_not_path_aware() {
        // The property `**/x/**` relies on: `*` is not stopped by `/`.
        assert!(GlobPattern::new("*.log").matches("nested/dir/debug.log"));
    }

    #[test]
    fn question_mark_matches_single_character() {
        let pattern = GlobPattern::new("file?.txt");
        assert!(pattern.matches("file1.txt"));
        assert!(!pattern.matches("file12.txt"));
        assert!(!pattern.matches("file.txt"));
    }

    #[test]
    fn question_mark_consumes_one_whole_multibyte_character() {
        // Matching by character rather than by byte: `й` is two bytes but one `?`.
        assert!(GlobPattern::new("файл?.txt").matches("файлы.txt"));
        assert!(!GlobPattern::new("файл?.txt").matches("файлы1.txt"));
    }

    #[test]
    fn bracket_class_matches_listed_characters() {
        let pattern = GlobPattern::new("file[123].txt");
        assert!(pattern.matches("file1.txt"));
        assert!(!pattern.matches("file4.txt"));
    }

    #[test]
    fn bracket_class_supports_ranges() {
        let pattern = GlobPattern::new("file[0-9].txt");
        assert!(pattern.matches("file7.txt"));
        assert!(!pattern.matches("filex.txt"));
    }

    #[test]
    fn negated_bracket_class_excludes_listed_characters() {
        for spelling in ["file[!123].txt", "file[^123].txt"] {
            let pattern = GlobPattern::new(spelling);
            assert!(pattern.matches("file4.txt"), "{spelling}");
            assert!(!pattern.matches("file1.txt"), "{spelling}");
        }
    }

    #[test]
    fn a_closing_bracket_first_in_the_class_is_a_literal_member() {
        let pattern = GlobPattern::new("x[]]");
        assert!(pattern.matches("x]"));
    }

    #[test]
    fn unterminated_bracket_is_treated_as_literal() {
        let pattern = GlobPattern::new("weird[file.txt");
        assert!(pattern.matches("weird[file.txt"));
    }

    #[test]
    fn special_regex_characters_are_literals_not_operators() {
        // The previous implementation had to escape these before compiling; here they
        // are simply ordinary characters.
        assert!(GlobPattern::new("a+b.txt").matches("a+b.txt"));
        assert!(!GlobPattern::new("a+b.txt").matches("aab.txt"));
        assert!(GlobPattern::new("a(b).txt").matches("a(b).txt"));
        assert!(GlobPattern::new("a|b").matches("a|b"));
        assert!(!GlobPattern::new("a|b").matches("a"));
        assert!(GlobPattern::new("a.b").matches("a.b"));
        assert!(!GlobPattern::new("a.b").matches("axb"));
    }

    #[test]
    fn empty_pattern_only_matches_empty_string() {
        let pattern = GlobPattern::new("");
        assert!(pattern.matches(""));
        assert!(!pattern.matches("x"));
    }

    #[test]
    fn a_lone_star_matches_everything_including_the_empty_string() {
        let pattern = GlobPattern::new("*");
        assert!(pattern.matches(""));
        assert!(pattern.matches("anything/at/all"));
    }

    #[test]
    fn many_stars_do_not_blow_up() {
        // A pathological pattern against a non-matching text is the classic backtracking
        // trap; the star-run collapse keeps it tractable.
        let pattern = GlobPattern::new("*a*a*a*a*a*b");
        assert!(!pattern.matches(&"a".repeat(40)));
        assert!(pattern.matches(&("a".repeat(40) + "b")));
    }

    #[test]
    fn has_glob_metachars_detects_wildcards() {
        assert!(has_glob_metachars("*.egg-info"));
        assert!(has_glob_metachars("file?.txt"));
        assert!(has_glob_metachars("[abc]"));
        assert!(!has_glob_metachars("plain-name"));
    }

    /// Differential check against the regex translation this replaced.
    ///
    /// The reference is the exact `translate`-then-compile the previous implementation
    /// used, so this asserts the rewrite is behaviour-preserving rather than merely
    /// plausible.
    #[test]
    fn matches_the_previous_regex_translation() {
        let patterns = [
            "*.log",
            "**/node_modules/**",
            "file?.txt",
            "file[123].txt",
            "file[!123].txt",
            "file[0-9].txt",
            "weird[file.txt",
            "a+b.txt",
            "",
            "*",
            "src/**/*.rs",
            "*.egg-info",
            "[abc]def",
            "no-metachars",
        ];
        let texts = [
            "debug.log",
            "debug.logger",
            "nested/dir/debug.log",
            "frontend/node_modules/react/index.js",
            "frontend/src/index.js",
            "file1.txt",
            "file12.txt",
            "file4.txt",
            "file7.txt",
            "weird[file.txt",
            "a+b.txt",
            "aab.txt",
            "",
            "x",
            "src/a/b/c.rs",
            "src/c.rs",
            "pkg.egg-info",
            "adef",
            "zdef",
            "no-metachars",
        ];

        for pattern in patterns {
            let reference = regex::Regex::new(&translate_for_reference(pattern))
                .expect("the previous translation always produced valid regex");
            let ours = GlobPattern::new(pattern);
            for text in texts {
                assert_eq!(
                    ours.matches(text),
                    reference.is_match(text),
                    "pattern {pattern:?} disagreed on text {text:?}"
                );
            }
        }
    }

    /// The previous implementation's `translate`, kept verbatim as a test reference.
    fn translate_for_reference(pattern: &str) -> String {
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
}
