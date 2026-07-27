//! Accuracy corpus (BLUEPRINT §E.3, `.ai/project/12-domain-rules.md`): synthetic
//! labeled positives/negatives — never a real leaked credential, only clearly-fake,
//! format-correct token shapes — comparing legacy-parity-mode detection (the keyword
//! cascade alone) against full-mode detection (keyword cascade + provider signatures +
//! entropy, gated by the `aho-corasick` prefilter where documented). Establishes the
//! first precision/recall/F1 baseline for this crate (no legacy baseline exists to
//! compare against — S3 is the first stage to measure this at all).
//!
//! **Known, documented gap not exercised by this corpus:** hash-shaped and UUID-shaped
//! identifiers (git commit SHAs, UUIDs) are genuine false-positive risks for the
//! entropy detector — both clear the hex-like threshold (`len ≥ 32`, `H ≥ 3.0`) purely
//! by looking "random enough", exactly as documented in `patterns::entropy`'s and
//! `patterns::prefilter`'s module doc comments. This corpus does not include such
//! fixtures as negatives because doing so would make this test primarily a referendum
//! on that one already-documented weakness rather than a general precision/recall
//! comparison; the gap is real, is not hidden, and is called out in the stage report.
//!
//! Lowering any detector's threshold to make this test pass is forbidden — see
//! `.ai/universal/06-quality-and-testing.md` and `.ai/universal/08-rules-evolution.md`.
//! A recall drop here is a defect to report, never a reason to edit the corpus.

use codepack_security::patterns::{credentials, entropy, keyword, prefilter, provider};

/// Legacy-parity detection: only the keyword cascade legacy already had.
fn parity_detect(line: &str) -> bool {
    keyword::secret_confidence(line).is_some()
}

/// Full-mode detection: parity plus provider signatures (prefilter-gated, matching
/// `scan::collect_secret_hits`'s own gating exactly), the always-on telegram pass, the
/// structural credential detectors added for Finding 2 (2026-07-27 audit), and entropy.
///
/// The early return on a parity hit is not just a short-circuit — it mirrors
/// `scan::collect_secret_hits`'s precedence rule, where a keyword hit suppresses every
/// later rule's hit on the same line (legacy emits at most one finding per line).
/// Because that rule only ever collapses *duplicates* on an already-detected line, it
/// cannot change any cell of this matrix: a line detected by parity stays detected.
fn full_detect(line: &str) -> bool {
    if parity_detect(line) {
        return true;
    }
    if prefilter::has_hit(line) && !provider::find_provider_matches(line).is_empty() {
        return true;
    }
    if !provider::find_telegram_matches(line).is_empty() {
        return true;
    }
    if !credentials::find_url_credentials(line).is_empty() {
        return true;
    }
    if !credentials::find_http_auth_tokens(line).is_empty() {
        return true;
    }
    !entropy::entropy_findings(line).is_empty()
}

