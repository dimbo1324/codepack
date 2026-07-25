//! Keyword-based secret detection, ported from legacy `reports/insights/security.py`
//! (`_secret_confidence`, `redacted_line`) plus the shared `constants.py` patterns.
//!
//! Owns the canonical `SECRET_PATTERNS`/`SECRET_KEY_PATTERN`/`ASSIGNMENT_SECRET_RE`/
//! `PRIVATE_KEY_RE` regex objects; [`crate::redact::redact_secrets`] reuses
//! [`SECRET_PATTERNS`] rather than duplicating the regex definitions.

use std::sync::LazyLock;

use regex::Regex;

/// The five single/compound keyword roots that redaction acts on. Ported from
/// legacy `_REDACT_KEYWORDS`.
pub(crate) const REDACT_KEYWORDS: &str = "API[_-]?KEY|SECRET|TOKEN|PASSWORD|PASS|PRIVATE[_-]?KEY";

/// [`REDACT_KEYWORDS`] plus four keyword roots that are scanned for (contributing to
/// `low`-confidence findings) but are not themselves redaction targets. Ported from
/// legacy `_SCAN_KEYWORDS`. Kept as an independent literal (Rust `const` cannot
/// concatenate another `const &str` at compile time) with a test asserting it starts
/// with [`REDACT_KEYWORDS`] to keep the two in sync by construction.
pub(crate) const SCAN_KEYWORDS: &str = "API[_-]?KEY|SECRET|TOKEN|PASSWORD|PASS|PRIVATE[_-]?KEY|DATABASE[_-]?URL|JWT[_-]?SECRET|ACCESS[_-]?KEY|CLIENT[_-]?SECRET";

/// Legacy `SECRET_PATTERNS`: a keyword/value pattern (see `crate::redact` for the
/// backreference-rewrite rationale) and a `BEARER <token>` pattern. A line matching
/// either yields `high` confidence.
pub(crate) static SECRET_PATTERNS: LazyLock<[Regex; 2]> = LazyLock::new(|| {
    [
        Regex::new(&format!(
            r#"(?i)\b({REDACT_KEYWORDS})\b\s*[:=]\s*(?:"[^\s"\n]+"|'[^\s'\n]+'|[^\s'"\n]+)"#
        ))
        .expect("hand-written keyword/value pattern is a valid regex, proven by test coverage"),
        Regex::new(r"(?i)\b(BEARER)\s+[A-Za-z0-9._\-+/=]{16,}")
            .expect("hand-written BEARER pattern is a valid regex, proven by test coverage"),
    ]
});

/// Legacy `SECRET_KEY_PATTERN`: a bare keyword mention (no value shape required).
/// A line matching this — outside a comment — yields `low` confidence.
pub(crate) static SECRET_KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?i)\b({SCAN_KEYWORDS})\b"))
        .expect("hand-written scan-keyword pattern is a valid regex, proven by test coverage")
});

/// Legacy `_ASSIGNMENT_SECRET_RE`: a secret-shaped key immediately followed by `:`/`=`,
/// with no requirement on the value itself — deliberately looser than
/// [`SECRET_PATTERNS`], yielding `medium` confidence.
pub(crate) static ASSIGNMENT_SECRET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(api[_-]?key|secret|token|password|pass|private[_-]?key|database[_-]?url|jwt[_-]?secret)\b\s*[:=]",
    )
    .expect("hand-written assignment pattern is a valid regex, proven by test coverage")
});

/// Legacy `_PRIVATE_KEY_RE`: a PEM private-key header. The single `critical`-tier
/// keyword rule; also reused by `patterns::provider` as the `pem-private-key` provider
/// signature (same regex, two rule identities).
pub(crate) static PRIVATE_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")
        .expect("hand-written PEM header pattern is a valid regex, proven by test coverage")
});

