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

use regex::Captures;

use crate::patterns::keyword::SECRET_PATTERNS;

fn replace_match(matched: &str) -> String {
    // The retained key name goes through the same sanitizer as the scan-report path
    // (Q16). This pass rewrites *exported file content* and the clipboard, so a secret
    // surviving here is handed to whoever receives the bundle — strictly worse than the
    // finding-message leak Q16 was opened for, and the identical split-on-first-`=`
    // cause. `SECRET: "dXNlcjpwYXNzd29yZA=="` used to yield
    // `SECRET: "dXNlcjpwYXNzd29yZA=<REDACTED>`, which decodes to a real credential.
    use crate::patterns::keyword::sanitize_key_prefix as sanitize;
    if let Some(eq_pos) = matched.find('=') {
        let key = sanitize(&matched[..eq_pos]);
        return format!("{key}=<REDACTED>");
    }
    if let Some(colon_pos) = matched.find(':') {
        let key = sanitize(&matched[..colon_pos]);
        return format!("{key}: <REDACTED>");
    }
    "<REDACTED_SECRET>".to_string()
}

fn whole_match<'a>(caps: &Captures<'a>) -> &'a str {
    caps.get(0).map(|m| m.as_str()).unwrap_or_default()
}

/// Replaces `KEY: value` / `KEY=value` / `BEARER <token>` shapes in `text` with a
/// redacted placeholder. Applied to file content before it is included in an export or
/// copied to the clipboard — see `BLUEPRINT.md` §A.4.2.
pub fn redact_secrets(text: &str) -> String {
    let mut redacted = text.to_string();
    for pattern in SECRET_PATTERNS.iter() {
        redacted = pattern
            .replace_all(&redacted, |caps: &Captures<'_>| {
                replace_match(whole_match(caps))
            })
            .into_owned();
    }
    redacted
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
