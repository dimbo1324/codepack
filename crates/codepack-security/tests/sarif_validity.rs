//! Structural validity of the `.sarif` artifact against the SARIF 2.1.0 core schema
//! (https://json.schemastore.org/sarif-2.1.0.json): this does not run a full JSON
//! Schema validator (no such dependency exists in this crate, by design — no network,
//! minimal deps), but asserts every field a SARIF 2.1.0 consumer requires: top-level
//! `version`/`$schema`/`runs`, each run's `tool.driver.name`, and every result's
//! `ruleId`/`level`/`message.text`/`locations[].physicalLocation.artifactLocation.uri`.

use std::fs;
use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use codepack_security::scan::write::write_sarif_report;
use codepack_security::scan_project;

fn write_file(root: &Path, relative: &str, content: &str) -> PathBuf {
    let full = root.join(relative);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&full, content).unwrap();
    PathBuf::from(relative)
}

fn sample_sarif_value() -> serde_json::Value {
    let dir = tempfile::tempdir().unwrap();
    let env_file = write_file(dir.path(), ".env", "SECRET=totally-fake-value-123\n");
    let app_py = write_file(dir.path(), "app.py", "eval(user_input)\n");

    let result = scan_project(
        dir.path(),
        &[env_file, app_py],
        None,
        &CancellationToken::new(),
    )
    .unwrap();
    assert!(
        !result.findings.is_empty(),
        "fixture must produce at least one finding"
    );

    let sarif_path = dir.path().join("06_security_scan.sarif");
    write_sarif_report(&result, &sarif_path).unwrap();

    let raw = fs::read_to_string(&sarif_path).unwrap();
    serde_json::from_str(&raw).expect("sarif output must be valid JSON")
}

#[test]
fn top_level_shape_has_version_schema_and_runs() {
    let value = sample_sarif_value();
    assert_eq!(value["version"], "2.1.0");
    assert!(value["$schema"].as_str().unwrap().contains("sarif-2.1.0"));
    let runs = value["runs"].as_array().expect("runs must be an array");
    assert!(!runs.is_empty(), "runs must contain at least one run");
}

#[test]
fn driver_has_a_non_empty_tool_name() {
    let value = sample_sarif_value();
    let name = value["runs"][0]["tool"]["driver"]["name"]
        .as_str()
        .expect("tool.driver.name must be a string");
    assert!(!name.is_empty());
}

#[test]
fn every_rule_has_id_name_and_short_description() {
    let value = sample_sarif_value();
    let rules = value["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("tool.driver.rules must be an array");
    assert!(!rules.is_empty());
    for rule in rules {
        assert!(rule["id"].is_string());
        assert!(rule["name"].is_string());
        assert!(rule["shortDescription"]["text"].is_string());
    }
}

#[test]
fn every_result_has_the_core_required_fields() {
    let value = sample_sarif_value();
    let results = value["runs"][0]["results"]
        .as_array()
        .expect("results must be an array");
    assert!(!results.is_empty());

    for result in results {
        assert!(
            result["ruleId"].is_string(),
            "ruleId must be present and a string"
        );

        let level = result["level"].as_str().expect("level must be a string");
        assert!(
            matches!(level, "error" | "warning" | "note" | "none"),
            "level {level:?} is not a valid SARIF level"
        );
        // Ported behavior (deliberate, not a bug): the legacy scanner never emits
        // "note" or "none" — only "error" (critical/high) or "warning" (else).
        assert!(matches!(level, "error" | "warning"));

        assert!(
            result["message"]["text"].is_string(),
            "message.text must be present and a string"
        );

        let locations = result["locations"]
            .as_array()
            .expect("locations must be an array");
        assert!(
            !locations.is_empty(),
            "every result must have at least one location"
        );
        for location in locations {
            let uri = location["physicalLocation"]["artifactLocation"]["uri"]
                .as_str()
                .expect("artifactLocation.uri must be a string");
            assert!(!uri.is_empty());
            assert!(
                !uri.starts_with('.'),
                "uri must not carry the display dot-prefix"
            );
            assert!(
                !uri.contains('\\'),
                "uri must use forward slashes per SARIF convention"
            );
        }
    }
}
