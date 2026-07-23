//! `.sarif` writer, ported from legacy `_write_security_sarif`. Minimal-but-valid
//! SARIF 2.1.0: one run, one driver, one rule entry per distinct `rule_id` (first
//! occurrence's message wins, matching legacy's `rules.setdefault`), one result per
//! finding. `level` is `"error"` for `critical`/`high` severity, `"warning"`
//! otherwise — legacy never emits `"note"`, and neither does this.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::{Value, json};

use crate::error::{Result, SecurityError};
use crate::scan::ScanResult;
use crate::scan::paths::sarif_uri;

const SARIF_VERSION: &str = "2.1.0";
const SARIF_SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const DRIVER_NAME: &str = "codepack Security Scanner";

#[derive(Debug, Serialize)]
struct SarifRule {
    id: String,
    name: String,
    #[serde(rename = "shortDescription")]
    short_description: SarifText,
}

#[derive(Debug, Serialize)]
struct SarifText {
    text: String,
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        text.chars().take(max_chars).collect()
    }
}

pub(crate) fn render_sarif(result: &ScanResult) -> Result<String> {
    let mut rules: BTreeMap<String, SarifRule> = BTreeMap::new();
    let mut results: Vec<Value> = Vec::with_capacity(result.findings.len());

    for finding in &result.findings {
        rules
            .entry(finding.rule.clone())
            .or_insert_with(|| SarifRule {
                id: finding.rule.clone(),
                name: finding.rule.clone(),
                short_description: SarifText {
                    text: truncate_chars(&finding.message, 120),
                },
            });

        let uri = sarif_uri(&finding.file);
        let mut physical_location = json!({
            "artifactLocation": { "uri": uri },
        });
        if let Some(line) = finding.line {
            physical_location["region"] = json!({ "startLine": line });
        }

        let level = if matches!(finding.severity.as_str(), "critical" | "high") {
            "error"
        } else {
            "warning"
        };

        results.push(json!({
            "ruleId": finding.rule,
            "level": level,
            "message": { "text": finding.message },
            "locations": [ { "physicalLocation": physical_location } ],
        }));
    }

    // Preserve first-seen rule order (matching legacy's dict-insertion order), not the
    // BTreeMap's alphabetical order used only for accumulation above.
    let mut ordered_rules: Vec<&SarifRule> = Vec::with_capacity(rules.len());
    let mut seen = std::collections::HashSet::new();
    for finding in &result.findings {
        if seen.insert(finding.rule.clone()) {
            ordered_rules.push(rules.get(&finding.rule).expect("just inserted above"));
        }
    }

    let sarif = json!({
        "version": SARIF_VERSION,
        "$schema": SARIF_SCHEMA,
        "runs": [ {
            "tool": {
                "driver": {
                    "name": DRIVER_NAME,
                    "rules": ordered_rules,
                }
            },
            "results": results,
        } ],
    });

    serde_json::to_string_pretty(&sarif).map_err(|source| SecurityError::InvalidJson {
        field: "security_scan_sarif",
        source,
    })
}

pub fn write_sarif_report(result: &ScanResult, output_file: &Path) -> Result<()> {
    let rendered = render_sarif(result)?;
    fs::write(output_file, rendered).map_err(|source| SecurityError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{Finding, FindingKind, ScanSummary};

    fn sample_result() -> ScanResult {
        ScanResult {
            summary: ScanSummary {
                sensitive_files: 0,
                potential_secrets: 1,
                risky_code: 1,
                total_findings: 2,
            },
            findings: vec![
                Finding {
                    kind: FindingKind::PotentialSecret,
                    severity: "critical".to_string(),
                    confidence: "critical".to_string(),
                    file: ".\\src\\config.py".to_string(),
                    line: Some(3),
                    rule: "aws-access-key-id".to_string(),
                    message: "API_KEY=<REDACTED>".to_string(),
                },
                Finding {
                    kind: FindingKind::RiskyCode,
                    severity: "medium".to_string(),
                    confidence: "medium".to_string(),
                    file: ".\\src\\app.js".to_string(),
                    line: Some(10),
                    rule: "inner-html".to_string(),
                    message: "innerHTML assignment can create XSS risk with untrusted input."
                        .to_string(),
                },
            ],
        }
    }

    #[test]
    fn top_level_shape_is_sarif_2_1_0() {
        let rendered = render_sarif(&sample_result()).unwrap();
        let value: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["version"], "2.1.0");
        assert_eq!(value["$schema"], SARIF_SCHEMA);
        assert!(value["runs"].as_array().unwrap().len() == 1);
        assert!(value["runs"][0]["tool"]["driver"]["name"].is_string());
        assert!(value["runs"][0]["tool"]["driver"]["rules"].is_array());
        assert!(value["runs"][0]["results"].is_array());
    }

    #[test]
    fn critical_severity_maps_to_error_level() {
        let rendered = render_sarif(&sample_result()).unwrap();
        let value: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["runs"][0]["results"][0]["level"], "error");
    }

    #[test]
    fn medium_severity_maps_to_warning_never_note() {
        let rendered = render_sarif(&sample_result()).unwrap();
        let value: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["runs"][0]["results"][1]["level"], "warning");
    }

    #[test]
    fn region_present_when_line_known() {
        let rendered = render_sarif(&sample_result()).unwrap();
        let value: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(
            value["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["startLine"],
            3
        );
    }

    #[test]
    fn region_omitted_entirely_when_no_line_number() {
        let mut result = sample_result();
        result.findings[0].line = None;
        let rendered = render_sarif(&result).unwrap();
        let value: Value = serde_json::from_str(&rendered).unwrap();
        let location = &value["runs"][0]["results"][0]["locations"][0]["physicalLocation"];
        assert!(location.get("region").is_none());
    }

    #[test]
    fn uri_is_forward_slashed_and_not_dot_prefixed() {
        let rendered = render_sarif(&sample_result()).unwrap();
        let value: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(
            value["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
                ["uri"],
            "src/config.py"
        );
    }
}
