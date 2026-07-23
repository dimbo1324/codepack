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
            let key = redacted[..pos].trim();
            return format!("{key}=<REDACTED>");
        }
        if let Some(pos) = redacted.find(':') {
            let key = redacted[..pos].trim();
            return format!("{key}: <REDACTED>");
        }
        return "<REDACTED_SECRET_LINE>".to_string();
    }
    redacted.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
