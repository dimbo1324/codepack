//! Tests for [`super`], the heuristic scanner.
//!
//! Split out of `mod.rs` on 2026-07-27: that file had grown to 890 lines, past the
//! project's own ~600-line limit (`.ai/project/12-domain-rules.md`), and the tests
//! were the larger half. Same remedy already applied to `commands/export.rs`.

use super::*;
use std::io::Write;

fn write_file(dir: &Path, relative: &str, content: &str) -> PathBuf {
    let full = dir.join(relative);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = fs::File::create(&full).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    PathBuf::from(relative)
}

fn no_cancel() -> CancellationToken {
    CancellationToken::new()
}

#[test]
fn sensitive_file_and_secret_line_and_risky_code_all_detected() {
    let dir = tempfile::tempdir().unwrap();
    let env = write_file(dir.path(), ".env", "API_KEY=abcdef0123456789\n");
    let script = write_file(
        dir.path(),
        "app.py",
        "eval(user_input)\nAKIAABCDEFGHIJKLMNOP\n",
    );

    let result = scan_project(dir.path(), &[env, script], None, &no_cancel()).unwrap();

    assert!(
        result
            .findings
            .iter()
            .any(|f| f.kind == FindingKind::SensitiveFile && f.severity == "critical")
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.kind == FindingKind::PotentialSecret && f.rule == "secret_like_line")
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.kind == FindingKind::PotentialSecret && f.rule == "aws-access-key-id")
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.kind == FindingKind::RiskyCode && f.rule == "python-eval")
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.kind == FindingKind::RiskyCode && f.rule == "js-eval")
    );
    assert_eq!(result.summary.total_findings, result.findings.len());
}

#[test]
fn self_protection_suppresses_own_source_lines() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(
        dir.path(),
        "notes.txt",
        "// see SECRET_PATTERNS and redact_secrets for details\n",
    );

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    assert!(
        result
            .findings
            .iter()
            .all(|f| f.kind != FindingKind::PotentialSecret)
    );
}

#[test]
fn binary_files_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let full = dir.path().join("data.bin");
    fs::write(&full, [0u8, 1, 2, 3, 0, 0, 0]).unwrap();

    let result =
        scan_project(dir.path(), &[PathBuf::from("data.bin")], None, &no_cancel()).unwrap();
    assert!(result.findings.is_empty());
}

#[test]
fn no_raw_secret_value_ever_reaches_a_finding_message() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(
        dir.path(),
        "config.py",
        "API_KEY = \"super-secret-value-xyz\"\n",
    );

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    for finding in &result.findings {
        assert!(!finding.message.contains("super-secret-value-xyz"));
    }
}

#[test]
fn bare_provider_signature_with_no_adjacent_keyword_is_still_redacted() {
    // Regression test for a real I3 violation found during S3 integration:
    // `keyword::redacted_line` alone only redacts keyword-shaped `key=value`
    // spans. A bare AWS-shaped key with no keyword anywhere on the line has no
    // such span for it to act on, so without `mask_non_keyword_secret_spans` the
    // raw key text passed straight through into `Finding.message`.
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "notes.txt", "AKIAABCDEFGHIJKLMNOP\n");

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    let hit = result
        .findings
        .iter()
        .find(|f| f.rule == "aws-access-key-id")
        .expect("aws-access-key-id finding present");
    assert!(!hit.message.contains("AKIAABCDEFGHIJKLMNOP"));
}

#[test]
fn bare_high_entropy_token_with_no_adjacent_keyword_is_still_redacted() {
    let dir = tempfile::tempdir().unwrap();
    let token = "aZ9kQ2wLp7xR4tY8mN1cJ6hF3sD0eU";
    let file = write_file(dir.path(), "notes.txt", &format!("{token}\n"));

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    let hit = result
        .findings
        .iter()
        .find(|f| f.rule == "high-entropy-token")
        .expect("high-entropy-token finding present");
    assert!(!hit.message.contains(token));
}

