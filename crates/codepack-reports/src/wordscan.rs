//! Word-bounded keyword scanning, done with plain string operations.
//!
//! Several reports ask the same question — "does this text mention any of these fixed
//! words, as whole words?" — which was previously answered by a regex per report. Three
//! problems followed from that:
//!
//! 1. **The marker sets drifted.** `07_todo_fixme` and `17_code_quality` scanned eight
//!    markers; `23_refactoring_opportunities` scanned five, so a file whose only marker
//!    was `BUG` counted as flagged in two reports and clean in the third.
//! 2. **One pattern was silently wrong.** `r"(?i)\bTODO|FIXME|HACK|XXX|DEPRECATED\b"`
//!    reads as if every alternative were word-bounded, but `|` binds looser than
//!    concatenation, so it parses as `(\bTODO)|(FIXME)|(HACK)|(XXX)|(DEPRECATED\b)` —
//!    only the first and last alternative are bounded at all. `PREFIXME` matched.
//! 3. **The regexes were rebuilt per file.** Two call sites constructed their pattern
//!    inside the per-file loop, recompiling it once per scanned file.
//!
//! A regex is the wrong tool for a fixed set of literal words: the engine's generality
//! buys nothing, and expressing the boundary rule as data rather than pattern syntax
//! makes it impossible to write down incorrectly. [`find_word`] is a direct
//! implementation of what `\b(A|B|C)\b` means, so the semantics are unchanged where the
//! original pattern was written correctly, and corrected where it was not.

/// Legacy `TODO_PATTERN` (`constants.py`): the maintenance markers every report agrees
/// on. Kept as one list so the three reports that scan for markers cannot disagree
/// about what counts as one.
pub(crate) const CODE_MARKERS: &[&str] = &[
    "TODO",
    "FIXME",
    "HACK",
    "XXX",
    "BUG",
    "TEMP",
    "REFACTOR",
    "DEPRECATED",
];

/// Heavyweight infrastructure imports that are a smell inside a UI module — the
/// question `17_code_quality` and `23_refactoring_opportunities` both ask of files whose
/// path mentions "ui".
///
/// `os.walk` carries a `.`, which is not a word character, so the boundary rule applies
/// to the run as a whole: `os.walk` matches inside `for os.walk(` but not inside
/// `myos.walker`.
pub(crate) const UI_INFRA_SYMBOLS: &[&str] = &[
    "shutil",
    "zipfile",
    "subprocess",
    "os.walk",
    "threading",
    "Queue",
];

/// Symbols whose presence together in one file makes it a "mixed concern" candidate in
/// `17_code_quality` — UI toolkits, I/O and persistence appearing side by side.
///
/// `open(` deliberately carries its opening parenthesis: the bare word `open` is far too
/// common to be a signal. See [`is_word_bounded`] for what boundary means for an entry
/// ending in punctuation.
pub(crate) const MIXED_CONCERN_SYMBOLS: &[&str] = &[
    "tkinter",
    "subprocess",
    "requests",
    "fetch",
    "sql",
    "database",
    "threading",
    "Queue",
    "open(",
    "write_text",
    "read_text",
];

/// A word character, matching the `\w` class the replaced patterns relied on.
///
/// `regex` compiles `\w` and `\b` **Unicode-aware** by default, so a Cyrillic letter is
/// a word character to it just as an ASCII one is. Restricting this to ASCII would make
/// `путьTODOпуть` report a marker that the original pattern did not — caught by
/// [`tests::matches_the_regex_engine_on_every_word_in_every_set`], which is exactly the
/// kind of quiet divergence a hand-written replacement has to rule out rather than
/// assume away.
fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// True when both edges of `haystack[start..end]` sit on a `\b` word boundary.
///
/// `\b` asserts a *transition*: it holds at a position when exactly one side of it is a
/// word character. That distinction matters for entries that do not begin or end with a
/// word character — `open(` is bounded on its right only when a word character follows
/// the parenthesis, which is why `open(path` qualifies and `open()` does not. Treating
/// `\b` as "the neighbour is not a word character" would silently invert that case.
///
/// `start`/`end` are byte offsets. Every entry in the sets above is ASCII, and an ASCII
/// byte never occurs inside a multi-byte UTF-8 sequence, so a match can only ever land
/// on real character boundaries and the neighbour lookups below cannot split a
/// character.
fn is_word_bounded(haystack: &str, start: usize, end: usize) -> bool {
    let boundary_at = |left: Option<char>, right: Option<char>| {
        left.is_some_and(is_word_char) != right.is_some_and(is_word_char)
    };

    let before = haystack[..start].chars().next_back();
    let matched = &haystack[start..end];
    let after = haystack[end..].chars().next();

    boundary_at(before, matched.chars().next()) && boundary_at(matched.chars().next_back(), after)
}

