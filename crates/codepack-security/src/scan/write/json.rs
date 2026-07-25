//! `.json` writer, ported from legacy `_write_security_json`. Field order and names
//! are a contract (`.ai/project/12-domain-rules.md`) — `schema_version`,
//! `generated_at`, `summary`, `findings`, each finding `type`/`severity`/`confidence`/
//! `file`/`line`/`rule`/`message`.

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::{Result, SecurityError};
use crate::scan::{Finding, ScanResult, ScanSummary};

use codepack_core::time::now_human_utc;

pub(crate) const SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Serialize)]
struct ScanReportJson<'a> {
    schema_version: &'static str,
    generated_at: String,
    summary: &'a ScanSummary,
    findings: &'a [Finding],
}

pub(crate) fn render_json(result: &ScanResult) -> Result<String> {
    let payload = ScanReportJson {
        schema_version: SCHEMA_VERSION,
        generated_at: now_human_utc(),
        summary: &result.summary,
        findings: &result.findings,
    };
    serde_json::to_string_pretty(&payload).map_err(|source| SecurityError::InvalidJson {
        field: "security_scan_report",
        source,
    })
}

pub fn write_json_report(result: &ScanResult, output_file: &Path) -> Result<()> {
    let rendered = render_json(result)?;
    fs::write(output_file, rendered).map_err(|source| SecurityError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::FindingKind;

    fn sample_result() -> ScanResult {
        ScanResult {
            summary: ScanSummary {
                sensitive_files: 1,
                potential_secrets: 0,
                risky_code: 0,
                total_findings: 1,
            },
            findings: vec![Finding {
                kind: FindingKind::SensitiveFile,
                severity: "critical".to_string(),
                confidence: "high".to_string(),
                file: ".\\\\.env".to_string(),
                line: None,
                rule: "sensitive_filename".to_string(),
                message: "Sensitive-looking filename or suffix.".to_string(),
            }],
        }
    }

    #[test]
    fn renders_expected_top_level_keys() {
        let rendered = render_json(&sample_result()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["schema_version"], "1.0");
        assert!(value.get("generated_at").is_some());
        assert!(value["summary"].is_object());
        assert!(value["findings"].is_array());
    }

    #[test]
    fn finding_has_exact_legacy_field_set() {
        let rendered = render_json(&sample_result()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        let finding = &value["findings"][0];
        let mut keys: Vec<&str> = finding
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "confidence",
                "file",
                "line",
                "message",
                "rule",
                "severity",
                "type"
            ]
        );
    }
}
