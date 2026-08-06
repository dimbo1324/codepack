//! Stable per-secret labels: telling two redacted values apart without revealing either.
//!
//! ## The problem this solves
//!
//! Redaction replaces every secret with the same placeholder, so a reader of the bundle
//! — very often a language model — sees `DATABASE_URL=<REDACTED>` in three files and
//! cannot tell whether that is one credential used three times or three different ones.
//! The structure is gone along with the values, and structure is exactly what somebody
//! reasoning about the code needs: "the worker and the API talk to the same database"
//! is a fact about the project, not about the password.
//!
//! With labels on, the first distinct secret becomes `<REDACTED:s1>`, the second
//! `<REDACTED:s2>`, and every later occurrence of the first is `<REDACTED:s1>` again.
//!
//! ## Why a counter and not a hash
//!
//! A hash of the value would be simpler — no shared state, stable across runs — and it
//! would be wrong. A truncated hash printed beside a secret is a *commitment* to it: a
//! reader who guesses `hunter2` can hash the guess and confirm it, which turns every
//! redacted low-entropy secret into a checkable dictionary attack. Invariant I3 says the
//! value never leaves the redactor, and a verifiable fingerprint of the value is a form
//! of leaving.
//!
//! A first-seen counter reveals only what the reader can already see: how many distinct
//! secrets there are and which occurrences coincide. That is the entire feature, with
//! nothing extra attached.
//!
//! The cost is that labels are per-run and order-dependent — `s1` in yesterday's bundle
//! is not necessarily `s1` in today's. They are a key to one document, not an identity
//! across documents; [`crate::scan::Finding`] fingerprints already fill that role, and
//! they are computed from the redacted message rather than the secret.
//!
//! ## Scope
//!
//! Off by default (`Config::redaction_labels`). With it off every placeholder is byte
//! for byte what it has always been, so no artifact moves and no `schema_version`
//! changes.

use std::collections::HashMap;
use std::sync::Mutex;

/// First-seen numbering of the distinct secrets in one run.
#[derive(Debug, Default)]
pub struct Labels {
    seen: Mutex<HashMap<String, usize>>,
}

impl Labels {
    /// The label for `secret`, assigning the next number the first time it is seen.
    ///
    /// Poisoning is handled by taking the inner value rather than propagating: a panic
    /// in another thread must not turn redaction — the thing standing between a secret
    /// and the output — into a panic of its own.
    fn label(&self, secret: &str) -> usize {
        let key = normalise(secret);
        let mut seen = self.seen.lock().unwrap_or_else(|error| error.into_inner());
        let next = seen.len() + 1;
        *seen.entry(key).or_insert(next)
    }

    /// How many distinct secrets have been labelled so far. Reported by
    /// [`crate::Redactor::distinct_secrets`] so an artifact header can say how many
    /// different credentials the labels stand for.
    pub fn distinct(&self) -> usize {
        self.seen
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len()
    }
}

/// Two spellings of the same secret get the same label.
///
/// Quotes and surrounding whitespace are syntax, not value: `KEY="abc"`, `KEY = abc` and
/// `KEY: 'abc'` all carry `abc`. A trailing comma or semicolon is punctuation from the
/// language, and is stripped for the same reason.
fn normalise(secret: &str) -> String {
    let trimmed = secret.trim().trim_end_matches([',', ';']).trim();
    let unquoted = match trimmed.chars().next() {
        Some(quote @ ('"' | '\'')) if trimmed.len() >= 2 && trimmed.ends_with(quote) => {
            &trimmed[1..trimmed.len() - 1]
        }
        _ => trimmed,
    };
    unquoted.to_string()
}

/// How a redaction site spells its replacement.
///
/// Threaded through the redaction functions rather than read from a global: two exports
/// can run in the same process (the desktop app), and their labels must not share a
/// numbering.
#[derive(Debug, Clone, Copy)]
pub struct Placeholders<'a> {
    labels: Option<&'a Labels>,
}