/// Finds the first whole-word, case-insensitive occurrence of any entry in `words`,
/// returning the entry that matched (not the text as it was spelled in `haystack`).
///
/// Returning the canonical spelling from `words` rather than the matched slice is what
/// lets callers report `TODO` for a source line that said `todo`, which is the
/// behavior the previous `to_uppercase()` on the captured group produced.
///
/// Candidate positions are found by ASCII-case-insensitive byte comparison (every entry
/// in the sets above is ASCII), then filtered by [`is_word_bounded`], which applies the
/// Unicode-aware boundary rule the original patterns used.
pub(crate) fn find_word<'w>(haystack: &str, words: &[&'w str]) -> Option<&'w str> {
    let bytes = haystack.as_bytes();
    let mut best: Option<(usize, &'w str)> = None;

    for word in words {
        let needle = word.as_bytes();
        if needle.is_empty() || needle.len() > bytes.len() {
            continue;
        }
        for start in 0..=bytes.len() - needle.len() {
            let end = start + needle.len();
            if !bytes[start..end].eq_ignore_ascii_case(needle) {
                continue;
            }
            if !is_word_bounded(haystack, start, end) {
                continue;
            }
            // The earliest bounded occurrence across the whole set wins, matching how a
            // regex alternation scans left to right.
            if best.is_none_or(|(best_start, _)| start < best_start) {
                best = Some((start, word));
            }
            // Any later occurrence of this same word is further right, so it can never
            // improve on the one just recorded.
            break;
        }
    }

    best.map(|(_, word)| word)
}

/// True when `haystack` mentions any entry in `words` as a whole word. Equivalent to
/// `find_word(..).is_some()`, named for the call sites that only need the yes/no answer.
pub(crate) fn contains_word(haystack: &str, words: &[&str]) -> bool {
    find_word(haystack, words).is_some()
}

