//! Golden parity against the legacy Python implementation's own *executed* output.
//!
//! The S2 and S9 acceptance criteria (`ROADMAP.md`) require the export to match the
//! legacy version's output on fixtures. Until 2026-07-25 that was verified by reading
//! the archived Python sources — a check of intent, not of result. This suite closes
//! that gap: it compares our pipeline's artifacts against reference files produced by
//! actually running legacy (`cargo xtask golden`, see
//! `tests/golden/generate_reference.py` for the comparison specification and the
//! reasoning behind every excluded field).
//!
//! CI never runs Python: the references are committed, and this test only reads them.
//!
//! Paths inside the compared artifacts are backslash-separated on every platform in
//! both implementations (legacy's own `str(path).replace("/", "\\")`, mirrored by
//! `codepack_diff::normalize` and `codepack_scanner`'s `display_backslash`), so a
//! reference generated on Windows is equally valid on Linux and macOS CI.

mod support;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_storage::open;
use serde_json::Value;
use support::{read_zip_entry, zip_entry_names};

/// Mirrors `generate_reference.py`'s `VOLATILE_KEYS`. Two implementations of one
/// written contract: if they ever drift apart, these tests fail, which is the signal.
const VOLATILE_KEYS: &[&str] = &["generated_at", "source_root", "copied_root", "bundle_name"];

const MANIFEST_COMPARED_KEYS: &[&str] = &[
    "project_name",
    "cancelled",
    "settings",
    "diff_selection",
    "ignored_dirs",
];

const ARCHIVE_PLAN_COMPARED_KEYS: &[&str] = &["split_planned", "limit_bytes", "target_bytes"];

const PROJECT_DIR_PLACEHOLDER: &str = "<PROJECT>";

fn golden_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the engine crate always lives two levels below the repository root")
        .join("tests/golden")
}

fn strip_volatile(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(key, _)| !VOLATILE_KEYS.contains(&key.as_str()))
                .map(|(key, item)| (key.clone(), strip_volatile(item)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(strip_volatile).collect()),
        other => other.clone(),
    }
}

fn subset(value: &Value, keys: &[&str]) -> Value {
    let map = value.as_object().expect("artifact root must be an object");
    Value::Object(
        keys.iter()
            .filter_map(|key| map.get(*key).map(|item| ((*key).to_string(), item.clone())))
            .collect(),
    )
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not valid JSON: {error}", path.display()))
}

fn read_zip_json(zip_path: &Path, entry: &str) -> Value {
    let text = read_zip_entry(zip_path, entry);
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{entry} in the produced ZIP is not valid JSON: {error}"))
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Walks both trees together and records every differing path, so one run reports the
/// whole picture instead of stopping at the first mismatch. `reference` is legacy's
/// output, `ours` is this pipeline's.
fn collect_differences(path: &str, ours: &Value, reference: &Value, out: &mut Vec<String>) {
    match (ours, reference) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match (a.get(key), b.get(key)) {
                    (Some(x), Some(y)) => collect_differences(&child, x, y, out),
                    (Some(x), None) => out.push(format!("{child}: only ours has it ({x})")),
                    (None, Some(y)) => {
                        out.push(format!("{child}: MISSING from ours (legacy: {y})"))
                    }
                    (None, None) => unreachable!("key came from one of the two maps"),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (index, (x, y)) in a.iter().zip(b).enumerate() {
                collect_differences(&format!("{path}[{index}]"), x, y, out);
            }
        }
        (x, y) if x != y => out.push(format!("{path}: ours={x} legacy={y}")),
        _ => {}
    }
}

