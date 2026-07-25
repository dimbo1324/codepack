//! Content-level secret redaction, ported from legacy `utils/text_utils.py::redact_secrets`.
//!
//! ## The backreference-rewrite deviation
//!
//! Legacy's regex is:
//!
//! ```text
//! (?i)\b(API[_-]?KEY|SECRET|TOKEN|PASSWORD|PASS|PRIVATE[_-]?KEY)\b\s*[:=]\s*(['"]?)[^\s'"\n]+(\2)
//! ```
//!
//! Group 2 captures an optional opening quote, and group 3 (`\2`, a **backreference**)
//! requires the closing character to match whatever the opening quote was (or, if there
//! was no opening quote, requires an empty closing match). Rust's `regex` crate is a
//! linear-time automaton engine and has no backreference support by design.
//!
//! This is rewritten ([`crate::patterns::keyword::SECRET_PATTERNS`], first entry) as an
//! alternation of three shapes — double-quoted, single-quoted, unquoted — each still
//! forbidding embedded whitespace exactly like legacy's `[^\s'"\n]+`:
//!
//! ```text
//! (?i)\b(API[_-]?KEY|SECRET|TOKEN|PASSWORD|PASS|PRIVATE[_-]?KEY)\b\s*[:=]\s*(?:"[^\s"\n]+"|'[^\s'\n]+'|[^\s'"\n]+)
//! ```
//!
//! **Span equivalence, not bit-for-bit identity — by construction, not by luck.** On
//! well-formed input (matching open/close quotes, or no quotes at all — the only shapes
//! that occur in practice) this produces the identical match span as the backreference
//! original; see `redact_span_matches_legacy_backreference_semantics` below.
//!
//! On the mismatched-quote edge case `KEY: "value'`, both implementations happen to
//! agree by *not* matching: the backreference original requires the closing character
//! to equal the opening `"`, which never appears again, so it fails; this rewrite's
//! double-quoted alternative also requires a closing `"` that is absent, its
//! single-quoted alternative requires an opening `'` that is not the next character,
//! and its unquoted alternative cannot start on a `"` at all (excluded by
//! `[^\s'"\n]`). See `mismatched_quote_style_does_not_falsely_close_early` below — this
//! is verified by test, not assumed.

use crate::patterns::keyword::{KeySpacing, find_secret_spans, redact_value_after_separator};

/// Rewrites one matched `KEY: value` / `KEY=value` / `BEARER <token>` span.
///
/// Shares [`redact_value_after_separator`] with the scan-report path (Q16). Keeping two
/// copies is what let the content path — the more dangerous of the two, since its output
/// is handed to whoever receives the bundle — retain the leak after the message path was
/// fixed: `SECRET: "dXNlcjpwYXNzd29yZA=="` used to yield
/// `SECRET: "dXNlcjpwYXNzd29yZA=<REDACTED>`, which decodes to a real credential.
fn replace_match(matched: &str) -> String {
    redact_value_after_separator(
        matched,
        // The match span starts at the keyword itself, and legacy preserved any space
        // before the separator; golden references contain that spelling.
        KeySpacing::Preserve,
        // A `BEARER <token>` span carries no key at all, so there is nothing to name.
        "<REDACTED_SECRET>",
    )
}

/// Replaces `KEY: value` / `KEY=value` / `BEARER <token>` shapes in `text` with a
/// redacted placeholder. Applied to file content before it is included in an export or
/// copied to the clipboard — see `BLUEPRINT.md` §A.4.2.
///
/// Works line by line because the shapes themselves are line-scoped: the original
/// patterns excluded `\n` from every value character class, so a value could never span
/// a newline. Scanning per line also keeps the span offsets simple and bounds the work
/// on a very large file.
pub fn redact_secrets(text: &str) -> String {
    // `split_inclusive` keeps each line's terminator attached, so joining the results
    // reproduces the original line endings exactly — including a missing final newline,
    // and including `\r\n`, whose `\r` is simply part of the line's tail.
    text.split_inclusive('\n').map(redact_line_spans).collect()
}