/// Every distinct entry of `words` that `haystack` mentions as a whole word, in the
/// order the set declares them.
///
/// Counting *distinct entries* rather than distinct matched spellings is deliberate:
/// the caller uses the count as a "how many separate concerns does this file touch"
/// signal, where `SQL` and `sql` are one concern, not two. The regex this replaced
/// collected matched text case-sensitively and would have counted them separately.
pub(crate) fn matching_words<'w>(haystack: &str, words: &[&'w str]) -> Vec<&'w str> {
    words
        .iter()
        .filter(|word| !word.is_empty() && contains_word(haystack, std::slice::from_ref(word)))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_marker_as_a_whole_word() {
        assert_eq!(find_word("// TODO: fix this", CODE_MARKERS), Some("TODO"));
        assert_eq!(find_word("# fixme later", CODE_MARKERS), Some("FIXME"));
    }

    #[test]
    fn returns_the_canonical_spelling_not_the_matched_text() {
        // Callers report the marker kind; a lowercase mention still reports as `TODO`,
        // reproducing the previous `captures.to_uppercase()` behavior.
        assert_eq!(find_word("todo", CODE_MARKERS), Some("TODO"));
        assert_eq!(find_word("ToDo", CODE_MARKERS), Some("TODO"));
    }

    #[test]
    fn rejects_a_marker_embedded_in_a_longer_word() {
        // The defect this module was written to fix: the old refactoring pattern read
        // as `(\bTODO)|(FIXME)|…`, so only the first and last alternative were bounded
        // and `PREFIXME` matched on `FIXME`.
        assert_eq!(find_word("PREFIXME", CODE_MARKERS), None);
        assert_eq!(find_word("prefixme_handler", CODE_MARKERS), None);
        assert_eq!(find_word("TODOS", CODE_MARKERS), None);
        assert_eq!(find_word("my_todo_list", CODE_MARKERS), None);
        assert_eq!(find_word("XXXX", CODE_MARKERS), None);
    }

    #[test]
    fn accepts_a_marker_bounded_by_punctuation_or_string_edges() {
        assert_eq!(find_word("TODO", CODE_MARKERS), Some("TODO"));
        assert_eq!(find_word("(TODO)", CODE_MARKERS), Some("TODO"));
        assert_eq!(find_word("/*XXX*/", CODE_MARKERS), Some("XXX"));
        assert_eq!(find_word("done.TODO", CODE_MARKERS), Some("TODO"));
    }

    #[test]
    fn underscore_counts_as_a_word_character_just_like_the_regex_w_class() {
        assert_eq!(find_word("_TODO", CODE_MARKERS), None);
        assert_eq!(find_word("TODO_", CODE_MARKERS), None);
        assert_eq!(find_word("-TODO-", CODE_MARKERS), Some("TODO"));
    }

    #[test]
    fn reports_the_leftmost_marker_when_several_appear() {
        // Matches how a regex alternation scans: position wins over set order.
        assert_eq!(find_word("HACK then TODO", CODE_MARKERS), Some("HACK"));
        assert_eq!(find_word("TODO then HACK", CODE_MARKERS), Some("TODO"));
    }

    #[test]
    fn every_legacy_marker_is_detected() {
        // Guards the set itself: the three reports that scan markers previously carried
        // three different lists, so a file whose only marker was BUG counted as flagged
        // in two of them and clean in the third.
        for marker in CODE_MARKERS {
            let line = format!("// {marker}: something");
            assert_eq!(
                find_word(&line, CODE_MARKERS),
                Some(*marker),
                "marker {marker} was not detected"
            );
        }
    }

    #[test]
    fn ui_infra_symbols_respect_boundaries_including_the_dotted_entry() {
        assert!(contains_word("import subprocess", UI_INFRA_SYMBOLS));
        assert!(contains_word("for root in os.walk(path)", UI_INFRA_SYMBOLS));
        assert!(!contains_word("myos.walker(path)", UI_INFRA_SYMBOLS));
        assert!(!contains_word("subprocessing", UI_INFRA_SYMBOLS));
        assert!(!contains_word("a_threading_helper", UI_INFRA_SYMBOLS));
    }

    #[test]
    fn ui_infra_symbols_are_case_sensitive_in_spirit_but_matched_case_insensitively() {
        // The original patterns had no `(?i)` flag, but every symbol here is a literal
        // module name that only ever appears in one casing in real source. Matching
        // case-insensitively can only widen detection for a heuristic advisory report,
        // never narrow it, and keeps one scanning rule instead of two.
        assert!(contains_word("import SubProcess", UI_INFRA_SYMBOLS));
    }

    #[test]
    fn empty_inputs_are_handled_without_panicking() {
        assert_eq!(find_word("", CODE_MARKERS), None);
        assert_eq!(find_word("anything", &[]), None);
        assert_eq!(find_word("anything", &[""]), None);
    }

    #[test]
    fn a_non_ascii_letter_is_a_word_character_so_it_suppresses_the_boundary() {
        // Unicode-aware `\b`, matching the regex engine: a Cyrillic letter is a word
        // character, so `вTODOв` is one identifier-like run and reports no marker, while
        // a space-separated mention does. An ASCII-only boundary rule would flag the
        // first case and quietly over-report on non-English source.
        assert_eq!(
            find_word("комментарий TODO здесь", CODE_MARKERS),
            Some("TODO")
        );
        assert_eq!(find_word("вTODOв", CODE_MARKERS), None);
        assert_eq!(find_word("нет маркеров", CODE_MARKERS), None);
        // Scanning must also never panic by splitting a multi-byte character.
        assert_eq!(find_word("тест—TODO—тест", CODE_MARKERS), Some("TODO"));
    }

    #[test]
    fn a_word_longer_than_the_haystack_is_not_matched() {
        assert_eq!(find_word("hi", CODE_MARKERS), None);
    }

    #[test]
    fn an_entry_ending_in_punctuation_follows_real_word_boundary_semantics() {
        // `\b` after `(` asserts a transition, so it holds only when a word character
        // follows. Reading `\b` as "the neighbour is not a word character" would invert
        // both of these.
        assert!(contains_word("open(path)", MIXED_CONCERN_SYMBOLS));
        assert!(!contains_word("open()", MIXED_CONCERN_SYMBOLS));
    }

    /// Differential check against the regex engine itself.
    ///
    /// The point of this module is that a hand-written scan is clearer and cheaper than
    /// a regex for fixed word sets — but "clearer" is worthless if it is not also
    /// *equivalent*. This asserts agreement with `regex` on every entry of every set
    /// across a corpus of boundary-relevant contexts, so the replacement is verified
    /// rather than assumed. `regex` remains a dependency of this crate for the reports
    /// that genuinely parse source syntax, so this costs nothing extra.
    #[test]
    fn matches_the_regex_engine_on_every_word_in_every_set() {
        let sets: [&[&str]; 3] = [CODE_MARKERS, UI_INFRA_SYMBOLS, MIXED_CONCERN_SYMBOLS];

        // Contexts chosen to exercise both edges: string boundaries, whitespace,
        // punctuation, underscores, digits, adjacent letters and non-ASCII neighbours.
        let contexts: [&str; 12] = [
            "{}",
            " {} ",
            "prefix{}",
            "{}suffix",
            "_{}",
            "{}_",
            "({})",
            "a{}b",
            "1{}2",
            "//{}: text",
            "путь{}путь",
            "{}{}",
        ];

        for set in sets {
            let alternation = set
                .iter()
                .map(|word| regex::escape(word))
                .collect::<Vec<_>>()
                .join("|");
            let pattern = regex::Regex::new(&format!(r"(?i)\b(?:{alternation})\b"))
                .expect("escaped alternation of literals is always a valid regex");

            for word in set {
                for context in contexts {
                    let haystack = context.replace("{}", word);
                    assert_eq!(
                        contains_word(&haystack, set),
                        pattern.is_match(&haystack),
                        "disagreed with the regex engine on {haystack:?} (word {word:?})"
                    );
                }
            }
        }
    }
}
