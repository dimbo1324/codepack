//! End-to-end `.exportignore` behavior through the public `build_export_plan` API,
//! using synthetic temporary trees (not checked-in fixtures — `.exportignore` content
//! is the point under test, so it is easier to read inline per test).

use std::fs;

use codepack_core::CancellationToken;
use codepack_scanner::{
    ExportIgnoreRules, ScanOptions, build_export_plan, no_safety_classification,
};

fn included_paths(plan: &codepack_scanner::ExportPlan) -> Vec<String> {
    plan.included_files
        .iter()
        .map(|f| f.relative_path.clone())
        .collect()
}

#[test]
fn exportignore_glob_rule_excludes_matching_files_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".exportignore"), "*.log\n*.tmp\n").unwrap();
    fs::write(dir.path().join("main.py"), "x").unwrap();
    fs::write(dir.path().join("debug.log"), "x").unwrap();
    fs::write(dir.path().join("cache.tmp"), "x").unwrap();

    let options = ScanOptions::default();
    let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
    let plan = build_export_plan(
        dir.path(),
        &options,
        &rules,
        &no_safety_classification,
        &CancellationToken::new(),
    )
    .unwrap();

    let included = included_paths(&plan);
    assert!(included.contains(&"main.py".to_string()));
    assert!(!included.contains(&"debug.log".to_string()));
    assert!(!included.contains(&"cache.tmp".to_string()));
    assert_eq!(plan.excluded_files.len(), 2);
    assert!(plan.excluded_files.iter().all(|f| f.severity == "medium"));
}

#[test]
fn exportignore_directory_rule_prunes_the_whole_subtree() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".exportignore"), "private/\n").unwrap();
    fs::create_dir_all(dir.path().join("private/nested")).unwrap();
    fs::write(dir.path().join("private/nested/secret.txt"), "x").unwrap();
    fs::write(dir.path().join("public.py"), "x").unwrap();

    let options = ScanOptions::default();
    let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
    let plan = build_export_plan(
        dir.path(),
        &options,
        &rules,
        &no_safety_classification,
        &CancellationToken::new(),
    )
    .unwrap();

    let included = included_paths(&plan);
    assert!(included.contains(&"public.py".to_string()));
    assert!(!included.iter().any(|p| p.contains("private")));
    // The directory is pruned compactly: no per-file entry for the excluded subtree.
    assert!(plan.excluded_files.is_empty());
    assert!(
        plan.skipped_dirs
            .iter()
            .any(|d| d.starts_with(".\\private"))
    );
}

#[test]
fn negation_rescues_a_file_from_a_glob_rule_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".exportignore"),
        "*.secret\n!shared.secret\n",
    )
    .unwrap();
    fs::write(dir.path().join("shared.secret"), "x").unwrap();
    fs::write(dir.path().join("other.secret"), "x").unwrap();

    let options = ScanOptions::default();
    let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
    let plan = build_export_plan(
        dir.path(),
        &options,
        &rules,
        &no_safety_classification,
        &CancellationToken::new(),
    )
    .unwrap();

    let included = included_paths(&plan);
    assert!(included.contains(&"shared.secret".to_string()));
    assert!(!included.contains(&"other.secret".to_string()));
}

#[test]
fn custom_excluded_extension_from_config_applies_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("module.pyc"), "x").unwrap();
    fs::write(dir.path().join("module.py"), "x").unwrap();

    let options = ScanOptions {
        custom_excluded_extensions: vec!["pyc".to_string()],
        ..ScanOptions::default()
    };
    let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
    let plan = build_export_plan(
        dir.path(),
        &options,
        &rules,
        &no_safety_classification,
        &CancellationToken::new(),
    )
    .unwrap();

    let included = included_paths(&plan);
    assert!(included.contains(&"module.py".to_string()));
    assert!(!included.contains(&"module.pyc".to_string()));
}
