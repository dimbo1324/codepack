//! Invariant I5: `ExportPlan`/`PlannedFile`/the `rules` object's field order is a
//! documented artifact contract. This asserts the *literal* serialized JSON key
//! sequence, not just semantic equality — a field rename or reorder must fail this
//! test, not silently pass a value-based comparison.
//!
//! Deliberately does not route through `serde_json::Value`: without the
//! `preserve_order` cargo feature (not enabled for this workspace), `Value::Object` is
//! a `BTreeMap` and alphabetizes keys on the way in, which would silently defeat this
//! exact test. Instead this scans the raw serialized text for each key's byte offset
//! and asserts they appear in strictly increasing order — a true test of what
//! `serde_json::to_string`/`to_string_pretty` (what `write_export_plan_files` actually
//! calls) produced.

use std::fs;

use codepack_core::CancellationToken;
use codepack_scanner::{ExportIgnoreRules, ScanOptions, build_export_plan};

/// Asserts that `"key":` for every key in `keys_in_order` appears in `json`, and that
/// their positions are strictly increasing.
fn assert_key_order(json: &str, keys_in_order: &[&str]) {
    let mut last_position = 0usize;
    for key in keys_in_order {
        let needle = format!("\"{key}\":");
        // Searching only the remainder after the previous match is what enforces
        // order here: a key that appears earlier than expected (or not at all after
        // its predecessor) makes this `find` return `None`.
        let position = json[last_position..].find(&needle).unwrap_or_else(|| {
            panic!("key {key:?} not found after byte {last_position} in {json}")
        }) + last_position;
        last_position = position + needle.len();
    }
}

#[test]
fn export_plan_top_level_key_order_matches_the_documented_contract() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "x").unwrap();
    let options = ScanOptions::default();
    let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
    let plan = build_export_plan(dir.path(), &options, &rules, &CancellationToken::new()).unwrap();

    let json = serde_json::to_string(&plan).unwrap();
    assert_key_order(
        &json,
        &[
            "generated_at",
            "project_name",
            "source_root",
            "profile",
            "safe_export_mode",
            "diff_export_mode",
            "incremental_enabled",
            "included_files",
            "excluded_files",
            "skipped_dirs",
            "warnings",
            "rules",
            "summary",
        ],
    );
}

#[test]
fn planned_file_key_order_matches_the_documented_contract() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "x").unwrap();
    let options = ScanOptions::default();
    let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
    let plan = build_export_plan(dir.path(), &options, &rules, &CancellationToken::new()).unwrap();

    let json = serde_json::to_string(&plan.included_files[0]).unwrap();
    assert_key_order(
        &json,
        &[
            "relative_path",
            "size",
            "status",
            "reason",
            "severity",
            "group",
        ],
    );
}

#[test]
fn rules_report_key_order_matches_the_documented_contract() {
    let dir = tempfile::tempdir().unwrap();
    let options = ScanOptions::default();
    let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);

    let json = serde_json::to_string(&rules.to_report()).unwrap();
    assert_key_order(
        &json,
        &[
            "source_file",
            "loaded_rules",
            "excluded_dirs",
            "excluded_files",
            "excluded_extensions",
            "always_include_files",
            "always_include_dirs",
        ],
    );
}

#[test]
fn summary_key_order_matches_the_documented_contract() {
    let dir = tempfile::tempdir().unwrap();
    let options = ScanOptions::default();
    let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
    let plan = build_export_plan(dir.path(), &options, &rules, &CancellationToken::new()).unwrap();

    let json = serde_json::to_string(&plan.summary).unwrap();
    assert_key_order(
        &json,
        &[
            "included_count",
            "excluded_count",
            "estimated_included_bytes",
        ],
    );
}

#[test]
fn a_reordered_or_renamed_field_would_fail_this_test() {
    // Self-check that `assert_key_order` actually detects drift, per the "write a
    // test that would fail if a field were renamed or reordered" requirement.
    let json = r#"{"a":1,"b":2}"#;
    let result = std::panic::catch_unwind(|| assert_key_order(json, &["b", "a"]));
    assert!(result.is_err());
}
