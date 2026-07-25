//! End-to-end coverage for Group C (`git2`-based reports) and Group E
//! (heuristic/derived reports) run through [`run_reports`] against a multi-stack
//! fixture (Node + Python + Go), plus the fault-tolerance guarantee already proven for
//! Groups A/B/D here extended to these new jobs, plus a dedicated check that this
//! crate never shells out to a `git` binary.

use std::path::Path;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_reports::context::Inventory;
use codepack_reports::plugin::run_reports;
use codepack_reports::reports::{group_c_jobs, group_e_jobs};
use codepack_reports::{ReportContext, ReportError};
use codepack_scanner::{ExportIgnoreRules, ExportPlan, ScanOptions, build_export_plan};
use git2::{IndexAddOption, Repository, Signature};

fn write_multi_stack_fixture(root: &Path) {
    std::fs::write(
        root.join("package.json"),
        r#"{"name": "demo", "scripts": {"build": "vite build"}, "dependencies": {"react": "18.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(root.join("utils.py"), "").unwrap();
    std::fs::create_dir_all(root.join("src").join("services")).unwrap();
    std::fs::write(
        root.join("src").join("services").join("api.py"),
        "import utils\n\ndef handle_request():\n    pass\n",
    )
    .unwrap();
    std::fs::write(
        root.join("go.mod"),
        "module example.com/app\n\nrequire github.com/foo/bar v1.0.0\n",
    )
    .unwrap();
    std::fs::write(root.join("main.go"), "package main\n\nfunc main() {}\n").unwrap();
    std::fs::write(root.join("README.md"), "# Demo\n").unwrap();
}

fn build_plan(root: &Path) -> ExportPlan {
    build_export_plan(
        root,
        &ScanOptions::default(),
        &ExportIgnoreRules::default(),
        &codepack_scanner::no_safety_classification,
        &CancellationToken::new(),
    )
    .unwrap()
}

fn init_git_history(root: &Path) {
    let repo = Repository::init(root).unwrap();
    let mut index = repo.index().unwrap();
    index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = Signature::now("Test User", "test@example.local").unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "init: import demo project",
        &tree,
        &[],
    )
    .unwrap();
}

#[test]
fn group_c_and_group_e_jobs_run_end_to_end_for_a_multi_stack_git_fixture() {
    let project = tempfile::tempdir().unwrap();
    write_multi_stack_fixture(project.path());
    init_git_history(project.path());

    let plan = build_plan(project.path());
    let inventory = Inventory::from_plan(&plan);
    let config = Config::default();
    let cancel = CancellationToken::new();
    let ctx = ReportContext {
        source_root: project.path().to_path_buf(),
        staging_root: project.path().to_path_buf(),
        inventory: &inventory,
        plan: &plan,
        scan: None,
        diff: None,
        config: &config,
        cancel: &cancel,
        profile: "full",
    };

    let mut jobs: Vec<codepack_reports::plugin::ReportJob> = Vec::new();
    jobs.extend(group_c_jobs());
    jobs.extend(group_e_jobs());

    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert!(
        summary.failed.is_empty(),
        "unexpected failures: {:?}",
        summary.failed
    );
    assert_eq!(summary.succeeded.len(), jobs.len());

    for job in &jobs {
        let output = out_dir.path().join(job.filename);
        assert!(output.exists(), "{} was not written", job.filename);
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(!content.trim().is_empty(), "{} is empty", job.filename);
    }

    // Group C: real Git history was found and read.
    let git_deep = std::fs::read_to_string(out_dir.path().join("05_git_deep.txt")).unwrap();
    assert!(git_deep.contains("$ git log --oneline --decorate --graph -20"));
    assert!(!git_deep.contains("No .git directory was found"));

    let git_timeline =
        std::fs::read_to_string(out_dir.path().join("21_git_timeline_report.md")).unwrap();
    assert!(git_timeline.contains("init: import demo project"));

    // Group E: the dependency graph's Mermaid sibling exists and resolved the Python edge.
    assert!(out_dir.path().join("14_dependency_graph.mmd").exists());
    let dependency_graph =
        std::fs::read_to_string(out_dir.path().join("14_dependency_graph.md")).unwrap();
    assert!(dependency_graph.contains("src\\services\\api.py"));

    let key_files = std::fs::read_to_string(out_dir.path().join("16_key_files_report.md")).unwrap();
    assert!(key_files.contains("main.go"));
}

