//! End-to-end coverage for Group B (`06_security_scan.*`) and Group D (manifest
//! parsers) run through [`run_reports`], exercising at least two stacks (Node + Go)
//! simultaneously, plus the fault-tolerance guarantee already proven for Group A here
//! extended to these new jobs.

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_reports::context::Inventory;
use codepack_reports::plugin::run_reports;
use codepack_reports::reports::{group_b_jobs, group_d_jobs};
use codepack_reports::{ReportContext, ReportError};
use codepack_scanner::{ExportIgnoreRules, ExportPlan, ScanOptions, build_export_plan};
use codepack_security::scan::{Finding, FindingKind, ScanResult, ScanSummary};

fn write_two_stack_fixture(root: &std::path::Path) {
    std::fs::write(
        root.join("package.json"),
        r#"{"name": "demo", "scripts": {"build": "vite build"}, "dependencies": {"react": "18.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();
    std::fs::write(
        root.join("go.mod"),
        "module example.com/app\n\nrequire github.com/foo/bar v1.0.0\n",
    )
    .unwrap();
    std::fs::write(root.join("Dockerfile"), "FROM scratch\n").unwrap();
    std::fs::write(
        root.join("docker-compose.yml"),
        "services:\n  app:\n    image: demo:latest\n  db:\n    image: postgres:16\n",
    )
    .unwrap();
    std::fs::write(root.join("tsconfig.json"), "{}").unwrap();
}

fn build_plan(root: &std::path::Path) -> ExportPlan {
    build_export_plan(
        root,
        &ScanOptions::default(),
        &ExportIgnoreRules::default(),
        &CancellationToken::new(),
    )
    .unwrap()
}

#[test]
fn group_d_jobs_run_end_to_end_for_a_node_and_go_fixture() {
    let project = tempfile::tempdir().unwrap();
    write_two_stack_fixture(project.path());
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

    let jobs = group_d_jobs();
    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

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

    let dependencies = std::fs::read_to_string(out_dir.path().join("03_dependencies.txt")).unwrap();
    assert!(dependencies.contains("react"));

    let scripts = std::fs::read_to_string(out_dir.path().join("04_scripts.txt")).unwrap();
    assert!(scripts.contains("pnpm run build"));

    let config_report = std::fs::read_to_string(out_dir.path().join("09_config.txt")).unwrap();
    assert!(config_report.contains("tsconfig.json"));

    let docker = std::fs::read_to_string(out_dir.path().join("10_docker.txt")).unwrap();
    assert!(docker.contains("Service: app"));
    assert!(docker.contains("Service: db"));

    let intelligence =
        std::fs::read_to_string(out_dir.path().join("26_dependency_intelligence.md")).unwrap();
    assert!(intelligence.contains("example.com/app"));
}

#[test]
fn group_b_job_writes_security_scan_siblings_when_scan_is_supplied() {
    let project = tempfile::tempdir().unwrap();
    write_two_stack_fixture(project.path());
    let plan = build_plan(project.path());
    let inventory = Inventory::from_plan(&plan);
    let config = Config::default();
    let cancel = CancellationToken::new();
    let scan = ScanResult {
        summary: ScanSummary {
            sensitive_files: 1,
            potential_secrets: 0,
            risky_code: 0,
            total_findings: 1,
        },
        findings: vec![Finding {
            kind: FindingKind::SensitiveFile,
            severity: "high".to_string(),
            confidence: "high".to_string(),
            file: ".\\.env".to_string(),
            line: None,
            rule: "sensitive_filename".to_string(),
            message: "Sensitive-looking filename or suffix.".to_string(),
        }],
    };
    let ctx = ReportContext {
        source_root: project.path().to_path_buf(),
        staging_root: project.path().to_path_buf(),
        inventory: &inventory,
        plan: &plan,
        scan: Some(&scan),
        diff: None,
        config: &config,
        cancel: &cancel,
        profile: "full",
    };

    let jobs = group_b_jobs();
    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert!(summary.failed.is_empty());
    assert_eq!(summary.succeeded, vec!["06_security_scan.txt"]);
    assert!(out_dir.path().join("06_security_scan.txt").exists());
    assert!(out_dir.path().join("06_security_scan.json").exists());
    assert!(out_dir.path().join("06_security_scan.sarif").exists());
}

#[test]
fn group_b_job_is_a_placeholder_when_scan_is_not_wired_in() {
    let project = tempfile::tempdir().unwrap();
    write_two_stack_fixture(project.path());
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

    let jobs = group_b_jobs();
    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert!(summary.failed.is_empty());
    assert_eq!(summary.succeeded, vec!["06_security_scan.txt"]);
    assert!(!out_dir.path().join("06_security_scan.json").exists());
    assert!(!out_dir.path().join("06_security_scan.sarif").exists());
}

fn failing_config_job() -> codepack_reports::plugin::ReportJob {
    codepack_reports::plugin::ReportJob {
        filename: "09_config.txt",
        profiles: codepack_reports::profile::ALL_PROFILES,
        description: "deliberately fails in place of the real 09_config job",
        run: |_ctx, _output| {
            Err(ReportError::Write {
                path: std::path::PathBuf::from("nowhere"),
                source: std::io::Error::other("forced failure for Group D fault-tolerance test"),
            })
        },
    }
}

#[test]
fn a_single_failing_group_d_job_does_not_stop_the_rest_of_the_run() {
    let project = tempfile::tempdir().unwrap();
    write_two_stack_fixture(project.path());
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

    let mut jobs: Vec<codepack_reports::plugin::ReportJob> = group_d_jobs().into_iter().collect();
    jobs[2] = failing_config_job();

    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert_eq!(summary.succeeded.len(), 4);
    assert_eq!(summary.failed.len(), 1);
    let error_file = out_dir.path().join("ERROR_09_config.txt.txt");
    assert!(error_file.exists());
    let content = std::fs::read_to_string(&error_file).unwrap();
    assert!(content.contains("forced failure for Group D fault-tolerance test"));

    assert!(out_dir.path().join("03_dependencies.txt").exists());
    assert!(out_dir.path().join("04_scripts.txt").exists());
    assert!(out_dir.path().join("10_docker.txt").exists());
    assert!(
        out_dir
            .path()
            .join("26_dependency_intelligence.md")
            .exists()
    );
}
