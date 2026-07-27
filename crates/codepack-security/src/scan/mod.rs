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
use crate::patterns::{
    confidence_rank, credentials, entropy, keyword, prefilter, provider, risky_code,
};

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
/// **At most one hit per line**, because legacy's `_collect_security_findings` appends at
/// most one `SecretFinding` per line and a second finding on the identical file+line
/// breaks golden parity outright.
///
/// Which one survives is chosen by how much it tells the user, not by detector order:
///
/// 1. **A confident keyword hit wins**, meaning `critical` or `high`. These are the tiers
///    legacy itself treats as definitive (the `critical` tier is the PEM private-key
///    header), so reporting anything else there would be a parity divergence with no
///    gain — the line is already described as strongly as it can be.
/// 2. **Otherwise a provider signature wins.** `aws-access-key-id`/`critical` names the
///    provider and the real severity; a `medium`/`low` `secret_like_line` says only "this
///    line contains a secret-ish word". An earlier version of this function let *any*
///    keyword hit suppress provider hits, which silently demoted a confirmed AWS key to
///    `low` on any line containing the word `token` or `key` — i.e. on most real lines,
///    since people label their keys. The corpus test could not see that regression: it
///    measures detection as a boolean per line, so rule id and severity are invisible to
///    precision/recall/F1.
/// 3. **Otherwise the weak keyword hit wins over the structural credential rules and
///    entropy.** Entropy and the credential rules carry no provider identity, so they
///    add nothing to a line the keyword cascade already described — and legacy, which
///    has neither, reports exactly `secret_like_line` there.
/// 4. **Otherwise a structural credential match wins over entropy** (Finding 2,
///    2026-07-27 audit): a password's *position* inside a URL, or an HTTP
///    `Basic`/`Digest` auth token. Both are more specific and more precise than a
///    generic high-entropy guess, so they are checked first — but only reached once
///    every keyword-based rule above has passed, which is why a `Bearer` line never
///    reaches this step: it was already caught by rule 1's `has_secret_with_value`.
/// 5. **Entropy is the last resort**, which is where its whole recall contribution lives:
///    lines nothing above can see.
///
/// The redaction applied to the surviving hit's message is unaffected by this choice —
/// [`redacted_message`] masks every detected span regardless of which hit is reported.
fn collect_secret_hits(line: &str) -> Vec<SecretHit> {
    if keyword::is_self_protected(line) {
        return Vec::new();
    }

    let prefiltered = prefilter::has_hit(line);
    let keyword_hit = if prefiltered {
        keyword::secret_confidence(line)
    } else {
        None
    };

    // Rule 1: a keyword hit legacy would consider definitive.
    if let Some(confidence) = keyword_hit
        && matches!(confidence, "critical" | "high")
    {
        return vec![SecretHit {
            rule: "secret_like_line",
            confidence,
        }];
    }

    // Rule 2: a provider signature, which names what was found and how bad it is.
    if prefiltered && let Some(found) = provider::find_provider_matches(line).into_iter().next() {
        return vec![SecretHit {
            rule: found.rule_id,
            confidence: found.confidence,
        }];
    }
    // Never gated by the prefilter — see patterns::prefilter's documented scope limits.
    if let Some(found) = provider::find_telegram_matches(line).into_iter().next() {
        return vec![SecretHit {
            rule: found.rule_id,
            confidence: found.confidence,
        }];
    }

    // Rule 3: the weaker keyword hit, which is what legacy would have reported.
    if let Some(confidence) = keyword_hit {
        return vec![SecretHit {
            rule: "secret_like_line",
            confidence,
        }];
    }

    // Rule 4: structural credential detectors that need no keyword context at all.
    // Not gated by the prefilter: neither "://" nor "Basic"/"Digest" is in its literal
    // set (adding them would help little — every line reaching this point already
    // cleared every prefilter-gated check above without matching), and both matchers
    // are cheap single-pass scans, the same order of cost as the keyword cascade
    // itself.
    if !credentials::find_url_credentials(line).is_empty() {
        return vec![SecretHit {
            rule: "url-credentials",
            confidence: "high",
        }];
    }
    if !credentials::find_http_auth_tokens(line).is_empty() {
        return vec![SecretHit {
            rule: "http-auth-credentials",
            confidence: "high",
        }];
    }

    // Rule 5: entropy, on lines nothing else recognised.
    entropy::entropy_findings(line)
        .into_iter()
        .next()
        .map(|found| SecretHit {
            rule: "high-entropy-token",
            confidence: found.confidence,
        })
        .into_iter()
        .collect()
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
        .filter(|rule| rule.is_match(line))
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
mod tests;
