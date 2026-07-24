//! The heuristic scanner — "Enhanced Security Scan v3" — ported from legacy
//! `reports/insights/security.py`, plus provider signatures and entropy (BLUEPRINT
//! §B.1). [`scan_project`] takes a **caller-supplied list of files**; it never walks
//! the filesystem itself (that is `codepack-scanner`'s job, S2; combining the two is
//! S9's). Reading individual file contents from disk is not "walking" and stays in
//! scope here.

mod paths;
pub mod write;

use std::fs;
use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use serde::Serialize;

use crate::classify;
use crate::constants;
use crate::error::{Result, SecurityError};
use crate::patterns::{confidence_rank, entropy, keyword, prefilter, provider, risky_code};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    SensitiveFile,
    PotentialSecret,
    RiskyCode,
}

/// Mirrors legacy's flat finding dict exactly: `type`, `severity`, `confidence`,
/// `file`, `line`, `rule`, `message`. `message` is **always** either a fixed,
/// hard-coded description or the output of [`keyword::redacted_line`] — invariant I3:
/// the raw matched substring never reaches this field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    #[serde(rename = "type")]
    pub kind: FindingKind,
    pub severity: String,
    pub confidence: String,
    pub file: String,
    pub line: Option<usize>,
    pub rule: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ScanSummary {
    pub sensitive_files: usize,
    pub potential_secrets: usize,
    pub risky_code: usize,
    pub total_findings: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ScanResult {
    pub summary: ScanSummary,
    pub findings: Vec<Finding>,
}

fn file_name_lower(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn extension_lower(path: &Path) -> String {
    path.extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Legacy `_collect_security_findings`'s sensitive-file check: `name ∈
/// SENSITIVE_FILENAMES OR suffix ∈ SENSITIVE_SUFFIXES OR name.startswith(".env")`,
/// `critical` when the name starts with `.env` or the suffix is one of
/// `{key,pem,p12,pfx}`, `high` otherwise.
fn sensitive_file_severity(relative: &Path) -> Option<&'static str> {
    let name = file_name_lower(relative);
    let suffix = extension_lower(relative);
    let is_sensitive = constants::is_sensitive_filename(&name)
        || constants::is_sensitive_suffix(&suffix)
        || name.starts_with(".env");
    if !is_sensitive {
        return None;
    }
    let critical =
        name.starts_with(".env") || matches!(suffix.as_str(), "key" | "pem" | "p12" | "pfx");
    Some(if critical { "critical" } else { "high" })
}

fn read_text_for_scan(path: &Path) -> Result<Option<String>> {
    let raw = fs::read(path).map_err(|source| SecurityError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if classify::looks_binary(&raw) {
        return Ok(None);
    }
    match String::from_utf8(raw) {
        Ok(text) => Ok(Some(text)),
        // Documented encoding-scope gap: legacy tries 6 encodings
        // (utf-8, utf-8-sig, cp1251, cp866, utf-16, latin-1); this stage reads UTF-8
        // with a lossy fallback only. The full chain's real owner is S9's text-dump
        // step — pulling in an encoding-detection dependency for S3 alone would be
        // scope creep.
        Err(err) => Ok(Some(
            String::from_utf8_lossy(&err.into_bytes()).into_owned(),
        )),
    }
}

struct SecretHit {
    rule: &'static str,
    confidence: &'static str,
}

/// Runs every secret detector over one line and applies the self-protection exemption
/// once, uniformly, across all of them (keyword, provider, entropy) — not only the
/// keyword cascade, which is the minimum legacy required.
///
/// **A keyword hit suppresses every provider/entropy hit on the same line.** Legacy's
/// `_collect_security_findings` appends at most one `SecretFinding` per line, so a second
/// finding on the identical file+line breaks golden parity outright. It also adds no
/// information: the provider and entropy detectors (BLUEPRINT §B.1, 🎯 new — legacy has
/// neither) exist to raise *recall* on lines the keyword cascade structurally cannot see,
/// not to re-report a span it already reported. Lines the keyword cascade did **not**
/// flag still yield provider/telegram/entropy hits — that is where the entire recall gain
/// lives, and it is untouched (`tests/corpus.rs`'s `full_detect` models exactly this
/// precedence).
fn collect_secret_hits(line: &str) -> Vec<SecretHit> {
    if keyword::is_self_protected(line) {
        return Vec::new();
    }

    let prefiltered = prefilter::has_hit(line);
    if prefiltered && let Some(confidence) = keyword::secret_confidence(line) {
        return vec![SecretHit {
            rule: "secret_like_line",
            confidence,
        }];
    }

    let mut hits = Vec::new();
    if prefiltered {
        for found in provider::find_provider_matches(line) {
            hits.push(SecretHit {
                rule: found.rule_id,
                confidence: found.confidence,
            });
        }
    }
    // Never gated by the prefilter — see patterns::prefilter's documented scope limits.
    for found in provider::find_telegram_matches(line) {
        hits.push(SecretHit {
            rule: found.rule_id,
            confidence: found.confidence,
        });
    }
    for found in entropy::entropy_findings(line) {
        hits.push(SecretHit {
            rule: "high-entropy-token",
            confidence: found.confidence,
        });
    }
    hits
}

/// Invariant I3 (`.ai/project/12-domain-rules.md`): a `Finding.message` must never
/// contain a raw secret value. [`keyword::redacted_line`] alone only redacts
/// keyword-shaped `key=value`/`key: value` spans — a bare provider signature or
/// entropy match with **no** adjacent keyword (for example a lone AWS key on its own
/// line) has no keyword span for it to act on and would otherwise pass through
/// untouched. This masks every provider/telegram/entropy match span with a fixed
/// placeholder, so a message is never built from text that still holds a known
/// secret-shaped span.
///
/// **Runs *after* [`keyword::redacted_line`], never before** (see
/// [`redacted_message`]). Running it first destroys information legacy's redaction
/// depends on: the entropy tokenizer's candidate alphabet includes `=`, so
/// `JWT_SECRET=<value>` is one single token, and masking that span wipes the key name
/// the finding exists to identify — leaving a useless `- <REDACTED>`. It equally erases
/// the keyword text `redacted_line` needs to recognise the line as secret-shaped at all.
fn mask_non_keyword_secret_spans(line: &str) -> std::borrow::Cow<'_, str> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    if prefilter::has_hit(line) {
        for found in provider::find_provider_matches(line) {
            spans.push((found.start, found.end));
        }
    }
    for found in provider::find_telegram_matches(line) {
        spans.push((found.start, found.end));
    }
    for found in entropy::entropy_findings(line) {
        spans.push((found.start, found.end));
    }
    if spans.is_empty() {
        return std::borrow::Cow::Borrowed(line);
    }

    spans.sort_by_key(|&(start, _)| start);
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0usize;
    for (start, end) in spans {
        if start < cursor {
            // Overlapping with an already-masked span (can happen if two detectors
            // claim intersecting ranges that are not simple containment — e.g. a
            // short fixed-length provider match and a longer entropy-tokenized span
            // sharing the same start but extending further). Extend the already-open
            // redaction to cover the further end instead of dropping this span: the
            // overlapping bytes are already excluded from `out` by the advanced
            // `cursor`, so widening it is sufficient to keep the rest masked too.
            cursor = cursor.max(end);
            continue;
        }
        out.push_str(&line[cursor..start]);
        out.push_str("<REDACTED>");
        cursor = end;
    }
    out.push_str(&line[cursor..]);
    std::borrow::Cow::Owned(out)
}

/// Builds the single [`Finding::message`] shared by every hit on one line.
///
/// Two passes, in this order and no other:
///
/// 1. [`keyword::redacted_line`] on the **raw** line — legacy `redacted_line` verbatim.
///    Whenever the line is keyword-shaped this collapses it to `key=<REDACTED>` /
///    `key: <REDACTED>`, keeping the key name that identifies *which* secret was found
///    and dropping everything from the first `=`/`:` onward, value included.
/// 2. [`mask_non_keyword_secret_spans`] on the result — a residual safety net for what
///    step 1 legitimately leaves behind: the whole line when it holds no keyword at all
///    (a bare AWS key, a lone high-entropy blob), and the surviving key prefix, which on
///    a line such as `AKIA… token: x` would otherwise carry a provider token into the
///    message exactly as legacy does. This is a deliberate strengthening over legacy in
///    favour of invariant I3, and the only respect in which this message can differ from
///    legacy's.
fn redacted_message(line: &str) -> String {
    let legacy = keyword::redacted_line(line);
    mask_non_keyword_secret_spans(&legacy).into_owned()
}

struct RiskyHit {
    severity: &'static str,
    rule: &'static str,
    explanation: &'static str,
}

fn collect_risky_hits(line: &str) -> Vec<RiskyHit> {
    risky_code::RISKY_CODE_PATTERNS
        .iter()
        .filter(|rule| rule.regex.is_match(line))
        .map(|rule| RiskyHit {
            severity: rule.severity,
            rule: rule.rule_id,
            explanation: rule.explanation,
        })
        .collect()
}

struct FileRecord {
    severity: &'static str,
    display: String,
}

struct SecretRecord {
    confidence: &'static str,
    display: String,
    line_number: usize,
    rule: &'static str,
    message: String,
}

struct RiskyRecord {
    severity: &'static str,
    display: String,
    line_number: usize,
    rule: &'static str,
    explanation: &'static str,
}

/// Scans a caller-supplied list of files (relative to `root`) for sensitive filenames,
/// secret-like lines (keyword cascade + provider signatures + entropy), and risky code
/// patterns. `max_bytes_per_file`, when set, skips reading files above that size —
/// their filename is still checked for sensitivity.
///
/// `cancel` is checked once per file (`.ai/project/12-domain-rules.md` requires
/// checking cancellation *inside* the loop, not only between pipeline steps; per-file
/// is this loop's natural granularity, matching `codepack-scanner::walk_project`'s
/// per-entry checks from S2).
pub fn scan_project(
    root: &Path,
    relative_files: &[PathBuf],
    max_bytes_per_file: Option<u64>,
    cancel: &CancellationToken,
) -> Result<ScanResult> {
    let mut files: Vec<FileRecord> = Vec::new();
    let mut secrets: Vec<SecretRecord> = Vec::new();
    let mut risky: Vec<RiskyRecord> = Vec::new();

    for relative in relative_files {
        if cancel.is_cancelled() {
            return Err(SecurityError::Cancelled);
        }
        if let Some(severity) = sensitive_file_severity(relative) {
            files.push(FileRecord {
                severity,
                display: paths::rel_display(relative),
            });
        }

        if !classify::should_consider_text_file(relative) {
            continue;
        }
        let absolute = root.join(relative);
        let metadata = match fs::metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if let Some(max) = max_bytes_per_file
            && metadata.len() > max
        {
            continue;
        }
        let Some(text) = read_text_for_scan(&absolute)? else {
            continue;
        };
        let display = paths::rel_display(relative);

        for (idx, line) in text.lines().enumerate() {
            let line_number = idx + 1;
            let secret_hits = collect_secret_hits(line);
            if !secret_hits.is_empty() {
                // Computed once per line, shared by every hit on it (see
                // `mask_non_keyword_secret_spans`'s doc comment for why this must not
                // be scoped to only the current hit's own span).
                let message = redacted_message(line);
                for hit in secret_hits {
                    secrets.push(SecretRecord {
                        confidence: hit.confidence,
                        display: display.clone(),
                        line_number,
                        rule: hit.rule,
                        message: message.clone(),
                    });
                }
            }
            for hit in collect_risky_hits(line) {
                risky.push(RiskyRecord {
                    severity: hit.severity,
                    display: display.clone(),
                    line_number,
                    rule: hit.rule,
                    explanation: hit.explanation,
                });
            }
        }
    }

    files.sort_by(|a, b| {
        confidence_rank(a.severity)
            .cmp(&confidence_rank(b.severity))
            .then_with(|| a.display.to_lowercase().cmp(&b.display.to_lowercase()))
    });
    secrets.sort_by(|a, b| {
        confidence_rank(a.confidence)
            .cmp(&confidence_rank(b.confidence))
            .then_with(|| a.display.to_lowercase().cmp(&b.display.to_lowercase()))
            .then_with(|| a.line_number.cmp(&b.line_number))
    });
    risky.sort_by(|a, b| {
        confidence_rank(a.severity)
            .cmp(&confidence_rank(b.severity))
            .then_with(|| a.display.to_lowercase().cmp(&b.display.to_lowercase()))
            .then_with(|| a.line_number.cmp(&b.line_number))
    });

    let mut findings = Vec::with_capacity(files.len() + secrets.len() + risky.len());
    for file in &files {
        findings.push(Finding {
            kind: FindingKind::SensitiveFile,
            severity: file.severity.to_string(),
            confidence: "high".to_string(),
            file: file.display.clone(),
            line: None,
            rule: "sensitive_filename".to_string(),
            message: "Sensitive-looking filename or suffix.".to_string(),
        });
    }
    for secret in &secrets {
        findings.push(Finding {
            kind: FindingKind::PotentialSecret,
            severity: secret.confidence.to_string(),
            confidence: secret.confidence.to_string(),
            file: secret.display.clone(),
            line: Some(secret.line_number),
            rule: secret.rule.to_string(),
            message: secret.message.clone(),
        });
    }
    for hit in &risky {
        findings.push(Finding {
            kind: FindingKind::RiskyCode,
            severity: hit.severity.to_string(),
            confidence: risky_code::RISKY_CODE_FINDING_CONFIDENCE.to_string(),
            file: hit.display.clone(),
            line: Some(hit.line_number),
            rule: hit.rule.to_string(),
            message: hit.explanation.to_string(),
        });
    }

    let summary = ScanSummary {
        sensitive_files: files.len(),
        potential_secrets: secrets.len(),
        risky_code: risky.len(),
        total_findings: findings.len(),
    };

    Ok(ScanResult { summary, findings })
}

#[cfg(test)]
mod tests {
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
}
