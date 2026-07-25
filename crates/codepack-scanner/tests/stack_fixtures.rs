//! End-to-end tests against the checked-in per-stack fixtures under `tests/fixtures/`:
//! stack detection feeds directly into `build_export_plan`'s pruning, exercising base
//! + stack-ignore pruning together (not just the base set alone).

use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use codepack_scanner::{
    ExportIgnoreRules, ScanOptions, build_export_plan, detect_stacks, no_safety_classification,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn plan_for(root: &Path) -> codepack_scanner::ExportPlan {
    let options = ScanOptions::default();
    let rules = ExportIgnoreRules::from_project_and_config(root, &options);
    build_export_plan(
        root,
        &options,
        &rules,
        &no_safety_classification,
        &CancellationToken::new(),
    )
    .unwrap()
}

#[test]
fn node_fixture_detects_stack_and_prunes_node_modules() {
    let root = fixture("node");
    let stacks = detect_stacks(&root);
    assert!(stacks.iter().any(|s| s.name == "Node.js"));

    let plan = plan_for(&root);
    let included: Vec<&str> = plan
        .included_files
        .iter()
        .map(|f| f.relative_path.as_str())
        .collect();
    assert!(included.contains(&"package.json"));
    assert!(included.iter().any(|p| p.ends_with("index.js")));
    assert!(included.iter().any(|p| p.ends_with("util.js")));
    assert!(!included.iter().any(|p| p.contains("node_modules")));
    assert!(plan.skipped_dirs.iter().any(|d| d.contains("node_modules")));
}

#[test]
fn python_fixture_detects_stack_and_prunes_venv() {
    let root = fixture("python");
    let stacks = detect_stacks(&root);
    assert!(stacks.iter().any(|s| s.name == "Python"));

    let plan = plan_for(&root);
    let included: Vec<&str> = plan
        .included_files
        .iter()
        .map(|f| f.relative_path.as_str())
        .collect();
    assert!(included.contains(&"pyproject.toml"));
    assert!(included.iter().any(|p| p.ends_with("main.py")));
    assert!(included.iter().any(|p| p.ends_with("util.py")));
    assert!(!included.iter().any(|p| p.contains(".venv")));
    assert!(
        plan.skipped_dirs
            .iter()
            .any(|d| d.to_lowercase().contains(".venv"))
    );
}

#[test]
fn monorepo_fixture_detects_both_stacks_and_unions_their_ignored_dirs() {
    let root = fixture("monorepo");
    let stack_names: Vec<String> = detect_stacks(&root).into_iter().map(|s| s.name).collect();
    assert!(stack_names.contains(&"Node.js".to_string()));
    assert!(stack_names.contains(&"Python".to_string()));

    let plan = plan_for(&root);
    let included: Vec<&str> = plan
        .included_files
        .iter()
        .map(|f| f.relative_path.as_str())
        .collect();
    assert!(included.iter().any(|p| p.ends_with("index.js")));
    assert!(included.iter().any(|p| p.ends_with("app.py")));
    // node_modules is base-ignored regardless of stack detection.
    assert!(!included.iter().any(|p| p.contains("node_modules")));
    // htmlcov is *only* pruned via the Python stack rule (not in the base set) —
    // proves the union of both matched stacks' extra directories is actually applied,
    // not just the primary (most-markers) stack's.
    assert!(!included.iter().any(|p| p.contains("htmlcov")));
}