fn failing_dependency_graph_job() -> codepack_reports::plugin::ReportJob {
    codepack_reports::plugin::ReportJob {
        filename: "14_dependency_graph.md",
        profiles: codepack_reports::profile::ALL_PROFILES,
        description: "deliberately fails in place of the real 14_dependency_graph job",
        run: |_ctx, _output| {
            Err(ReportError::Write {
                path: std::path::PathBuf::from("nowhere"),
                source: std::io::Error::other("forced failure for Group E fault-tolerance test"),
            })
        },
    }
}

#[test]
fn a_single_failing_group_e_job_does_not_stop_the_rest_of_the_run() {
    let project = tempfile::tempdir().unwrap();
    write_multi_stack_fixture(project.path());

    let plan = build_plan(project.path());
    let inventory = Inventory::from_plan(&plan);
    let config = Config::default();
    let cancel = CancellationToken::new();
    let ctx = ReportContext {
        source_root: project.path().to_path_buf(),
        staging_root: project.path().to_path_buf(),
        inventory: &inventory,
        plan: &plan,
        scan: None,
        diff: None,
        config: &config,
        cancel: &cancel,
        profile: "full",
    };

    let mut jobs: Vec<codepack_reports::plugin::ReportJob> = group_e_jobs().into_iter().collect();
    jobs[1] = failing_dependency_graph_job();

    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert_eq!(summary.succeeded.len(), jobs.len() - 1);
    assert_eq!(summary.failed.len(), 1);
    let error_file = out_dir.path().join("ERROR_14_dependency_graph.md.txt");
    assert!(error_file.exists());
    let content = std::fs::read_to_string(&error_file).unwrap();
    assert!(content.contains("forced failure for Group E fault-tolerance test"));

    assert!(out_dir.path().join("11_routes_and_pages.txt").exists());
    assert!(out_dir.path().join("15_architecture_report.md").exists());
    assert!(out_dir.path().join("24_architecture_map.md").exists());
}

#[test]
fn group_c_jobs_degrade_gracefully_outside_a_git_repository() {
    let project = tempfile::tempdir().unwrap();
    write_multi_stack_fixture(project.path());

    let plan = build_plan(project.path());
    let inventory = Inventory::from_plan(&plan);
    let config = Config::default();
    let cancel = CancellationToken::new();
    let ctx = ReportContext {
        source_root: project.path().to_path_buf(),
        staging_root: project.path().to_path_buf(),
        inventory: &inventory,
        plan: &plan,
        scan: None,
        diff: None,
        config: &config,
        cancel: &cancel,
        profile: "full",
    };

    let jobs = group_c_jobs();
    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert!(summary.failed.is_empty());
    assert_eq!(summary.succeeded.len(), 2);

    let git_deep = std::fs::read_to_string(out_dir.path().join("05_git_deep.txt")).unwrap();
    assert!(git_deep.contains("No .git directory was found in the selected root."));

    let git_timeline =
        std::fs::read_to_string(out_dir.path().join("21_git_timeline_report.md")).unwrap();
    assert!(git_timeline.contains("No `.git` directory was found in the selected root."));
}

/// Domain rule (`.ai/project/12-domain-rules.md`): "Network access is forbidden in
/// every crate"; this crate also must never shell out to an external `git` binary
/// (`git2` only). A static source scan is the practical check here — there is no
/// runtime signal that distinguishes "a subprocess was spawned" from "a `git2` FFI
/// call was made" without a much heavier sandboxing harness, so this test enforces
/// the invariant at the source level instead: no file under `src/` may reference
/// `std::process::Command`.
#[test]
fn crate_source_never_references_std_process_command() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offending_files = Vec::new();

    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap();
            if content.contains("process::Command") || content.contains("process::Stdio") {
                offending_files.push(path);
            }
        }
    }

    assert!(
        offending_files.is_empty(),
        "found forbidden std::process usage in: {offending_files:?}"
    );
}
