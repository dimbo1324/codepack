//! Source-shape predicates for the risky-code rules, written as string scans.
//!
//! The nine rules in [`crate::patterns::risky_code`] all look for one of three shapes:
//! a call (`name(`), a member assignment (`.member =`), or a call whose *arguments*
//! carry something interesting further along the line. Expressed as regexes those three
//! shapes were spelled nine slightly different ways, each carrying its own
//! `.expect("hand-written pattern literal is a valid regex, …")`.
//!
//! Naming the three shapes instead means each rule declares which one it is and supplies
//! the literals, so a reader checks a rule by reading two identifiers rather than by
//! parsing a pattern. It also removes the last regex from this crate — see
//! `crate::patterns::token_scan` for the same argument applied to vendor token formats.
//!
//! Equivalence with the regexes these replaced is verified in
//! [`crate::patterns::risky_code`]'s own differential test, not assumed.

/// Horizontal whitespace, the `\s*` that every one of these shapes allows between a name
/// and the punctuation that follows it. Deliberately not vertical whitespace: these
/// predicates run per line, and the originals used `[^\n]*` to stay on one line too.
fn skip_spaces(text: &str, from: usize) -> usize {
    let bytes = text.as_bytes();
    let mut index = from;
    while index < bytes.len() && (bytes[index] == b' ' || bytes[index] == b'\t') {
        index += 1;
    }
    index
}

/// A word character, matching the `\w` the original `\b` assertions were defined against.
fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// True when a match starting at `start` is preceded by a word boundary — the `\b` the
/// bare-function rules (`\beval`, `\bexec`, `\bpickle`) carry.
///
/// Only the leading edge is checked, because that is all the originals asserted: they
/// end in punctuation (`(`), which already cannot continue an identifier.
fn has_leading_boundary(line: &str, start: usize) -> bool {
    line[..start]
        .chars()
        .next_back()
        .is_none_or(|c| !is_word_char(c))
}

/// Whether the identifier at `start` should be treated as word-bounded on its left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Anchor {
    /// `\b<name>` — the name must start an identifier, so `myeval(` does not match.
    WordStart,
    /// `<name>` anywhere, matching the originals that omitted `\b` because the literal
    /// already begins with `.` or is qualified (`yaml.load`, `document.write`).
    Anywhere,
}

/// Finds every offset at which `needle` occurs in `line`, honouring `anchor`.
fn occurrences<'a>(
    line: &'a str,
    needle: &'a str,
    anchor: Anchor,
) -> impl Iterator<Item = usize> + 'a {
    line.match_indices(needle)
        .map(|(index, _)| index)
        .filter(move |index| match anchor {
            Anchor::WordStart => has_leading_boundary(line, *index),
            Anchor::Anywhere => true,
        })
}

/// `<name>\s*(` — a call to `name`.
///
/// Used for `eval(`, `exec(`, `pickle.load(`, `yaml.load(`, `document.write(`.
pub(crate) fn calls(line: &str, name: &str, anchor: Anchor) -> bool {
    occurrences(line, name, anchor).any(|start| {
        let after_name = start + name.len();
        line.as_bytes().get(skip_spaces(line, after_name)) == Some(&b'(')
    })
}

/// `<member>\s*=` — an assignment to `member`, excluding `==` so a comparison is not
/// mistaken for one.
///
/// The original (`\.innerHTML\s*=`) did not exclude `==`; it did not need to, because
/// `el.innerHTML == x` is a comparison no one writes and the rule is advisory either
/// way. Excluding it here costs nothing and removes an obvious false positive, which is
/// the direction this crate errs in deliberately: precision over recall (invariant I9).
pub(crate) fn assigns_to(line: &str, member: &str, anchor: Anchor) -> bool {
    occurrences(line, member, anchor).any(|start| {
        let equals = skip_spaces(line, start + member.len());
        let bytes = line.as_bytes();
        bytes.get(equals) == Some(&b'=') && bytes.get(equals + 1) != Some(&b'=')
    })
}

/// Byte offset just past the `(` of a `<qualifier>.<member path>(` call, if the line has
/// one.
///
/// Everything between the qualifier and the parenthesis must be a plain member path —
/// identifier characters and dots — so `subprocess.run(` and `localStorage.setItem(`
/// qualify while `subprocess; other(` does not. Matching is case-insensitive, as the
/// `localStorage` rule's original `(?i)` required.
fn qualified_call_argument_start(lowered: &str, qualifier: &str) -> Option<usize> {
    let qualifier = qualifier.to_ascii_lowercase();
    occurrences(lowered, &qualifier, Anchor::Anywhere).find_map(|start| {
        let rest = &lowered[start + qualifier.len()..];
        let paren_offset = rest.find('(')?;
        let between = &rest[..paren_offset];
        if !between.chars().all(|c| is_word_char(c) || c == '.') {
            return None;
        }
        Some(start + qualifier.len() + paren_offset + 1)
    })
}

/// True when the line contains a `<qualifier>.<member path>(…)` call.
pub(crate) fn calls_member_of(line: &str, qualifier: &str) -> bool {
    qualified_call_argument_start(&line.to_ascii_lowercase(), qualifier).is_some()
}

/// `<qualifier>.<member path>(` followed, inside the call, by any of `arguments`.
///
/// The search for `arguments` starts after the opening parenthesis, so an interesting
/// word appearing earlier on the line — `token = localStorage.getItem('theme')` — does
/// not trigger the rule. Both the qualifier and the arguments are matched
/// case-insensitively, matching the original's `(?i)`.
pub(crate) fn call_with_argument(line: &str, qualifier: &str, arguments: &[&str]) -> bool {
    let lowered = line.to_ascii_lowercase();
    let Some(arguments_start) = qualified_call_argument_start(&lowered, qualifier) else {
        return false;
    };
    let tail = &lowered[arguments_start.min(lowered.len())..];
    arguments
        .iter()
        .any(|argument| tail.contains(&argument.to_ascii_lowercase()))
}

