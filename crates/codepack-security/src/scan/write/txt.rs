//! Human-readable `.txt` writer, ported from legacy `write_security_scan_report`'s
//! text section (structure and truncation caps — 300 files / 500 secret lines / 500
//! risky-code lines — kept verbatim; the extra raw-code-snippet line legacy prints
//! below each risky-code entry is intentionally not reproduced here, since this
//! crate's `Finding.message` for `risky_code` is the fixed rule explanation, not a
//! second, separately redacted copy of the line — see `scan::mod` for why keeping a
//! single `message` field per finding, matching the `.json`/`.sarif` contract exactly,
//! was chosen over adding a second field).

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::error::{Result, SecurityError};
use crate::scan::{Finding, FindingKind, ScanResult};

use codepack_core::time::now_human_utc;

const MAX_SENSITIVE_FILES_SHOWN: usize = 300;
const MAX_SECRET_LINES_SHOWN: usize = 500;
const MAX_RISKY_LINES_SHOWN: usize = 500;

fn write_section<'a>(
    out: &mut String,
    title: &str,
    items: impl Iterator<Item = &'a Finding>,
    max_shown: usize,
    render: impl Fn(&mut String, &Finding),
) {
    let _ = writeln!(out, "\n--- {title} ---");
    let items: Vec<&Finding> = items.collect();
    if items.is_empty() {
        out.push_str("None detected.\n");
        return;
    }
    for finding in items.iter().take(max_shown) {
        render(out, finding);
    }
    if items.len() > max_shown {
        let _ = writeln!(out, "... and {} more", items.len() - max_shown);
    }
}

pub(crate) fn render_txt(result: &ScanResult) -> String {
    let mut out = String::new();
    out.push_str("=== Enhanced Security Scan v3 ===\n");
    let _ = writeln!(out, "Generated: {}", now_human_utc());
    out.push_str(
        "This is a heuristic scanner, not a professional secret scanner or SAST engine. Values are redacted.\n",
    );
    out.push_str(
        "Machine-readable outputs are written next to this report: matching .json and .sarif files.\n",
    );
    out.push_str("Findings are grouped by confidence/severity to reduce noise.\n");
    out.push_str(&"=".repeat(100));
    out.push('\n');

    write_section(
        &mut out,
        "Sensitive-looking files",
        result
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::SensitiveFile),
        MAX_SENSITIVE_FILES_SHOWN,
        |out, finding| {
            let _ = writeln!(out, "[{}] {}", finding.severity, finding.file);
        },
    );

    write_section(
        &mut out,
        "Potential secret-like lines",
        result
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::PotentialSecret),
        MAX_SECRET_LINES_SHOWN,
        |out, finding| {
            let _ = writeln!(
                out,
                "[{}] {}:{}: {}",
                finding.confidence,
                finding.file,
                finding.line.unwrap_or(0),
                finding.message
            );
        },
    );

    write_section(
        &mut out,
        "Risky code patterns",
        result
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::RiskyCode),
        MAX_RISKY_LINES_SHOWN,
        |out, finding| {
            let _ = writeln!(
                out,
                "[{}] {}:{}: [{}] {}",
                finding.severity,
                finding.file,
                finding.line.unwrap_or(0),
                finding.rule,
                finding.message
            );
        },
    );

    out.push_str("\n--- Recommended actions before sharing ---\n");
    out.push_str("- Keep Safe Export mode enabled for external AI/code-review handoffs.\n");
    out.push_str("- Review all critical/high findings before sharing the archive.\n");
    out.push_str("- Rotate any secret that may have been committed or exported by mistake.\n");
    out.push_str("- Prefer `.env.example` with placeholder values for shared exports.\n");
    out.push_str(
        "- Manually review risky code findings before treating them as vulnerabilities.\n",
    );

    out
}

pub fn write_txt_report(result: &ScanResult, output_file: &Path) -> Result<()> {
    let rendered = render_txt(result);
    fs::write(output_file, rendered).map_err(|source| SecurityError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::ScanSummary;

    #[test]
    fn empty_result_reports_none_detected_in_every_section() {
        let result = ScanResult {
            summary: ScanSummary::default(),
            findings: Vec::new(),
        };
        let rendered = render_txt(&result);
        assert_eq!(rendered.matches("None detected.").count(), 3);
    }

    #[test]
    fn secret_finding_line_includes_redacted_message_only() {
        let result = ScanResult {
            summary: ScanSummary {
                sensitive_files: 0,
                potential_secrets: 1,
                risky_code: 0,
                total_findings: 1,
            },
            findings: vec![Finding {
                kind: FindingKind::PotentialSecret,
                severity: "high".to_string(),
                confidence: "high".to_string(),
                file: ".\\config.py".to_string(),
                line: Some(5),
                rule: "secret_like_line".to_string(),
                message: "API_KEY=<REDACTED>".to_string(),
            }],
        };
        let rendered = render_txt(&result);
        assert!(rendered.contains("[high] .\\config.py:5: API_KEY=<REDACTED>"));
    }

    #[test]
    fn truncation_cap_reports_remainder_count() {
        let findings: Vec<Finding> = (0..305)
            .map(|i| Finding {
                kind: FindingKind::SensitiveFile,
                severity: "high".to_string(),
                confidence: "high".to_string(),
                file: format!(".\\file{i}.env"),
                line: None,
                rule: "sensitive_filename".to_string(),
                message: "Sensitive-looking filename or suffix.".to_string(),
            })
            .collect();
        let result = ScanResult {
            summary: ScanSummary {
                sensitive_files: 305,
                potential_secrets: 0,
                risky_code: 0,
                total_findings: 305,
            },
            findings,
        };
        let rendered = render_txt(&result);
        assert!(rendered.contains("... and 5 more"));
    }
}
