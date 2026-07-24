//! The stage's closest thing to an overall acceptance test: runs every report group
//! (G through G-finish) end-to-end for one multi-stack, Git-backed fixture under the
//! `full` export profile, confirming the complete expected file set from
//! `task-checklist.md`'s Group listing is produced; then re-runs a representative
//! subset of jobs under two other profiles (`minimal`, `security`) against the
//! reconstructed profile-gating table (`codepack_reports::profile`); then performs a
//! crate-wide invariant-I3 sweep: a secret is planted in a script, a config file, a
//! Git commit message, and a TODO comment, a **real** `codepack_security::ScanResult`
//! is computed against the same fixture (so `06_security_scan.*` also participates,
//! not just its `None`-scan placeholder), and the raw planted secret is asserted to
//! appear nowhere in the entire reports output directory tree.

use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_core::{ArchiveBuildResult, CopyStats, ExportPaths, TextDumpStats};
use codepack_reports::context::Inventory;
use codepack_reports::plugin::{ReportJob, run_reports};
use codepack_reports::plugins_json::write_report_plugins_json;
use codepack_reports::project_profile::write_project_profile_json;
use codepack_reports::reports::{
    group_a_jobs, group_b_jobs, group_c_jobs, group_d_jobs, group_e_jobs, group_f_jobs,
    group_g_finish_jobs,
};
use codepack_reports::{IndexInput, ManifestInput, ReportContext, write_index_md, write_manifest};
use codepack_scanner::{ExportIgnoreRules, ExportPlan, ScanOptions, build_export_plan};
use codepack_security::scan_project;
use git2::{IndexAddOption, Repository, Signature};

/// The raw secret *value* planted everywhere below. It is always paired with a
/// `redact_secrets` keyword (`API_KEY`) so every plant is actually redactable by this
/// crate's real, currently-implemented mechanism ([`codepack_security::redact_secrets`],
/// keyword-based — see that crate's own module doc): a provider-signature-only value
/// with no adjacent keyword is a known, separate detection layer (`scan_project`'s
/// finding detector) that `redact_line` does not independently catch, so it would not
/// exercise the invariant this sweep is actually verifying.
const SECRET_VALUE: &str = "zZ9xQ7vLwPmR3sT8uAbCdEfGhIjKlmNoPqRs";

fn write_fixture(root: &Path) {
    std::fs::write(
        root.join("package.json"),
        r#"{"name": "demo", "scripts": {"build": "vite build", "test": "vitest run"}, "dependencies": {"react": "18.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();
    std::fs::write(
        root.join("go.mod"),
        "module example.com/app\n\nrequire github.com/foo/bar v1.0.0\n",
    )
    .unwrap();
    std::fs::write(root.join("README.md"), "# Demo\n").unwrap();

    // (1) A script assigning the secret directly (no TODO marker involved).
    std::fs::write(
        root.join("deploy.sh"),
        format!("#!/bin/sh\nexport API_KEY={SECRET_VALUE}\n"),
    )
    .unwrap();

    // (2) A config file (docker-compose environment) quoting the secret. List-style
    // `- KEY=value` matches `parse_compose_services`'s indent-sensitive parser (see
    // `reports/docker.rs`'s own fixture); a mapping-style `KEY: value` here would
    // never be captured into the parsed service at all, so it would not exercise the
    // redaction path this sweep is meant to check.
    std::fs::write(
        root.join("docker-compose.yml"),
        format!(
            "services:\n  app:\n    image: demo:latest\n    environment:\n      - API_KEY={SECRET_VALUE}\n"
        ),
    )
    .unwrap();

    // (4) A TODO comment quoting the secret, distinct from the plain script plant.
    std::fs::write(
        root.join("app.py"),
        format!("# TODO: rotate API_KEY={SECRET_VALUE}\nprint('hello')\n"),
    )
    .unwrap();

    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src").join("main.go"),
        "package main\n\nfunc main() {}\n",
    )
    .unwrap();
}

fn init_git_history_with_secret_commit(root: &Path) {
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
        // (3) A Git commit message quoting the secret.
        &format!("init: rotate API_KEY={SECRET_VALUE} before merging"),
        &tree,
        &[],
    )
    .unwrap();
}

fn build_plan(root: &Path) -> ExportPlan {
    build_export_plan(
        root,
        &ScanOptions::default(),
        &ExportIgnoreRules::default(),
        &CancellationToken::new(),
    )
    .unwrap()
}

fn all_full_catalog_jobs() -> Vec<ReportJob> {
    let mut jobs = Vec::new();
    jobs.extend(group_a_jobs());
    jobs.extend(group_b_jobs());
    jobs.extend(group_c_jobs());
    jobs.extend(group_d_jobs());
    jobs.extend(group_e_jobs());
    jobs.extend(group_f_jobs());
    jobs.extend(group_g_finish_jobs());
    jobs
}

