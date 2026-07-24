//! `06_security_scan.{txt,json,sarif}`: a thin adapter over `codepack-security`'s own
//! writers (`codepack_security::scan::write::*`), ported from legacy
//! `reports/insights/security.py::write_security_scan_report`. This job never re-scans
//! or re-redacts anything itself — [`ReportContext::scan`] is the already-computed
//! `codepack_security::ScanResult` (S3); every byte written here comes straight out of
//! that crate's own, already-tested writers (this crate's scope boundary, `lib.rs`).
//!
//! `ctx.scan` is `None` until a future engine stage (S9) actually calls
//! `codepack_security::scan_project` and threads the result through
//! [`ReportContext`] — no report job in this crate performs the scan itself. When it
//! is `None`, this job writes only a `.txt` placeholder explaining the scan did not
//! run; it deliberately does **not** write a `.json`/`.sarif` sibling with an
//! empty/default `ScanResult`, because a machine-readable "zero findings" result is
//! indistinguishable from "the scan ran and found nothing" — actively misleading for a
//! security artifact, and worse than a missing file for a downstream tool or reviewer
//! deciding whether it is safe to trust the absence of findings.

use std::path::{Path, PathBuf};

use codepack_security::scan::write::{write_json_report, write_sarif_report, write_txt_report};

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::plugin::ReportJob;
use crate::profile;

pub const JOB: ReportJob = ReportJob {
    filename: "06_security_scan.txt",
    profiles: profile::SECURITY_SCAN_TXT,
    description: "Secret and risky-code scanner output (.txt human-readable, plus .json/.sarif siblings).",
    run: write_security_scan_reports,
};

fn sibling(output_file: &Path, extension: &str) -> PathBuf {
    output_file.with_extension(extension)
}

fn write_security_scan_reports(
    ctx: &ReportContext<'_>,
    output_file: &Path,
) -> Result<(), ReportError> {
    let Some(scan) = ctx.scan else {
        return write_placeholder(output_file);
    };

    write_txt_report(scan, output_file).map_err(|source| ReportError::Security { source })?;
    write_json_report(scan, &sibling(output_file, "json"))
        .map_err(|source| ReportError::Security { source })?;
    write_sarif_report(scan, &sibling(output_file, "sarif"))
        .map_err(|source| ReportError::Security { source })?;
    Ok(())
}

fn write_placeholder(output_file: &Path) -> Result<(), ReportError> {
    let body = "=== Enhanced Security Scan v3 ===\n\
Security scanning was not run for this export (no scan result was supplied to the\n\
report stage). Re-run the export with the security scan enabled to populate this\n\
report, including its .json and .sarif siblings.\n";
    std::fs::write(output_file, body).map_err(|source| ReportError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fixture;
    use codepack_security::scan::{Finding, FindingKind, ScanResult, ScanSummary};

    fn fixture_scan_result() -> ScanResult {
        ScanResult {
            summary: ScanSummary {
                sensitive_files: 1,
                potential_secrets: 1,
                risky_code: 0,
                total_findings: 2,
            },
            findings: vec![
                Finding {
                    kind: FindingKind::SensitiveFile,
                    severity: "critical".to_string(),
                    confidence: "high".to_string(),
                    file: ".\\.env".to_string(),
                    line: None,
                    rule: "sensitive_filename".to_string(),
                    message: "Sensitive-looking filename or suffix.".to_string(),
                },
                Finding {
                    kind: FindingKind::PotentialSecret,
                    severity: "high".to_string(),
                    confidence: "high".to_string(),
                    file: ".\\config.py".to_string(),
                    line: Some(3),
                    rule: "secret_like_line".to_string(),
                    message: "API_KEY=<REDACTED>".to_string(),
                },
            ],
        }
    }

    #[test]
    fn writes_txt_json_and_sarif_matching_codepack_security_own_writers() {
        let fixture = Fixture::new(|_root| {});
        let ctx = fixture.context("full");
        let scan = fixture_scan_result();
        let ctx = ReportContext {
            scan: Some(&scan),
            ..ctx
        };
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_security_scan_reports(&ctx, &output_file).unwrap();

        let txt_path = out_dir.path().join("06_security_scan.txt");
        let json_path = out_dir.path().join("06_security_scan.json");
        let sarif_path = out_dir.path().join("06_security_scan.sarif");
        assert!(txt_path.exists());
        assert!(json_path.exists());
        assert!(sarif_path.exists());

        let expected_txt_path = out_dir.path().join("expected.txt");
        write_txt_report(&scan, &expected_txt_path).unwrap();
        assert_eq!(
            std::fs::read_to_string(&txt_path).unwrap(),
            std::fs::read_to_string(&expected_txt_path).unwrap()
        );

        let expected_json_path = out_dir.path().join("expected.json");
        write_json_report(&scan, &expected_json_path).unwrap();
        assert_eq!(
            std::fs::read_to_string(&json_path).unwrap(),
            std::fs::read_to_string(&expected_json_path).unwrap()
        );

        let expected_sarif_path = out_dir.path().join("expected.sarif");
        write_sarif_report(&scan, &expected_sarif_path).unwrap();
        assert_eq!(
            std::fs::read_to_string(&sarif_path).unwrap(),
            std::fs::read_to_string(&expected_sarif_path).unwrap()
        );
    }

    #[test]
    fn writes_only_a_placeholder_txt_when_scan_is_none() {
        let fixture = Fixture::new(|_root| {});
        let ctx = fixture.context("full");
        assert!(ctx.scan.is_none());
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_security_scan_reports(&ctx, &output_file).unwrap();

        assert!(output_file.exists());
        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("was not run"));
        assert!(!out_dir.path().join("06_security_scan.json").exists());
        assert!(!out_dir.path().join("06_security_scan.sarif").exists());
    }
}
