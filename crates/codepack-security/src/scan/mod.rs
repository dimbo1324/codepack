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
fn collect_secret_hits(line: &str) -> Vec<SecretHit> {
    if keyword::is_self_protected(line) {
        return Vec::new();
    }

    let mut hits = Vec::new();
    if prefilter::has_hit(line) {
        if let Some(confidence) = keyword::secret_confidence(line) {
            hits.push(SecretHit {
                rule: "secret_like_line",
                confidence,
            });
        }
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
/// untouched. This masks every provider/telegram/entropy match span on the line with a
/// fixed placeholder *before* handing the line to [`keyword::redacted_line`], so every
/// [`SecretRecord::message`] on a given line — regardless of which specific rule
/// produced it — is built from a line with **all** known secret-shaped spans already
/// removed, not just the span of the one rule that produced this particular record.
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
                let masked = mask_non_keyword_secret_spans(line);
                let message = keyword::redacted_line(&masked);
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
