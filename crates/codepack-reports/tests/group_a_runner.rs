//! End-to-end coverage for Group G's runner against Group A's five report jobs:
//! successful runs, fault tolerance (a forced failure and a forced panic must not
//! stop the rest of the run), and profile gating.

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_reports::context::Inventory;
use codepack_reports::plugin::{ReportJob, run_reports};
use codepack_reports::plugins_json::write_report_plugins_json;
use codepack_reports::project_profile::write_project_profile_json;
use codepack_reports::reports::group_a_jobs;
use codepack_reports::{ReportContext, ReportError};
use codepack_scanner::{ExportIgnoreRules, ExportPlan, ScanOptions, build_export_plan};

fn write_fixture_project(root: &std::path::Path) {
    std::fs::write(
        root.join("main.py"),
        "# TODO: refactor this\nprint('hello')\n",
    )
    .unwrap();
    std::fs::write(root.join("README.md"), "# Fixture project\n").unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/app.py"),
        "x = 1\ny = 2\n# FIXME: handle errors\n",
    )
    .unwrap();
    std::fs::write(
        root.join("package.json"),
        r#"{"dependencies": {"react": "18.0.0"}}"#,
    )
    .unwrap();
}

fn build_plan(root: &std::path::Path) -> ExportPlan {
    build_export_plan(
        root,
        &ScanOptions::default(),
        &ExportIgnoreRules::default(),
        &codepack_scanner::no_safety_classification,
        &CancellationToken::new(),
    )
    .unwrap()
}

#[test]
fn group_a_jobs_run_end_to_end_for_full_profile() {
    let project = tempfile::tempdir().unwrap();
    write_fixture_project(project.path());
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

    let jobs = group_a_jobs();
    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert!(!summary.cancelled);
    assert!(
        summary.failed.is_empty(),
        "unexpected failures: {:?}",
        summary.failed
    );
    assert_eq!(summary.succeeded.len(), 5);

    for job in &jobs {
        let output = out_dir.path().join(job.filename);
        assert!(output.exists(), "{} was not written", job.filename);
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(!content.trim().is_empty(), "{} is empty", job.filename);
    }

    write_report_plugins_json(&jobs, out_dir.path()).unwrap();
    let plugins_content =
        std::fs::read_to_string(out_dir.path().join("REPORT_PLUGINS.json")).unwrap();
    let plugins_json: serde_json::Value = serde_json::from_str(&plugins_content).unwrap();
    let entries = plugins_json.as_array().unwrap();
    assert_eq!(entries.len(), 5);
    for entry in entries {
        assert!(entry.get("filename").is_some());
        assert!(entry.get("profiles").is_some());
        assert!(entry.get("description").is_some());
    }

    let profile_output = out_dir.path().join("PROJECT_PROFILE.json");
    write_project_profile_json(&ctx, &profile_output).unwrap();
    let profile_content = std::fs::read_to_string(&profile_output).unwrap();
    let profile_json: serde_json::Value = serde_json::from_str(&profile_content).unwrap();
    for key in [
        "schema_version",
        "project_type",
        "detected_stack",
        "stack_by_group",
        "package_managers",
        "entrypoints",
        "commands",
        "capabilities",
        "counts",
        "risk_level",
        "important_config_files",
    ] {
        assert!(
            profile_json.get(key).is_some(),
            "missing PROJECT_PROFILE.json key: {key}"
        );
    }
}

fn failing_job() -> ReportJob {
    ReportJob {
        filename: "ZZ_forced_failure.txt",
        profiles: codepack_reports::profile::ALL_PROFILES,
        description: "deliberately fails for the fault-tolerance test",
        run: |_ctx, _output| {
            Err(ReportError::Write {
                path: std::path::PathBuf::from("nowhere"),
                source: std::io::Error::other("forced failure for test coverage"),
            })
        },
    }
}

fn panicking_job() -> ReportJob {
    ReportJob {
        filename: "ZZ_forced_panic.txt",
        profiles: codepack_reports::profile::ALL_PROFILES,
        description: "deliberately panics for the fault-tolerance test",
        run: |_ctx, _output| panic!("forced panic for test coverage"),
    }
}

#[test]
fn a_single_failing_or_panicking_job_does_not_stop_the_rest_of_the_run() {
    let project = tempfile::tempdir().unwrap();
    write_fixture_project(project.path());
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

    let mut jobs: Vec<ReportJob> = group_a_jobs().into_iter().collect();
    jobs.push(failing_job());
    jobs.push(panicking_job());

    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert_eq!(
        summary.succeeded.len(),
        5,
        "every Group A job should still complete"
    );
    assert_eq!(summary.failed.len(), 2);

    let error_for_failure = out_dir.path().join("ERROR_ZZ_forced_failure.txt.txt");
    assert!(error_for_failure.exists());
    let content = std::fs::read_to_string(&error_for_failure).unwrap();
    assert!(content.contains("ZZ_forced_failure.txt"));
    assert!(content.contains("forced failure for test coverage"));

    let error_for_panic = out_dir.path().join("ERROR_ZZ_forced_panic.txt.txt");
    assert!(error_for_panic.exists());
    let panic_content = std::fs::read_to_string(&error_for_panic).unwrap();
    assert!(panic_content.contains("forced panic for test coverage"));

    for job in group_a_jobs() {
        assert!(out_dir.path().join(job.filename).exists());
    }
}

#[test]
fn minimal_profile_only_runs_summary_among_group_a_jobs() {
    let project = tempfile::tempdir().unwrap();
    write_fixture_project(project.path());
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
        profile: "minimal",
    };

    let jobs = group_a_jobs();
    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert_eq!(summary.succeeded, vec!["01_summary.txt"]);
    assert_eq!(summary.skipped.len(), 4);
    assert!(out_dir.path().join("01_summary.txt").exists());
    assert!(!out_dir.path().join("02_file_statistics.txt").exists());
    assert!(!out_dir.path().join("07_todo_fixme.txt").exists());
    assert!(!out_dir.path().join("08_code_metrics.txt").exists());
    assert!(!out_dir.path().join("25_large_files_report.md").exists());
}

#[test]
fn quick_profile_runs_every_group_a_job() {
    let project = tempfile::tempdir().unwrap();
    write_fixture_project(project.path());
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
        profile: "quick",
    };

    let jobs = group_a_jobs();
    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert_eq!(summary.succeeded.len(), 5);
    assert!(summary.skipped.is_empty());
}