/// Legacy `_SCANNER_CODE_HINTS`, respelled to this crate's own Rust identifiers. Any
/// line containing one of these substrings is exempted from every detector (keyword,
/// provider, entropy) — this is what keeps the scanner from flagging its own source.
pub(crate) const SELF_PROTECTION_HINTS: &[&str] = &[
    "SECRET_PATTERNS",
    "SECRET_KEY_PATTERN",
    "redact_secrets",
    "REDACT_KEYWORDS",
    "SCAN_KEYWORDS",
    "ASSIGNMENT_SECRET_RE",
    "PRIVATE_KEY_RE",
    "PROVIDER_PATTERNS",
];

/// `true` when `line` mentions one of this crate's own pattern identifiers and must
/// therefore be exempted from every detector, not only the keyword cascade.
pub fn is_self_protected(line: &str) -> bool {
    SELF_PROTECTION_HINTS.iter().any(|hint| line.contains(hint))
}

/// The five-level confidence cascade, ported from legacy `_secret_confidence`. Order is
/// load-bearing: self-protection first, then `critical` → `high` → `medium` → `low`,
/// first match wins. Returns `None` when nothing qualifies.
pub fn secret_confidence(line: &str) -> Option<&'static str> {
    if is_self_protected(line) {
        return None;
    }
    if PRIVATE_KEY_RE.is_match(line) {
        return Some("critical");
    }
    if SECRET_PATTERNS.iter().any(|pattern| pattern.is_match(line)) {
        return Some("high");
    }
    if ASSIGNMENT_SECRET_RE.is_match(line) {
        return Some("medium");
    }
    let trimmed = line.trim_start();
    if SECRET_KEY_PATTERN.is_match(line)
        && !(trimmed.starts_with('#') || trimmed.starts_with("//") || trimmed.starts_with('*'))
    {
        return Some("low");
    }
    None
}

/// Legacy `redacted_line`: a second, stronger redaction pass specific to scan reports.
/// First applies [`crate::redact::redact_secrets`] (the content-level pass), then — if
/// the result still mentions a scan keyword — collapses the rest of the line using the
/// same split-on-first-`=`-or-`:` logic. Every [`crate::scan::Finding`] message must go
/// through this function (invariant I3): the raw matched substring never reaches the
/// output, only the key name survives.
pub fn redacted_line(line: &str) -> String {
    let redacted = crate::redact::redact_secrets(line);
    if SECRET_KEY_PATTERN.is_match(&redacted) {
        if let Some(pos) = redacted.find('=') {
            let key = sanitize_key_prefix(redacted[..pos].trim());
            return format!("{key}=<REDACTED>");
        }
        if let Some(pos) = redacted.find(':') {
            let key = sanitize_key_prefix(redacted[..pos].trim());
            return format!("{key}: <REDACTED>");
        }
        return "<REDACTED_SECRET_LINE>".to_string();
    }
    redacted.trim().to_string()
}

/// Length at which an unbroken alphanumeric run stops being plausible as a word a
/// person typed. Real key names break at `_`, `-`, `.` and spaces, so their runs are
/// short; encoded values do not break at all.
///
/// Set to 12 rather than something larger because the run only has to be long enough to
/// carry a secret: base64 of an 11-byte password is 16 characters, and the first version
/// of this fix used a 16-character threshold that a 15-character run walked straight
/// through. The cost of being wrong in this direction is a masked word in a message; the
/// cost of being wrong in the other direction is a leaked credential.
const ENCODED_RUN_MIN_LEN: usize = 12;