/// Compares one artifact against its reference, returning a readable report when they
/// differ. Returns `None` on a match.
fn compare(artifact: &str, ours: &Value, reference: &Value) -> Option<String> {
    if ours == reference {
        return None;
    }
    let mut differences = Vec::new();
    collect_differences("", ours, reference, &mut differences);
    Some(format!(
        "  {artifact}:\n{}",
        differences
            .iter()
            .map(|line| format!("    - {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

/// `REPORT_PLUGINS.json` is compared as a superset with identical gating rather than
/// field-for-field. See `tests/golden/generate_reference.py`'s specification for the
/// two intended differences (filled `description`s, and the extra `AI_CONTEXT`/
/// `AI_PROMPTS`/`REPORT_DASHBOARD.html` entries legacy builds outside its job list).
/// A missing legacy entry or a changed `profiles` set still fails.
fn compare_plugin_catalog(ours: &Value, reference: &Value) -> Option<String> {
    let ours = ours.as_array().expect("catalog must be an array");
    let reference = reference.as_array().expect("catalog must be an array");
    let mut differences = Vec::new();

    let ours_order: Vec<&str> = ours
        .iter()
        .filter_map(|entry| entry.get("filename").and_then(Value::as_str))
        .collect();

    for entry in reference {
        let filename = entry
            .get("filename")
            .and_then(Value::as_str)
            .expect("every catalog entry has a filename");
        let Some(mine) = ours
            .iter()
            .find(|candidate| candidate.get("filename").and_then(Value::as_str) == Some(filename))
        else {
            differences.push(format!("{filename}: MISSING from our catalog"));
            continue;
        };
        if mine.get("profiles") != entry.get("profiles") {
            differences.push(format!(
                "{filename}: profiles ours={} legacy={}",
                mine.get("profiles").unwrap_or(&Value::Null),
                entry.get("profiles").unwrap_or(&Value::Null)
            ));
        }
    }

    // Legacy's own entries must still appear in legacy's relative order.
    let expected_order: Vec<&str> = reference
        .iter()
        .filter_map(|entry| entry.get("filename").and_then(Value::as_str))
        .collect();
    let ours_filtered: Vec<&str> = ours_order
        .iter()
        .copied()
        .filter(|name| expected_order.contains(name))
        .collect();
    if ours_filtered != expected_order {
        differences.push(format!(
            "catalog order differs
      ours(legacy entries only)={ours_filtered:?}
      legacy={expected_order:?}"
        ));
    }

    if differences.is_empty() {
        return None;
    }
    Some(format!(
        "  REPORT_PLUGINS.json:
{}",
        differences
            .iter()
            .map(|line| format!("    - {line}"))
            .collect::<Vec<_>>()
            .join(
                "
"
            )
    ))
}

fn run_one_fixture(fixture_name: &str) {
    let golden = golden_root();
    let fixture_src = golden.join("fixtures").join(fixture_name);
    let reference_dir = golden.join("reference").join(fixture_name);
    assert!(
        reference_dir.is_dir(),
        "no golden reference for {fixture_name}; run `cargo xtask golden`"
    );

    // The fixture is copied outside the repository on purpose: our own git report walks
    // upward with `Repository::discover`, so running in place would make the codepack
    // repository itself part of the result and destroy determinism.
    let workspace = tempfile::tempdir().unwrap();
    let source = workspace.path().join(fixture_name);
    copy_tree(&fixture_src, &source);

    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut conn = open(&db_dir.path().join("codepack.db")).unwrap();
    // Deliberately the untouched default: `manifest.json` echoes the whole settings
    // block, so any override here would show up as a false parity mismatch. Artifacts
    // are therefore read back out of the produced ZIP rather than from staging.
    let config = Config::default();
    let (tx, _rx) = codepack_core::progress_channel();

    let outcome = codepack_engine::run_export(
        &mut conn,
        &source,
        output.path(),
        &config,
        &HashMap::new(),
        &tx,
        &CancellationToken::new(),
    )
    .unwrap_or_else(|error| panic!("{fixture_name}: export failed: {error}"));

    assert!(
        outcome.successful,
        "{fixture_name}: export was not successful: {:?}",
        outcome.copy_stats
    );

    let zip = &outcome.paths.final_zip;
    let mut reports = Vec::new();

    for (artifact, entry) in [
        (
            "28_export_plan.json",
            "reports/insights/28_export_plan.json",
        ),
        (
            "06_security_scan.json",
            "reports/insights/06_security_scan.json",
        ),
        ("PROJECT_PROFILE.json", "PROJECT_PROFILE.json"),
    ] {
        let ours = strip_volatile(&read_zip_json(zip, entry));
        let reference = read_json(&reference_dir.join(artifact));
        reports.extend(compare(artifact, &ours, &reference));
    }

    reports.extend(compare_plugin_catalog(
        &read_zip_json(zip, "reports/insights/REPORT_PLUGINS.json"),
        &read_json(&reference_dir.join("REPORT_PLUGINS.json")),
    ));

    let ours = subset(
        &strip_volatile(&read_zip_json(zip, "manifest.json")),
        MANIFEST_COMPARED_KEYS,
    );
    let reference = read_json(&reference_dir.join("manifest.subset.json"));
    reports.extend(compare("manifest.subset.json", &ours, &reference));

    let ours = subset(
        &read_zip_json(zip, "reports/insights/27_archive_plan.json"),
        ARCHIVE_PLAN_COMPARED_KEYS,
    );
    let reference = read_json(&reference_dir.join("27_archive_plan.subset.json"));
    reports.extend(compare("27_archive_plan.subset.json", &ours, &reference));

    let mut names: Vec<String> = zip_entry_names(zip)
        .into_iter()
        .map(|name| {
            let normalized = name.replace('\\', "/");
            match normalized.strip_prefix(&format!("{fixture_name}/")) {
                Some(rest) => format!("{PROJECT_DIR_PLACEHOLDER}/{rest}"),
                None => normalized,
            }
        })
        .collect();
    names.sort();
    let ours = serde_json::to_value(&names).unwrap();
    let reference = read_json(&reference_dir.join("artifact_names.json"));
    reports.extend(compare("artifact_names.json", &ours, &reference));

    assert!(
        reports.is_empty(),
        "golden parity mismatches for fixture `{fixture_name}`:\n{}\n\nEach line is \
         `path: ours=<value> legacy=<value>`. If a difference is genuinely intended, it \
         belongs in `generate_reference.py`'s documented exclusion list with a reason — \
         never regenerate the references to paper over a regression.",
        reports.join("\n")
    );
}

#[test]
fn node_app_matches_legacy() {
    run_one_fixture("node_app");
}

#[test]
fn python_app_matches_legacy() {
    run_one_fixture("python_app");
}

#[test]
fn mixed_stack_matches_legacy() {
    run_one_fixture("mixed_stack");
}
