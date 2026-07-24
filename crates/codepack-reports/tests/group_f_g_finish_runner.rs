//! End-to-end coverage for Group F (the AI bundle) and Group G-finish's report-writer
//! job (`REPORT_DASHBOARD.html`) run through [`run_reports`], plus the same
//! fault-tolerance guarantee already proven for every earlier group here extended to
//! these new jobs — matching the pattern established by `group_a_runner.rs`,
//! `group_b_d_runner.rs`, and `group_c_e_runner.rs`. The full-catalog acceptance test
//! and the crate-wide invariant-I3 sweep live in `full_catalog_runner.rs`.

use std::path::Path;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_reports::context::Inventory;
use codepack_reports::plugin::{ReportJob, run_reports};
use codepack_reports::reports::{group_f_jobs, group_g_finish_jobs};
use codepack_reports::{ReportContext, ReportError};
use codepack_scanner::{ExportIgnoreRules, ExportPlan, ScanOptions, build_export_plan};

fn write_fixture_project(root: &Path) {
    std::fs::write(
        root.join("package.json"),
        r#"{"name": "demo", "scripts": {"build": "vite build"}, "dependencies": {"react": "18.0.0"}}"#,
    )
    .unwrap();
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

#[test]
fn group_f_and_group_g_finish_jobs_run_end_to_end() {
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

    let mut jobs: Vec<ReportJob> = Vec::new();
    jobs.extend(group_f_jobs());
    jobs.extend(group_g_finish_jobs());

    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert!(
        summary.failed.is_empty(),
        "unexpected failures: {:?}",
        summary.failed
    );
    assert_eq!(summary.succeeded.len(), jobs.len());

    assert!(out_dir.path().join("12_ai_context_pack.md").exists());
    assert!(out_dir.path().join("13_runbook.md").exists());
    assert!(out_dir.path().join("REPORT_DASHBOARD.html").exists());
    assert!(
        out_dir
            .path()
            .join("AI_CONTEXT")
            .join("00_PROJECT_OVERVIEW.md")
            .exists()
    );
    assert!(
        out_dir
            .path()
            .join("AI_PROMPTS")
            .join("CUSTOM_PROMPT.md")
            .exists()
    );

    let dashboard = std::fs::read_to_string(out_dir.path().join("REPORT_DASHBOARD.html")).unwrap();
    assert!(dashboard.contains("Project Reports Dashboard"));
}

fn failing_runbook_job() -> ReportJob {
    ReportJob {
        filename: "13_runbook.md",
        profiles: codepack_reports::profile::ALL_PROFILES,
        description: "deliberately fails in place of the real 13_runbook job",
        run: |_ctx, _output| {
            Err(ReportError::Write {
                path: std::path::PathBuf::from("nowhere"),
                source: std::io::Error::other("forced failure for Group F fault-tolerance test"),
            })
        },
    }
}

#[test]
fn a_single_failing_group_f_job_does_not_stop_the_rest_of_the_run() {
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

    let mut jobs: Vec<ReportJob> = group_f_jobs().into_iter().collect();
    jobs[1] = failing_runbook_job();
    jobs.extend(group_g_finish_jobs());

    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert_eq!(summary.succeeded.len(), jobs.len() - 1);
    assert_eq!(summary.failed.len(), 1);
    let error_file = out_dir.path().join("ERROR_13_runbook.md.txt");
    assert!(error_file.exists());
    let content = std::fs::read_to_string(&error_file).unwrap();
    assert!(content.contains("forced failure for Group F fault-tolerance test"));

    assert!(out_dir.path().join("12_ai_context_pack.md").exists());
    assert!(
        out_dir
            .path()
            .join("AI_CONTEXT")
            .join("00_PROJECT_OVERVIEW.md")
            .exists()
    );
    assert!(
        out_dir
            .path()
            .join("AI_PROMPTS")
            .join("CUSTOM_PROMPT.md")
            .exists()
    );
    assert!(out_dir.path().join("REPORT_DASHBOARD.html").exists());
}