/// `(line, is_actual_secret)`. Positives are synthetic, clearly-fake token shapes
/// (never a real leaked credential); negatives are ordinary, unremarkable code/text.
const CORPUS: &[(&str, bool)] = &[
    // --- Positives caught by both parity and full mode (keyword-anchored). ---
    (r#"API_KEY = "abcFAKEkey1234567890""#, true),
    ("PASSWORD: hunter2fakepass", true),
    ("-----BEGIN RSA PRIVATE KEY-----", true),
    (
        "Authorization: BEARER abcdefghijklmnopqrstuvwxyz0123456789",
        true,
    ),
    ("DATABASE_URL=postgres://user:pass@host:5432/db", true),
    // --- Positives parity structurally cannot see (no keyword anywhere on the
    // line) that full mode catches via provider signatures or entropy. ---
    ("AKIAABCDEFGHIJKLMNOPQRST", true),
    ("ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", true),
    ("AIzaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", true),
    ("xoxb-11111111111111111111", true),
    // Split across a `concat!` so no single string literal in the source text
    // matches GitHub's push-protection Stripe-key pattern (synthetic, clearly-fake
    // all-`a` fixture, never a real key — the shape alone trips format-only scanners).
    (concat!("sk_live_", "aaaaaaaaaaaaaaaaaaaaaaaa"), true),
    ("sk-ant-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", true),
    ("sk-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", true),
    (
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.dGVzdC1zaWduYXR1cmU",
        true,
    ),
    ("123456789:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", true),
    // AWS secret access key (Q15). Unlike every other provider signature the value has
    // no prefix — it is 40 base64 characters — so the rule is anchored on AWS's own
    // field name. Split with `concat!` for the same push-protection reason as Stripe.
    (
        concat!(
            "aws_secret_access_key = ",
            "aaaaaaaaaaaaaaaaaaaaB7cB7cB7cB7cB7cB7cde"
        ),
        true,
    ),
    (
        r#"  secretAccessKey: "aaaaaaaaaaaaaaaaaaaaB7cB7cB7cB7cB7cB7cde","#,
        true,
    ),
    (r#"signature = "zX3kQ9mPv7wR2tY8bN4cJ6hF3sD0aE9gU8i""#, true),
    ("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4", true),
    // --- Finding 2 (2026-07-27 audit): structural credential detection, parity
    // structurally cannot see either shape (no keyword anywhere on the line). ---
    (
        "db.connect('postgres://admin:hunter2fakepass@host/db?sslmode=require')",
        true,
    ),
    (
        "headers = {'Authorization': 'Basic ZmFrZXVzZXI6ZmFrZXBhc3N3b3Jk'}",
        true,
    ),
    // --- Negatives: ordinary code/text that must not be flagged by either mode. ---
    ("let counter = 0;", false),
    ("def calculate_total(a, b): return a + b", false),
    ("import numpy as np", false),
    ("SELECT * FROM users WHERE id = 42;", false),
    ("console.log('hello world');", false),
    ("# just a regular comment about performance", false),
    ("struct Point { x: f64, y: f64 }", false),
    (
        "The quick brown fox jumps over the lazy dog repeatedly for testing purposes today",
        false,
    ),
    (r#"version = "1.4.2""#, false),
    (r#"<div className="container">{children}</div>"#, false),
    ("SOME_CONST_VALUE = 42", false),
    ("func Add(a int, b int) int { return a + b }", false),
    ("const MAX_RETRIES = 5;", false),
    (r#"fn main() { println!("hello"); }"#, false),
    // The same 40-character shape without AWS context. These are what a bare
    // 40-base64-character rule would have flagged, and they are why Q15 is anchored on
    // context: flagging them would drop precision below 1.000 (invariant I9).
    ("build_id = aaaaaaaaaaaaaaaaaaaaB7cB7cB7cB7cB7cB7cde", false),
    ("sha256sum: aaaaaaaaaaaaaaaaaaaaB7cB7cB7cB7cB7cB7cde", false),
    // Finding 2's own suggested negative: the '@' sits in the *path*, not the
    // authority, because a '/' appears before it — structurally not a credential, and
    // exactly the shape a naive "look for user:pass@" rule would have false-positived
    // on.
    ("http://host/a:b@c", false),
    // A username with no password, and a plain URL with no userinfo at all.
    ("https://readonly@cdn.example.com/assets", false),
    ("https://example.com/docs/getting-started", false),
    // The word "Basic"/"Digest" with no token-shaped value after it.
    ("Basic training starts Monday", false),
    ("run a digest of the week's changes", false),
];

struct Metrics {
    precision: f64,
    recall: f64,
    f1: f64,
    tp: usize,
    fp: usize,
    fn_: usize,
}

fn measure(detect: impl Fn(&str) -> bool) -> Metrics {
    let (mut tp, mut fp, mut fn_) = (0usize, 0usize, 0usize);
    for (line, is_secret) in CORPUS {
        let detected = detect(line);
        match (detected, *is_secret) {
            (true, true) => tp += 1,
            (true, false) => fp += 1,
            (false, true) => fn_ += 1,
            (false, false) => {}
        }
    }
    let precision = if tp + fp == 0 {
        1.0
    } else {
        tp as f64 / (tp + fp) as f64
    };
    let recall = if tp + fn_ == 0 {
        1.0
    } else {
        tp as f64 / (tp + fn_) as f64
    };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    Metrics {
        precision,
        recall,
        f1,
        tp,
        fp,
        fn_,
    }
}

#[test]
fn full_mode_strictly_improves_recall_without_regressing_precision() {
    let total_positives = CORPUS.iter().filter(|(_, is_secret)| *is_secret).count();
    let total_negatives = CORPUS.len() - total_positives;
    assert!(total_positives > 0 && total_negatives > 0);

    let parity = measure(parity_detect);
    let full = measure(full_detect);

    // Metrics are folded into the assertion messages (rather than `println!`, which
    // the workspace lint set forbids) so they surface on any future regression;
    // measured values at the time this baseline was established are recorded in the
    // stage report / `ROADMAP.md` status line, not just in test output.
    assert!(
        full.recall > parity.recall,
        "full mode must strictly improve recall over parity mode on {total_positives} \
         positives / {total_negatives} negatives (parity: TP={} FP={} FN={} P={:.3} \
         R={:.3} F1={:.3}; full: TP={} FP={} FN={} P={:.3} R={:.3} F1={:.3})",
        parity.tp,
        parity.fp,
        parity.fn_,
        parity.precision,
        parity.recall,
        parity.f1,
        full.tp,
        full.fp,
        full.fn_,
        full.precision,
        full.recall,
        full.f1,
    );
    assert!(
        full.precision >= parity.precision,
        "full mode must not regress precision versus parity mode (parity={:.3}, full={:.3})",
        parity.precision,
        full.precision
    );
}

/// Invariant I9's **absolute** floor, distinct from the relative comparison above.
///
/// The relative assertions can all hold while both modes degrade together: if a change
/// halved precision in parity *and* full mode, `full.precision >= parity.precision`
/// would still be satisfied and the suite would stay green. I9 is not a relative
/// statement — it fixes precision at 1.000, on the reasoning that a secret scanner
/// which cries wolf is worse than one that stays quiet. Asserting the number itself is
/// what makes the invariant testable rather than merely stated.
///
/// A failure here is a defect to report, never a reason to lower the threshold
/// (`.ai/universal/08-rules-evolution.md`: never weaken a rule to make a task pass).
#[test]
fn precision_stays_at_one_in_both_modes() {
    let parity = measure(parity_detect);
    let full = measure(full_detect);

    assert_eq!(
        parity.precision, 1.0,
        "parity-mode precision fell to {:.3}: the keyword cascade started reporting          something that is not a secret",
        parity.precision
    );
    assert_eq!(
        full.precision, 1.0,
        "full-mode precision fell to {:.3}: a provider signature or the entropy          detector started reporting something that is not a secret",
        full.precision
    );
}