/// Replaces every secret span on one line, left to right.
fn redact_line_spans(line: &str) -> String {
    let spans = find_secret_spans(line);
    if spans.is_empty() {
        return line.to_string();
    }

    // Applying the keyword spans first and the bearer spans after would shift offsets,
    // so all spans are sorted and applied in one pass. Overlaps cannot occur in
    // practice — a bearer token is not a `key=value` — but a later span starting inside
    // an earlier one is skipped rather than corrupting the output.
    let mut ordered = spans;
    ordered.sort_unstable();

    let mut out = String::with_capacity(line.len());
    let mut cursor = 0usize;
    for (start, end) in ordered {
        if start < cursor {
            continue;
        }
        out.push_str(&line[cursor..start]);
        out.push_str(&replace_match(&line[start..end]));
        cursor = end;
    }
    out.push_str(&line[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_containing_an_equals_sign_does_not_survive_into_exported_content() {
        // This pass rewrites exported file content and the clipboard, so a survivor here
        // is handed to whoever receives the bundle. `dXNlcjpwYXNzd29yZA==` decodes to
        // `user:password`.
        let redacted = redact_secrets(r#"SECRET: "dXNlcjpwYXNzd29yZA==""#);
        assert!(
            !redacted.contains("dXNlcjpwYXNzd29yZA"),
            "secret leaked into exported content: {redacted}"
        );
    }

    #[test]
    fn redacts_double_quoted_value() {
        assert_eq!(
            redact_secrets(r#"API_KEY: "abc123""#),
            "API_KEY: <REDACTED>"
        );
    }

    #[test]
    fn redacts_single_quoted_value() {
        assert_eq!(redact_secrets("API_KEY='abc123'"), "API_KEY=<REDACTED>");
    }

    #[test]
    fn redacts_unquoted_value() {
        assert_eq!(redact_secrets("API_KEY=abc123"), "API_KEY=<REDACTED>");
    }

    #[test]
    fn redact_span_matches_legacy_backreference_semantics() {
        // Well-formed matching-quote cases: span-equivalent to the Python backreference
        // original by construction (documented in the module doc comment above). Uses
        // "SECRET" (an actual `REDACT_KEYWORDS` member) rather than plain "KEY", which
        // is not itself a keyword root and would never match either implementation.
        assert_eq!(redact_secrets(r#"SECRET: "value""#), "SECRET: <REDACTED>");
        assert_eq!(redact_secrets("SECRET='value'"), "SECRET=<REDACTED>");
        assert_eq!(redact_secrets("SECRET=value"), "SECRET=<REDACTED>");
    }

    #[test]
    fn mismatched_quote_style_does_not_falsely_close_early() {
        // SECRET: "value' — neither the backreference original nor this rewrite
        // produces a match here (see module doc comment for the full trace); the value
        // is not safely identifiable as fully quoted, so it is left untouched by both.
        let input = r#"SECRET: "value'"#;
        assert_eq!(redact_secrets(input), input);
    }

    #[test]
    fn redacts_bearer_token() {
        // The BEARER pattern's own match span is just "BEARER <token>" — it never
        // includes the "Authorization:" prefix that precedes it on the line, so
        // `replace_match` finds neither `=` nor `:` inside the matched text itself and
        // falls through to the bare `<REDACTED_SECRET>` placeholder (legacy's `repl`
        // does the same: `match.group(0)` is only the BEARER span).
        assert_eq!(
            redact_secrets("Authorization: BEARER abcdefghijklmnopqrstuvwxyz"),
            "Authorization: <REDACTED_SECRET>"
        );
    }

    #[test]
    fn bearer_token_below_length_threshold_is_not_redacted() {
        assert_eq!(
            redact_secrets("Authorization: BEARER short"),
            "Authorization: BEARER short"
        );
    }

    #[test]
    fn non_secret_text_is_untouched() {
        let text = "the quick brown fox jumps over the lazy dog";
        assert_eq!(redact_secrets(text), text);
    }

    #[test]
    fn password_keyword_redacted() {
        assert_eq!(redact_secrets("PASSWORD: hunter2"), "PASSWORD: <REDACTED>");
    }

    #[test]
    fn private_key_keyword_redacted() {
        assert_eq!(
            redact_secrets("PRIVATE_KEY=abcdef0123456789"),
            "PRIVATE_KEY=<REDACTED>"
        );
    }

    #[test]
    fn no_placeholder_leaks_original_value() {
        let redacted = redact_secrets(r#"SECRET="super-sensitive-value-123""#);
        assert!(!redacted.contains("super-sensitive-value-123"));
    }
}