/// `shell` `\s*=\s*` `True` — the argument shape the `subprocess` rule looks for.
///
/// Spelled out rather than passed to [`call_with_argument`] as the literal `"shell=True"`
/// because the original allowed whitespace around the `=`, and `shell = True` is how the
/// keyword argument is commonly written.
pub(crate) fn has_keyword_argument(line: &str, name: &str, value: &str) -> bool {
    let lowered = line.to_ascii_lowercase();
    let name_lower = name.to_ascii_lowercase();
    let value_lower = value.to_ascii_lowercase();

    occurrences(&lowered, &name_lower, Anchor::WordStart).any(|start| {
        let equals = skip_spaces(&lowered, start + name_lower.len());
        if lowered.as_bytes().get(equals) != Some(&b'=') {
            return false;
        }
        let value_start = skip_spaces(&lowered, equals + 1);
        lowered[value_start..].starts_with(&value_lower)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_matches_a_bare_call_with_and_without_spacing() {
        assert!(calls("eval(userInput)", "eval", Anchor::WordStart));
        assert!(calls("eval (userInput)", "eval", Anchor::WordStart));
        assert!(calls("eval\t(x)", "eval", Anchor::WordStart));
        assert!(!calls("eval", "eval", Anchor::WordStart));
        assert!(!calls("evaluate = 1", "eval", Anchor::WordStart));
    }

    #[test]
    fn word_start_anchor_rejects_a_name_continuing_an_identifier() {
        assert!(!calls("myeval(x)", "eval", Anchor::WordStart));
        assert!(!calls("safe_eval(x)", "eval", Anchor::WordStart));
        // Anchored anywhere, the same text does match — which is why the two rules that
        // use qualified names do not need the boundary.
        assert!(calls("myeval(x)", "eval", Anchor::Anywhere));
    }

    #[test]
    fn calls_matches_a_qualified_call() {
        assert!(calls("yaml.load(stream)", "yaml.load", Anchor::Anywhere));
        assert!(calls(
            "document.write(html)",
            "document.write",
            Anchor::Anywhere
        ));
        assert!(!calls(
            "yaml.load_all(stream)",
            "yaml.load(",
            Anchor::Anywhere
        ));
    }

    #[test]
    fn assigns_to_matches_an_assignment_but_not_a_comparison() {
        assert!(assigns_to(
            "el.innerHTML = userInput;",
            ".innerHTML",
            Anchor::Anywhere
        ));
        assert!(assigns_to("el.innerHTML=x", ".innerHTML", Anchor::Anywhere));
        assert!(!assigns_to(
            "if (el.innerHTML == other)",
            ".innerHTML",
            Anchor::Anywhere
        ));
    }

    #[test]
    fn call_with_argument_requires_the_argument_after_the_parenthesis() {
        assert!(call_with_argument(
            "localStorage.setItem('auth', token)",
            "localStorage",
            &["token", "jwt", "secret", "password"]
        ));
        // The interesting word appears before the call, not inside it.
        assert!(!call_with_argument(
            "token = localStorage.getItem('unrelated')",
            "localStorage",
            &["token"]
        ));
        assert!(!call_with_argument(
            "localStorage.setItem('theme', 'dark')",
            "localStorage",
            &["token", "jwt", "secret", "password"]
        ));
    }

    #[test]
    fn call_with_argument_requires_a_plain_member_path_before_the_parenthesis() {
        // A qualifier that is merely mentioned, with unrelated punctuation before the
        // next call, must not trigger the rule.
        assert!(!call_with_argument(
            "localStorage; doSomething(token)",
            "localStorage",
            &["token"]
        ));
    }

    #[test]
    fn call_with_argument_is_case_insensitive_in_its_arguments() {
        assert!(call_with_argument(
            "localStorage.setItem('k', TOKEN)",
            "localStorage",
            &["token"]
        ));
    }

    #[test]
    fn keyword_argument_allows_whitespace_around_the_operator() {
        assert!(has_keyword_argument(
            "subprocess.run(cmd, shell=True)",
            "shell",
            "True"
        ));
        assert!(has_keyword_argument(
            "subprocess.run(cmd, shell = True)",
            "shell",
            "True"
        ));
        assert!(!has_keyword_argument(
            "subprocess.run(cmd, shell=False)",
            "shell",
            "True"
        ));
        assert!(!has_keyword_argument(
            "subprocess.run(cmd)",
            "shell",
            "True"
        ));
    }

    #[test]
    fn keyword_argument_requires_a_word_boundary_on_the_name() {
        // `noshell=True` is not the `shell` keyword argument.
        assert!(!has_keyword_argument(
            "run(cmd, noshell=True)",
            "shell",
            "True"
        ));
    }

    #[test]
    fn empty_and_non_ascii_inputs_are_handled_without_panicking() {
        assert!(!calls("", "eval", Anchor::WordStart));
        assert!(!assigns_to("", ".innerHTML", Anchor::Anywhere));
        assert!(!call_with_argument("", "localStorage", &["token"]));
        assert!(!has_keyword_argument("", "shell", "True"));
        // A Cyrillic neighbour is a word character, so it suppresses the boundary.
        assert!(!calls("этоeval(x)", "eval", Anchor::WordStart));
        assert!(calls("это eval(x)", "eval", Anchor::WordStart));
    }
}