impl<'a> Placeholders<'a> {
    /// Today's behaviour, unchanged: one indistinguishable placeholder.
    pub fn plain() -> Self {
        Self { labels: None }
    }

    pub fn labelled(labels: &'a Labels) -> Self {
        Self {
            labels: Some(labels),
        }
    }

    /// Replacement for a value that follows a key (`API_KEY=<here>`).
    pub fn value(&self, secret: &str) -> String {
        match self.labels {
            Some(labels) => format!("<REDACTED:s{}>", labels.label(secret)),
            None => "<REDACTED>".to_string(),
        }
    }

    /// Replacement for a span with no key of its own — a bare token, a provider
    /// signature, a password inside a URL.
    pub fn bare(&self, secret: &str) -> String {
        match self.labels {
            Some(labels) => format!("<REDACTED_SECRET:s{}>", labels.label(secret)),
            None => "<REDACTED_SECRET>".to_string(),
        }
    }
}

/// Every placeholder shape redaction can produce, as literal prefixes.
///
/// Exported because consumers have to recognise them: `verify` strips them before
/// asking whether anything credential-shaped is left, and the text dump counts them.
/// A consumer matching on `"<REDACTED>"` alone would silently stop recognising a
/// labelled bundle.
pub const REDACTION_PLACEHOLDER_PREFIXES: &[&str] = &[
    "<REDACTED_SECRET_LINE>",
    "<REDACTED_SECRET:",
    "<REDACTED_SECRET>",
    "<REDACTED:",
    "<REDACTED>",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_secret_gets_the_same_label_and_a_different_one_does_not() {
        let labels = Labels::default();
        let placeholders = Placeholders::labelled(&labels);

        assert_eq!(placeholders.value("hunter2"), "<REDACTED:s1>");
        assert_eq!(placeholders.value("other"), "<REDACTED:s2>");
        assert_eq!(placeholders.value("hunter2"), "<REDACTED:s1>");
        assert_eq!(labels.distinct(), 2);
    }

    #[test]
    fn quoting_and_spacing_do_not_create_a_second_identity() {
        let labels = Labels::default();
        let placeholders = Placeholders::labelled(&labels);

        assert_eq!(placeholders.value(" abc "), "<REDACTED:s1>");
        assert_eq!(placeholders.value("\"abc\""), "<REDACTED:s1>");
        assert_eq!(placeholders.value("'abc',"), "<REDACTED:s1>");
        assert_eq!(labels.distinct(), 1);
    }

    #[test]
    fn a_label_never_contains_any_part_of_the_secret() {
        // The whole point. A label is a position in a list, not a function of the value,
        // so it cannot be tested against a guess.
        let labels = Labels::default();
        let placeholders = Placeholders::labelled(&labels);

        let label = placeholders.value("wJalrXUtnFEMI/K7MDENG/bPxRfiCY");
        assert_eq!(label, "<REDACTED:s1>");
        assert!(!label.contains("wJal"));
        assert!(!label.contains("bPxRfiCY"));
    }

    #[test]
    fn the_plain_mode_spelling_is_exactly_what_it_always_was() {
        let placeholders = Placeholders::plain();
        assert_eq!(placeholders.value("anything"), "<REDACTED>");
        assert_eq!(placeholders.bare("anything"), "<REDACTED_SECRET>");
    }

    #[test]
    fn a_mismatched_quote_is_not_stripped() {
        // `"abc` is not a quoted `abc`; treating it as one would merge two different
        // values under one label.
        let labels = Labels::default();
        let placeholders = Placeholders::labelled(&labels);
        placeholders.value("\"abc");
        placeholders.value("abc");
        assert_eq!(labels.distinct(), 2);
    }

    #[test]
    fn two_runs_number_independently() {
        // A desktop process can run two exports; sharing a counter would leak the fact
        // that a value appeared in an earlier, unrelated bundle.
        let first = Labels::default();
        let second = Labels::default();
        assert_eq!(Placeholders::labelled(&first).value("a"), "<REDACTED:s1>");
        assert_eq!(Placeholders::labelled(&second).value("b"), "<REDACTED:s1>");
    }
}
