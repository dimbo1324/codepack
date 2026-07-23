//! Provider-specific secret signatures (BLUEPRINT §B.1, 🎯 new capability — legacy has
//! only the PEM-header rule, reused here as `pem-private-key`). High-precision,
//! low-false-positive patterns anchored to a known vendor token format. **No network
//! calls**: every rule is a pure regex match against text already in memory (invariant
//! I1 — this is exactly why AWS/GitHub/etc. keys are recognised by shape, never
//! validated against the provider's API).
//!
//! Four patterns (AWS, Google, Slack, JWT) are given verbatim in `BLUEPRINT.md` §B.1;
//! the rest are authored from each provider's publicly documented token format, not
//! ported from legacy or verified against any live account.

use std::sync::LazyLock;

use regex::Regex;

use crate::patterns::keyword::PRIVATE_KEY_RE;

pub struct ProviderRule {
    pub rule_id: &'static str,
    pub confidence: &'static str,
    pub regex: Regex,
}

/// Order is load-bearing: `anthropic-api-key` (`sk-ant-…`) is checked strictly before
/// `openai-api-key` (`sk-…`) so an Anthropic-shaped key is never reclassified — see
/// [`find_provider_matches`].
pub static PROVIDER_PATTERNS: LazyLock<Vec<ProviderRule>> = LazyLock::new(|| {
    vec![
        ProviderRule {
            rule_id: "aws-access-key-id",
            confidence: "critical",
            regex: Regex::new(r"AKIA[0-9A-Z]{16}")
                .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        ProviderRule {
            rule_id: "github-token",
            confidence: "critical",
            regex: Regex::new(r"gh[pous]_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{82}")
                .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        ProviderRule {
            rule_id: "google-api-key",
            confidence: "critical",
            regex: Regex::new(r"AIza[0-9A-Za-z\-_]{35}")
                .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        ProviderRule {
            rule_id: "slack-token",
            confidence: "critical",
            regex: Regex::new(r"xox[baprs]-[0-9A-Za-z-]{10,72}")
                .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        ProviderRule {
            rule_id: "stripe-key",
            confidence: "critical",
            regex: Regex::new(r"(sk|rk)_live_[0-9A-Za-z]{24,}")
                .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        ProviderRule {
            rule_id: "anthropic-api-key",
            confidence: "critical",
            regex: Regex::new(r"sk-ant-[A-Za-z0-9_-]{20,}")
                .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        ProviderRule {
            rule_id: "openai-api-key",
            confidence: "high",
            regex: Regex::new(r"sk-[A-Za-z0-9]{20,}")
                .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        ProviderRule {
            rule_id: "jwt",
            confidence: "high",
            regex: Regex::new(r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+")
                .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        ProviderRule {
            rule_id: "pem-private-key",
            confidence: "critical",
            regex: PRIVATE_KEY_RE.clone(),
        },
    ]
});

/// `telegram-bot-token` is scanned separately from [`PROVIDER_PATTERNS`] — see
/// `patterns::prefilter`'s module doc for why: unlike every other provider signature it
/// has no distinctive literal prefix (`\d{8,10}:…` starts with plain digits), so the
/// `aho-corasick` prefilter cannot anchor on it and it must run unconditionally.
pub static TELEGRAM_BOT_TOKEN_RULE: LazyLock<ProviderRule> = LazyLock::new(|| ProviderRule {
    rule_id: "telegram-bot-token",
    confidence: "critical",
    regex: Regex::new(r"\d{8,10}:[A-Za-z0-9_-]{35}")
        .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderMatch {
    pub rule_id: &'static str,
    pub confidence: &'static str,
    pub start: usize,
    pub end: usize,
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start < b_end && b_start < a_end
}

/// Runs every rule in [`PROVIDER_PATTERNS`] over `line`, in priority order. A match
/// whose span overlaps an already-accepted match from a higher-priority rule is
/// dropped — this is what keeps `openai-api-key` from ever double-claiming a span
/// already claimed by `anthropic-api-key` (both start with `sk-`).
pub fn find_provider_matches(line: &str) -> Vec<ProviderMatch> {
    let mut matches: Vec<ProviderMatch> = Vec::new();
    for rule in PROVIDER_PATTERNS.iter() {
        for found in rule.regex.find_iter(line) {
            let overlaps = matches.iter().any(|existing| {
                ranges_overlap(existing.start, existing.end, found.start(), found.end())
            });
            if overlaps {
                continue;
            }
            matches.push(ProviderMatch {
                rule_id: rule.rule_id,
                confidence: rule.confidence,
                start: found.start(),
                end: found.end(),
            });
        }
    }
    matches
}

/// See [`TELEGRAM_BOT_TOKEN_RULE`] — always run, never gated by the prefilter.
pub fn find_telegram_matches(line: &str) -> Vec<ProviderMatch> {
    TELEGRAM_BOT_TOKEN_RULE
        .regex
        .find_iter(line)
        .map(|found| ProviderMatch {
            rule_id: TELEGRAM_BOT_TOKEN_RULE.rule_id,
            confidence: TELEGRAM_BOT_TOKEN_RULE.confidence,
            start: found.start(),
            end: found.end(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule_ids(line: &str) -> Vec<&'static str> {
        find_provider_matches(line)
            .into_iter()
            .map(|m| m.rule_id)
            .collect()
    }

    #[test]
    fn ten_provider_rules_total_including_telegram() {
        assert_eq!(PROVIDER_PATTERNS.len() + 1, 10);
    }

    #[test]
    fn aws_access_key_matches() {
        assert_eq!(rule_ids("AKIAABCDEFGHIJKLMNOP"), vec!["aws-access-key-id"]);
    }

    #[test]
    fn github_token_variants_match() {
        let ghp = "ghp_".to_string() + &"a".repeat(36);
        assert_eq!(rule_ids(&ghp), vec!["github-token"]);
        let pat = "github_pat_".to_string() + &"b".repeat(82);
        assert_eq!(rule_ids(&pat), vec!["github-token"]);
    }

    #[test]
    fn google_api_key_matches() {
        let key = "AIza".to_string() + &"a".repeat(35);
        assert_eq!(rule_ids(&key), vec!["google-api-key"]);
    }

    #[test]
    fn slack_token_matches() {
        let token = "xoxb-".to_string() + &"1".repeat(20);
        assert_eq!(rule_ids(&token), vec!["slack-token"]);
    }

    #[test]
    fn stripe_key_matches() {
        let key = "sk_live_".to_string() + &"a".repeat(24);
        assert_eq!(rule_ids(&key), vec!["stripe-key"]);
    }

    #[test]
    fn anthropic_key_wins_over_openai_pattern() {
        let key = "sk-ant-".to_string() + &"a".repeat(30);
        let hits = find_provider_matches(&key);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rule_id, "anthropic-api-key");
        assert_eq!(hits[0].confidence, "critical");
    }

    #[test]
    fn openai_key_matches_when_not_anthropic_shaped() {
        let key = "sk-".to_string() + &"a".repeat(30);
        assert_eq!(rule_ids(&key), vec!["openai-api-key"]);
    }

    #[test]
    fn jwt_matches() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.dGVzdC1zaWduYXR1cmU";
        assert_eq!(rule_ids(jwt), vec!["jwt"]);
    }

    #[test]
    fn pem_private_key_matches_via_provider_rule_too() {
        assert_eq!(
            rule_ids("-----BEGIN RSA PRIVATE KEY-----"),
            vec!["pem-private-key"]
        );
    }

    #[test]
    fn telegram_bot_token_matches_via_dedicated_function() {
        let token = "123456789:".to_string() + &"a".repeat(35);
        assert_eq!(
            find_telegram_matches(&token)
                .into_iter()
                .map(|m| m.rule_id)
                .collect::<Vec<_>>(),
            vec!["telegram-bot-token"]
        );
        assert!(find_provider_matches(&token).is_empty());
    }

    #[test]
    fn no_match_on_plain_text() {
        assert!(find_provider_matches("just an ordinary line of code").is_empty());
    }
}