#[test]
fn overlapping_provider_and_entropy_spans_mask_the_full_extent() {
    // Regression test for a real I3 violation found during S3 review:
    // `mask_non_keyword_secret_spans` sorted spans by start and, on finding an
    // overlap, dropped the later span entirely instead of extending the
    // redaction to cover it. A short fixed-length provider match (the AWS key,
    // 20 chars) glued directly (no separator) to more characters makes the
    // entropy tokenizer see one long token starting at the same offset but
    // extending well past the provider match's end — the tail used to leak
    // into every `Finding.message` on the line, including the entropy
    // detector's own finding about that exact span.
    let dir = tempfile::tempdir().unwrap();
    let secret = "AKIAABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789zzzzTAILSECRETPORTION";
    let file = write_file(dir.path(), "notes.txt", &format!("{secret}\n"));

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    assert!(
        !result.findings.is_empty(),
        "expected at least one secret finding on this line"
    );
    for finding in &result.findings {
        assert!(!finding.message.contains("TAILSECRETPORTION"));
        assert!(!finding.message.contains(secret));
    }
}

#[test]
fn keyword_hit_suppresses_a_duplicate_entropy_hit_on_the_same_line() {
    // Golden-parity regression (fixture `python_app`, `app/main.py:5`, and fixture
    // `mixed_stack`, `docker-compose.yml:10`): the line was reported twice, once as
    // `secret_like_line` and once as `high-entropy-token`, on the identical
    // file+line span. Legacy emits exactly one `SecretFinding` per line.
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(
        dir.path(),
        "main.py",
        "SECRET_TOKEN = \"placeholder-token-for-fixture-only\"\n",
    );

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    let secrets: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::PotentialSecret)
        .collect();
    assert_eq!(
        secrets.len(),
        1,
        "expected exactly one potential-secret finding, got {secrets:?}"
    );
    assert_eq!(secrets[0].rule, "secret_like_line");
}

#[test]
fn entropy_hit_survives_on_a_line_the_keyword_cascade_does_not_flag() {
    // The other half of the suppression rule: the recall gain must not be lost. This
    // line carries no keyword root anywhere, so the keyword cascade is silent and the
    // entropy detector is the only thing that can see it.
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "notes.txt", "aZ9kQ2wLp7xR4tY8mN1cJ6hF3sD0eU\n");

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.rule == "high-entropy-token"),
        "entropy findings must survive on lines the keyword cascade misses"
    );
}

#[test]
fn provider_hit_survives_on_a_line_the_keyword_cascade_does_not_flag() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "notes.txt", "AKIAABCDEFGHIJKLMNOP\n");

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.rule == "aws-access-key-id")
    );
}

#[test]
fn redaction_keeps_the_key_name_that_identifies_the_finding() {
    // Golden-parity regression (fixture `mixed_stack`, `docker-compose.yml:10`): the
    // entropy tokenizer's alphabet includes `=`, so `JWT_SECRET=<value>` is a single
    // token; masking that span before `keyword::redacted_line` wiped `JWT_SECRET=`
    // too and produced a message — `- <REDACTED>` — that no longer said which secret
    // was found. Legacy's message is `- JWT_SECRET=<REDACTED>`.
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(
        dir.path(),
        "docker-compose.yml",
        "      - JWT_SECRET=fixture-placeholder-value\n",
    );

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    let secret = result
        .findings
        .iter()
        .find(|f| f.kind == FindingKind::PotentialSecret)
        .expect("a potential-secret finding on the JWT_SECRET line");
    assert_eq!(secret.message, "- JWT_SECRET=<REDACTED>");
    assert!(!secret.message.contains("fixture-placeholder-value"));
}

#[test]
fn redaction_matches_legacy_when_only_the_value_carries_the_keyword() {
    // Golden-parity regression (fixture `python_app`, `app/main.py:5`). Legacy's
    // `SECRET_KEY_PATTERN` matches `\btoken\b` inside the *value*
    // (`placeholder-token-for-fixture-only`), not inside `SECRET_TOKEN` (where `_`
    // blocks the word boundary on both roots) — which is why legacy reports this line
    // at `low` confidence and collapses it to `SECRET_TOKEN=<REDACTED>`. Masking the
    // value first deleted that `token` substring, the collapse never fired, and the
    // message came out as the uncollapsed `SECRET_TOKEN = "<REDACTED>"`.
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(
        dir.path(),
        "main.py",
        "SECRET_TOKEN = \"placeholder-token-for-fixture-only\"\n",
    );

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    let secret = result
        .findings
        .iter()
        .find(|f| f.kind == FindingKind::PotentialSecret)
        .expect("a potential-secret finding on the SECRET_TOKEN line");
    assert_eq!(secret.confidence, "low");
    assert_eq!(secret.message, "SECRET_TOKEN=<REDACTED>");
}