/// Masks anything in the retained key-name text that could itself be a secret.
///
/// **Q16 (owner decision 2026-07-25).** Legacy splits on the first `=`/`:` and keeps
/// everything before it, which assumes the separator belongs to a `key=value` pair. When
/// the separator sits *inside* the secret — base64 padding, an `Authorization: Basic …`
/// header — the "key name" is the secret itself, and it travelled into the finding
/// message, the JSON, the SARIF, the database row and the log. That breaches invariant
/// I3, which is absolute.
///
/// This masks rather than rejects, because rejecting the whole line throws away the
/// identifier the message exists to carry. A run of at least [`ENCODED_RUN_MIN_LEN`]
/// alphanumeric characters that is **not purely alphabetic** is replaced; anything else
/// is kept verbatim. Purely alphabetic is the right exemption: `Authorization`,
/// `postgres` and `SECRET` survive, while base64, hex digests and random tokens — none
/// of which are all-letters at that length — do not.
///
/// The rule deliberately over-masks rather than under-masks. `oauth2ClientSecret` is a
/// legitimate identifier that carries a digit and will be masked; the result is a less
/// informative message, never an exposed credential.
pub(crate) fn sanitize_key_prefix(prefix: &str) -> String {
    let mut out = String::with_capacity(prefix.len());
    let mut run = String::new();

    let flush = |run: &mut String, out: &mut String| {
        if run.len() >= ENCODED_RUN_MIN_LEN && !run.chars().all(|c| c.is_ascii_alphabetic()) {
            out.push_str("<REDACTED>");
        } else {
            out.push_str(run);
        }
        run.clear();
    };

    for ch in prefix.chars() {
        if ch.is_ascii_alphanumeric() || ch == '+' || ch == '/' {
            run.push(ch);
        } else {
            flush(&mut run, &mut out);
            out.push(ch);
        }
    }
    flush(&mut run, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_containing_an_equals_sign_does_not_survive_as_the_key_name() {
        // Q16. The split-on-first-`=` rule treats everything before it as a key name.
        // When the `=` is base64 padding inside the secret, the "key name" *is* the
        // secret, so it travelled into the finding message and from there into the
        // JSON, the SARIF, the database row and the log — a direct I3 breach.
        let line = "curl -H 'Authorization: Basic dXNlcjpwYXNzd29yZA==' # token";
        let message = redacted_line(line);
        assert!(
            !message.contains("dXNlcjpwYXNzd29yZA"),
            "the secret leaked into the message: {message}"
        );
    }

    #[test]
    fn a_short_encoded_secret_is_masked_too_not_just_a_long_one() {
        // The first version of this fix used a 16-character threshold, which base64 of
        // an 11-byte password (16 chars) cleared and a 15-character run walked straight
        // through. `aHVudGVyMnBhc3M=` decodes to `hunter2pass`.
        let message = redacted_line("curl -H 'Authorization: Basic aHVudGVyMnBhc3M=' # token");
        assert!(
            !message.contains("aHVudGVyMnBhc3M"),
            "short encoded secret leaked: {message}"
        );
    }

    #[test]
    fn a_hex_digest_is_masked_although_it_has_no_uppercase() {
        // An earlier rule required upper case, lower case *and* digits together, which
        // let lowercase hex digests through.
        let message = redacted_line("hash = d41d8cd98f00b204e9800998ecf8427e # token");
        assert!(
            !message.contains("d41d8cd98f00b204"),
            "hex digest leaked: {message}"
        );
    }

    #[test]
    fn an_ordinary_key_name_survives_verbatim() {
        // The masking must not eat the identifier the message exists to carry; these two
        // shapes are what the golden references contain.
        assert_eq!(
            redacted_line("      - JWT_SECRET=fixture-placeholder-value"),
            "- JWT_SECRET=<REDACTED>"
        );
        assert_eq!(
            redacted_line(r#"SECRET_TOKEN = "placeholder-token-for-fixture-only""#),
            "SECRET_TOKEN=<REDACTED>"
        );
    }

    #[test]
    fn sanitizer_keeps_words_and_masks_encoded_runs() {
        assert_eq!(sanitize_key_prefix("- JWT_SECRET"), "- JWT_SECRET");
        assert_eq!(sanitize_key_prefix("Authorization"), "Authorization");
        assert_eq!(
            sanitize_key_prefix("Basic aHVudGVyMnBhc3M"),
            "Basic <REDACTED>"
        );
        // Documented over-masking: a legitimate identifier carrying a digit is masked
        // rather than risked. Information loss, never exposure.
        assert_eq!(sanitize_key_prefix("oauth2ClientSecret"), "<REDACTED>");
    }

    #[test]
    fn scan_keywords_starts_with_redact_keywords() {
        assert!(SCAN_KEYWORDS.starts_with(REDACT_KEYWORDS));
    }

    #[test]
    fn self_protection_exempts_own_pattern_names() {
        assert!(is_self_protected("// see SECRET_PATTERNS for details"));
        assert_eq!(
            secret_confidence("// see SECRET_PATTERNS for details"),
            None
        );
        assert_eq!(
            secret_confidence("this line calls redact_secrets(line)"),
            None
        );
    }

    #[test]
    fn critical_confidence_for_pem_header() {
        assert_eq!(
            secret_confidence("-----BEGIN RSA PRIVATE KEY-----"),
            Some("critical")
        );
    }

    #[test]
    fn high_confidence_for_secret_pattern_match() {
        assert_eq!(
            secret_confidence(r#"API_KEY = "abcdef123456""#),
            Some("high")
        );
        assert_eq!(
            secret_confidence("Authorization: BEARER abcdefghijklmnopqrstuvwxyz"),
            Some("high")
        );
    }

    #[test]
    fn medium_confidence_for_assignment_without_value_shape() {
        // ASSIGNMENT_SECRET_RE fires on the key/operator alone. SECRET_PATTERNS'
        // unquoted alternative `[^\s'"\n]+` only needs one whitespace-delimited run of
        // characters after the operator to match — so a line with an actual (even
        // single-word) value after the colon already escalates to `high` (verified in
        // `high_confidence_for_secret_pattern_match` below). `medium` is reachable only
        // when the operator has no value at all following it on the line.
        assert_eq!(secret_confidence("token: "), Some("medium"));
    }

    #[test]
    fn high_confidence_when_any_word_follows_the_operator() {
        // Contrast with `medium_confidence_for_assignment_without_value_shape` above:
        // once *any* non-whitespace run follows the operator, SECRET_PATTERNS' unquoted
        // alternative already matches it, escalating straight to `high` — matching
        // legacy's backreference original, whose empty-capture group closes trivially
        // on an unquoted single word.
        assert_eq!(
            secret_confidence("token: this is not a single quoted value"),
            Some("high")
        );
    }

    #[test]
    fn low_confidence_for_bare_keyword_outside_comment() {
        assert_eq!(
            secret_confidence("we discussed the access_key rotation policy today"),
            Some("low")
        );
    }

    #[test]
    fn low_confidence_suppressed_inside_comment() {
        assert_eq!(secret_confidence("# token rotation policy"), None);
        assert_eq!(secret_confidence("// token rotation policy"), None);
        assert_eq!(secret_confidence("* token rotation policy"), None);
    }

    #[test]
    fn no_confidence_for_unrelated_line() {
        assert_eq!(secret_confidence("let counter = 0;"), None);
    }

    #[test]
    fn redacted_line_collapses_remaining_keyword_mentions() {
        // First pass (redact_secrets) yields "API_KEY =<REDACTED>" (space before `=`
        // preserved from the matched span); the keyword `API_KEY` is still present, so
        // the second, stronger pass collapses it again into a clean "key=<REDACTED>".
        assert_eq!(
            redacted_line(r#"API_KEY = "abcdef123456""#),
            "API_KEY=<REDACTED>"
        );
    }

    #[test]
    fn redacted_line_never_contains_original_secret_value() {
        let redacted = redacted_line(r#"SECRET="super-sensitive-value-123""#);
        assert!(!redacted.contains("super-sensitive-value-123"));
    }

    #[test]
    fn redacted_line_trims_and_passes_through_clean_lines() {
        assert_eq!(redacted_line("   let x = 1;   "), "let x = 1;");
    }
}
