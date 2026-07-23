//! Token estimation from byte counts (BLUEPRINT §B.2/§E.1; legacy
//! `utils/text_utils.py::estimate_tokens`).

/// The bytes-per-character density and characters-per-token ratio used by
/// [`estimate_tokens_refined`] to calibrate a token estimate for a specific kind of
/// text content. Both fields are new capabilities: the archived legacy implementation
/// used a single flat 3.5-bytes-per-token ratio for everything (see
/// [`estimate_tokens_fallback`]) and never calibrated by content type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenCoefficients {
    pub bytes_per_char: f64,
    pub chars_per_token: f64,
}

impl TokenCoefficients {
    /// ASCII source code. BLUEPRINT §E.1 states a 3.6-4.0 chars-per-token range for
    /// ASCII code without picking one value; 3.8 is chosen here as the range's
    /// midpoint, absent any legacy precedent to match instead.
    pub const ASCII_CODE: Self = Self {
        bytes_per_char: 1.0,
        chars_per_token: 3.8,
    };

    /// Cyrillic text encoded as UTF-8. BLUEPRINT §E.1 gives `b ≈ 1.9` bytes/char
    /// (each Cyrillic codepoint takes 2 bytes in UTF-8, averaged down slightly by the
    /// ASCII punctuation and whitespace mixed into real text). `chars_per_token` of
    /// 2.5 is a natural-language estimate: BPE tokenizer vocabularies are trained
    /// overwhelmingly on English/Latin-script corpora, so Cyrillic prose tokenizes
    /// less efficiently (more tokens per character) than ASCII code does — a lower
    /// chars-per-token ratio than the ASCII preset. No legacy precedent exists for
    /// this value either.
    pub const CYRILLIC_UTF8: Self = Self {
        bytes_per_char: 1.9,
        chars_per_token: 2.5,
    };
}

/// The legacy fallback estimate: a flat 3.5-bytes-per-token ratio applied uniformly,
/// regardless of content. This is `utils/text_utils.py::estimate_tokens` in the
/// archived Python implementation, ported verbatim.
///
/// BLUEPRINT §E.1's prose describes this as a ceiling, `⌈B / 3.5⌉`. The archive is the
/// actual behavioral oracle (`.ai/project/14-legacy-reference.md`) and its literal
/// source reads `max(1, round(byte_count / 3.5))` — a nearest-integer round, not a
/// ceiling. The two disagree whenever the fractional part of `B / 3.5` is `<= 0.5`
/// (e.g. `B = 4` gives `round(1.142..) = 1` but `ceil(1.142..) = 2`). This function
/// ports the archive's `round`, not the prose's `ceil` — do not "fix" this back to a
/// ceiling; doing so would silently diverge from what the legacy tool actually
/// reported.
pub fn estimate_tokens_fallback(byte_count: u64) -> u64 {
    let estimate = (byte_count as f64 / 3.5).round();
    (estimate as u64).max(1)
}

/// Refines [`estimate_tokens_fallback`] with content-aware coefficients (BLUEPRINT
/// §E.1): `T ≈ B / (bytes_per_char * chars_per_token)`, floored at 1 token. This is a
/// new capability with no legacy precedent — the archive only ever used the flat 3.5
/// ratio for every file. [`estimate_tokens_fallback`] remains the crate's
/// legacy-compatible baseline and this function never replaces it anywhere in this
/// crate's API (invariant I4, `docs/architecture/invariants.md`): callers that want
/// the legacy-compatible number keep calling the fallback directly.
pub fn estimate_tokens_refined(byte_count: u64, coefficients: TokenCoefficients) -> u64 {
    let denominator = coefficients.bytes_per_char * coefficients.chars_per_token;
    let estimate = (byte_count as f64 / denominator).round();
    (estimate as u64).max(1)
}

#[cfg(test)]
mod tests {
    use super::{TokenCoefficients, estimate_tokens_fallback, estimate_tokens_refined};

    #[test]
    fn fallback_floors_at_one_token() {
        assert_eq!(estimate_tokens_fallback(0), 1);
        assert_eq!(estimate_tokens_fallback(1), 1);
    }

    #[test]
    fn fallback_uses_round_not_ceil() {
        // 4 / 3.5 = 1.142857..., round() = 1, ceil() would be 2. This byte count is
        // deliberately chosen so the two formulas disagree, proving the fallback is
        // actually rounding rather than accidentally matching a ceiling.
        assert_eq!(estimate_tokens_fallback(4), 1);
        // 6 / 3.5 = 1.714285..., round() = 2, matching ceil() here — not the
        // discriminating case, kept only as an ordinary sanity check.
        assert_eq!(estimate_tokens_fallback(6), 2);
        // 1000 / 3.5 = 285.714285..., round() = 286.
        assert_eq!(estimate_tokens_fallback(1000), 286);
    }

    #[test]
    fn refined_matches_fallback_shape_for_ascii_density() {
        // bytes_per_char = 1.0 keeps the character count equal to the byte count, so
        // only chars_per_token (3.8 vs 3.5) changes the result versus the fallback.
        let refined = estimate_tokens_refined(1000, TokenCoefficients::ASCII_CODE);
        assert_eq!(refined, (1000.0_f64 / 3.8).round() as u64);
    }

    #[test]
    fn refined_is_meaningfully_lower_than_fallback_for_cyrillic_utf8() {
        let byte_count = 1000;
        let fallback = estimate_tokens_fallback(byte_count);
        let refined = estimate_tokens_refined(byte_count, TokenCoefficients::CYRILLIC_UTF8);

        // Fallback treats every byte as if it were 3.5 bytes/token of ASCII-ish
        // content. Cyrillic UTF-8 is ~1.9 bytes/char, so the same byte count holds
        // fewer characters than the fallback assumes, and thus fewer tokens.
        assert_eq!(fallback, 286);
        assert_eq!(refined, 211);
        assert!(refined < fallback);
        assert!(
            fallback - refined >= 50,
            "expected a meaningfully lower estimate"
        );
    }

    #[test]
    fn refined_floors_at_one_token() {
        assert_eq!(estimate_tokens_refined(0, TokenCoefficients::ASCII_CODE), 1);
        assert_eq!(
            estimate_tokens_refined(1, TokenCoefficients::CYRILLIC_UTF8),
            1
        );
    }
}