#[test]
fn provider_token_in_the_surviving_key_prefix_is_still_masked() {
    // The residual pass in `redacted_message` is not decoration: `redacted_line`
    // keeps everything before the first `=`/`:`, so a provider token sitting in that
    // prefix reaches the message untouched in legacy. I3 forbids that here.
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(
        dir.path(),
        "notes.txt",
        "AKIAABCDEFGHIJKLMNOP token: value\n",
    );

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    for finding in &result.findings {
        assert!(
            !finding.message.contains("AKIAABCDEFGHIJKLMNOP"),
            "provider token leaked through the surviving key prefix: {}",
            finding.message
        );
    }
}

// --- Finding 2 (2026-07-27 audit): the scanner now sees connection-string
// passwords and Basic/Digest auth, neither of which any prior detector caught. ---

#[test]
fn a_password_inside_a_connection_string_is_now_found() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(
        dir.path(),
        "app.py",
        "db.connect('postgres://admin:hunter2fakepass@host/db?sslmode=require')\n",
    );

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    let hit = result
        .findings
        .iter()
        .find(|f| f.rule == "url-credentials")
        .expect("url-credentials finding present");
    assert!(!hit.message.contains("hunter2fakepass"));
}

#[test]
fn a_basic_auth_header_is_now_found() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(
        dir.path(),
        "app.py",
        "headers = {'Authorization': 'Basic ZmFrZXVzZXI6ZmFrZXBhc3N3b3Jk'}\n",
    );

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    let hit = result
        .findings
        .iter()
        .find(|f| f.rule == "http-auth-credentials")
        .expect("http-auth-credentials finding present");
    assert!(!hit.message.contains("ZmFrZXVzZXI6ZmFrZXBhc3N3b3Jk"));
}

#[test]
fn a_url_with_no_credentials_is_not_flagged() {
    // The audit's own suggested negative: the '@' sits in the path, not the
    // authority, because a '/' appears first -- structurally not a credential.
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "notes.txt", "http://host/a:b@c\n");

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    assert!(
        result.findings.iter().all(|f| f.rule != "url-credentials"),
        "a path that only looks like userinfo must not be flagged"
    );
}

#[test]
fn the_audits_own_reproduction_is_now_fully_detected() {
    // AUDIT-2026-07-27.md, finding 2's table: of four planted secrets, only two were
    // found before this fix. All four must be found now.
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(
        dir.path(),
        "app.py",
        "db.connect('postgres://admin:hunter2fakepass@host/db?sslmode=require')\n\
         AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n\
         headers = {'Authorization': 'Basic ZmFrZXVzZXI6ZmFrZXBhc3N3b3Jk'}\n\
         api_key = \"sk-projFAKEfghijklmnopqrstuvwxyz1234567890ABCD\"\n",
    );

    let result = scan_project(dir.path(), &[file], None, &no_cancel()).unwrap();
    let secrets: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::PotentialSecret)
        .collect();
    assert_eq!(
        secrets.len(),
        4,
        "expected all four planted secrets to be found, got {secrets:?}"
    );
    for finding in &secrets {
        for raw in [
            "hunter2fakepass",
            "AKIAIOSFODNN7EXAMPLE",
            "ZmFrZXVzZXI6ZmFrZXBhc3N3b3Jk",
            "sk-projFAKEfghijklmnopqrstuvwxyz1234567890ABCD",
        ] {
            assert!(
                !finding.message.contains(raw),
                "{raw} leaked into a finding message: {finding:?}"
            );
        }
    }
}

#[test]
fn cancellation_is_checked_inside_the_file_loop() {
    let dir = tempfile::tempdir().unwrap();
    let files: Vec<PathBuf> = (0..3)
        .map(|i| write_file(dir.path(), &format!("file{i}.txt"), "nothing interesting\n"))
        .collect();

    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = scan_project(dir.path(), &files, None, &cancel);
    assert!(matches!(result, Err(SecurityError::Cancelled)));
}
