//! Invariant I3 audit (`.ai/project/12-domain-rules.md`): no `Finding` produced by any
//! detector — keyword cascade, provider signature, or entropy — may contain a
//! substring of the original unredacted secret value anywhere in its **serialized**
//! (`.json`) output. Every fixture value below is synthetic and clearly fake (never a
//! real leaked credential), but format-correct enough to exercise each detector.

use std::fs;
use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use codepack_security::scan::write::write_json_report;
use codepack_security::scan_project;

fn write_file(root: &Path, relative: &str, content: &str) -> PathBuf {
    let full = root.join(relative);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&full, content).unwrap();
    PathBuf::from(relative)
}

/// One secret value per detector family, each planted in a line shaped so that
/// *only* that family's detector would plausibly fire on it (bare tokens for
/// provider/entropy, keyword-shaped assignments for the keyword cascade).
const SECRET_VALUES: &[(&str, &str)] = &[
    // Keyword cascade (SECRET_PATTERNS): keyword=value shape.
    ("keyword-assignment", "API_KEY=fake-super-secret-value-0001"),
    // Provider signatures: bare tokens, no keyword anywhere on the line.
    ("aws", "AKIAABCDEFGHIJKLMNOPQRST"),
    ("github", "ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("google", "AIzaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("slack", "xoxb-1111111111111111111"),
    // Split across a `concat!` so no single string literal in the source text
    // matches GitHub's push-protection Stripe-key pattern (this is a synthetic,
    // clearly-fake all-`a` fixture, never a real key, but the shape alone trips
    // format-only secret scanners).
    ("stripe", concat!("sk_live_", "aaaaaaaaaaaaaaaaaaaaaaaa")),
    ("anthropic", "sk-ant-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("openai", "sk-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    (
        "jwt",
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.dGVzdC1zaWduYXR1cmU",
    ),
    ("telegram", "123456789:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("pem", "-----BEGIN RSA PRIVATE KEY-----"),
    // Entropy-only: a high-entropy bare token with no keyword and no provider shape.
    ("entropy", "aZ9kQ2wLp7xR4tY8mN1cJ6hF3sD0eU"),
];

#[test]
fn no_finding_json_ever_contains_a_raw_secret_substring() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = Vec::new();
    for (name, value) in SECRET_VALUES {
        files.push(write_file(
            dir.path(),
            &format!("{name}.txt"),
            &format!("{value}\n"),
        ));
    }

    let result = scan_project(dir.path(), &files, None, &CancellationToken::new()).unwrap();
    assert!(
        result.findings.len() >= SECRET_VALUES.len(),
        "expected at least one finding per planted secret, got {}",
        result.findings.len()
    );

    let json_path = dir.path().join("06_security_scan.json");
    write_json_report(&result, &json_path).unwrap();
    let json_text = fs::read_to_string(&json_path).unwrap();

    for (name, value) in SECRET_VALUES {
        assert!(
            !json_text.contains(value),
            "serialized JSON leaks the raw {name} secret value"
        );
    }

    // Also assert directly against the in-memory `Finding.message` field, independent
    // of the JSON writer, so the invariant holds for every future artifact format too
    // (`.txt`, `.sarif`), not only the current `.json` writer's serialization.
    for finding in &result.findings {
        for (name, value) in SECRET_VALUES {
            assert!(
                !finding.message.contains(value),
                "Finding.message for rule {:?} leaks the raw {name} secret value",
                finding.rule
            );
        }
    }
}