fn read_dir_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            read_dir_recursive(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn full_profile_produces_the_complete_expected_file_set() {
    let project = tempfile::tempdir().unwrap();
    write_fixture(project.path());
    init_git_history_with_secret_commit(project.path());

    let plan = build_plan(project.path());
    let inventory = Inventory::from_plan(&plan);
    let config = Config::default();
    let cancel = CancellationToken::new();

    let relative_files: Vec<PathBuf> = vec![
        PathBuf::from("app.py"),
        PathBuf::from("deploy.sh"),
        PathBuf::from("docker-compose.yml"),
        PathBuf::from("package.json"),
        PathBuf::from("pnpm-lock.yaml"),
        PathBuf::from("go.mod"),
        PathBuf::from("README.md"),
        Path::new("src").join("main.go"),
    ];
    let scan = scan_project(project.path(), &relative_files, None, &cancel).unwrap();

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

    let jobs = all_full_catalog_jobs();
    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert!(
        summary.failed.is_empty(),
        "unexpected failures: {:?}",
        summary.failed
    );

    write_report_plugins_json(&jobs, out_dir.path()).unwrap();
    let profile_file = out_dir.path().join("PROJECT_PROFILE.json");
    write_project_profile_json(&ctx, &profile_file).unwrap();

    let paths = ExportPaths {
        desktop: project.path().to_path_buf(),
        source_root: project.path().to_path_buf(),
        project_name: "demo".to_string(),
        bundle_name: "demo_export".to_string(),
        staging_dir: project.path().to_path_buf(),
        final_zip: project.path().join("demo.zip"),
        archive_set_dir: project.path().join("demo_archive_set"),
        project_dir: project.path().to_path_buf(),
        reports_dir: out_dir.path().to_path_buf(),
        insights_dir: out_dir.path().to_path_buf(),
        manifest_file: out_dir.path().join("manifest.json"),
        project_profile_file: profile_file.clone(),
        index_file: out_dir.path().join("INDEX.md"),
        structure_report: out_dir.path().join("01_structure.txt"),
        git_report: out_dir.path().join("02_git.txt"),
        text_dump: out_dir.path().join("03_text_dump.txt"),
    };
    let manifest_input = ManifestInput {
        app_name: "codepack",
        app_version: "0.1.0",
        generated_at: plan.generated_at.clone(),
        paths: &paths,
        cancelled: false,
        config: &config,
        extra_ignored_dirs: &[],
        copy_stats: &CopyStats::default(),
        text_stats: &TextDumpStats::default(),
        archive_result: None::<&ArchiveBuildResult>,
        diff_selection: None,
    };
    write_manifest(&manifest_input, &out_dir.path().join("manifest.json")).unwrap();
    write_index_md(
        &IndexInput {
            app_name: "codepack",
            app_version: "0.1.0",
            generated_at: plan.generated_at.clone(),
            project_name: "demo",
            config: &config,
            extra_ignored_dirs: &[],
        },
        &out_dir.path().join("INDEX.md"),
    )
    .unwrap();

    // Every numbered report + fixed artifact this crate owns must exist.
    let expected_flat_files = [
        "01_summary.txt",
        "02_file_statistics.txt",
        "03_dependencies.txt",
        "04_scripts.txt",
        "05_git_deep.txt",
        "06_security_scan.txt",
        "06_security_scan.json",
        "06_security_scan.sarif",
        "07_todo_fixme.txt",
        "08_code_metrics.txt",
        "09_config.txt",
        "10_docker.txt",
        "11_routes_and_pages.txt",
        "12_ai_context_pack.md",
        "13_runbook.md",
        "14_dependency_graph.md",
        "14_dependency_graph.mmd",
        "15_architecture_report.md",
        "16_key_files_report.md",
        "17_code_quality_report.md",
        "18_api_surface_report.md",
        "19_frontend_report.md",
        "20_backend_report.md",
        "21_git_timeline_report.md",
        "22_project_health_report.md",
        "23_refactoring_opportunities.md",
        "24_architecture_map.md",
        "25_large_files_report.md",
        "26_dependency_intelligence.md",
        "REPORT_DASHBOARD.html",
        "REPORT_PLUGINS.json",
        "PROJECT_PROFILE.json",
        "manifest.json",
        "INDEX.md",
    ];
    for name in expected_flat_files {
        assert!(
            out_dir.path().join(name).exists(),
            "expected report file missing: {name}"
        );
    }

    let ai_context_dir = out_dir.path().join("AI_CONTEXT");
    for name in [
        "00_PROJECT_OVERVIEW.md",
        "01_ARCHITECTURE.md",
        "02_FILE_TREE.md",
        "03_ENTRYPOINTS.md",
        "04_KEY_FILES.md",
        "05_DEPENDENCIES.md",
        "06_SECURITY_NOTES.md",
        "07_TODO_FIXME.md",
        "08_REFACTORING_TARGETS.md",
        "09_PROMPT_FOR_CODEX.md",
        "10_SCRIPTS.md",
    ] {
        assert!(
            ai_context_dir.join(name).exists(),
            "expected AI_CONTEXT file missing: {name}"
        );
    }
    assert!(
        out_dir
            .path()
            .join("AI_PROMPTS")
            .join("CUSTOM_PROMPT.md")
            .exists()
    );
    let ai_prompts_entries: Vec<_> = std::fs::read_dir(out_dir.path().join("AI_PROMPTS"))
        .unwrap()
        .collect();
    assert_eq!(
        ai_prompts_entries.len(),
        1,
        "AI_PROMPTS/ must contain exactly CUSTOM_PROMPT.md"
    );

    // --- Invariant I3 sweep: the raw planted secret must appear nowhere. ---
    let mut all_files = Vec::new();
    read_dir_recursive(out_dir.path(), &mut all_files);
    assert!(!all_files.is_empty());
    for path in &all_files {
        if let Ok(content) = std::fs::read_to_string(path) {
            assert!(
                !content.contains(SECRET_VALUE),
                "planted secret leaked in {}",
                path.display()
            );
        }
    }

    // Cross-check that redaction actually fired somewhere rather than the secret
    // simply never having been reachable by any report in the first place.
    let todo_report = std::fs::read_to_string(out_dir.path().join("07_todo_fixme.txt")).unwrap();
    assert!(todo_report.contains("TODO"));
    assert!(todo_report.contains("REDACTED"));

    let docker_report = std::fs::read_to_string(out_dir.path().join("10_docker.txt")).unwrap();
    assert!(docker_report.contains("REDACTED"));

    let git_deep = std::fs::read_to_string(out_dir.path().join("05_git_deep.txt")).unwrap();
    assert!(git_deep.contains("REDACTED"));

    let security_json =
        std::fs::read_to_string(out_dir.path().join("06_security_scan.json")).unwrap();
    assert!(security_json.contains("REDACTED") || security_json.contains("\"potential_secrets\""));
}

#[test]
fn minimal_and_security_profiles_match_the_reconstructed_gating_table() {
    let project = tempfile::tempdir().unwrap();
    write_fixture(project.path());

    let plan = build_plan(project.path());
    let inventory = Inventory::from_plan(&plan);
    let config = Config::default();
    let cancel = CancellationToken::new();

    let jobs = all_full_catalog_jobs();

    for (profile_name, must_run, must_skip) in [
        (
            "minimal",
            vec![
                "01_summary.txt",
                "12_ai_context_pack.md",
                "24_architecture_map.md",
            ],
            vec![
                "02_file_statistics.txt",
                "14_dependency_graph.md",
                "19_frontend_report.md",
            ],
        ),
        (
            "security",
            vec![
                "09_config.txt",
                "18_api_surface_report.md",
                "06_security_scan.txt",
            ],
            vec!["19_frontend_report.md", "20_backend_report.md"],
        ),
    ] {
        let ctx = ReportContext {
            source_root: project.path().to_path_buf(),
            staging_root: project.path().to_path_buf(),
            inventory: &inventory,
            plan: &plan,
            scan: None,
            diff: None,
            config: &config,
            cancel: &cancel,
            profile: profile_name,
        };
        let out_dir = tempfile::tempdir().unwrap();
        let summary = run_reports(&jobs, &ctx, out_dir.path());
        assert!(
            summary.failed.is_empty(),
            "unexpected failures for profile {profile_name}: {:?}",
            summary.failed
        );

        for name in &must_run {
            assert!(
                out_dir.path().join(name).exists(),
                "profile {profile_name} should have produced {name}"
            );
        }
        for name in &must_skip {
            assert!(
                !out_dir.path().join(name).exists(),
                "profile {profile_name} should NOT have produced {name}"
            );
        }
    }
}

#[test]
fn ai_context_folder_is_gated_out_for_security_but_ai_prompts_is_not() {
    let project = tempfile::tempdir().unwrap();
    write_fixture(project.path());

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
        profile: "security",
    };

    let jobs = group_f_jobs();
    let out_dir = tempfile::tempdir().unwrap();
    let summary = run_reports(&jobs, &ctx, out_dir.path());

    assert!(summary.failed.is_empty());
    assert!(!out_dir.path().join("AI_CONTEXT").exists());
    assert!(
        out_dir
            .path()
            .join("AI_PROMPTS")
            .join("CUSTOM_PROMPT.md")
            .exists()
    );
}
