//! Shape-parity coverage (`task-checklist.md`'s S9 Verification section): one fixture
//! combining a `.py` file, a `.md` file, a `.env` file, and a small one-commit git
//! repo, run through the full eight-step pipeline once. Asserts every expected
//! top-level artifact *exists* — shape, not byte-identity, since content is
//! host/time-dependent (per this criterion's own wording). This is not a substitute
//! for a golden byte-for-byte comparison against the legacy Python tool (there is no
//! live legacy binary to run in this sandbox); `ae74299`/`b4d02a0`/`29070d9` already
//! golden-tested each step's own output *format* against the legacy source directly.

mod support;

use std::collections::HashMap;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_storage::open;
use support::{build_multi_type_fixture, zip_entry_names};

#[test]
fn a_multi_type_fixture_produces_every_expected_top_level_artifact() {
    let source = tempfile::tempdir().unwrap();
    build_multi_type_fixture(source.path());
    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut conn = open(&db_dir.path().join("codepack.db")).unwrap();
    let config = Config {
        keep_staging_folder: true,
        ..Config::default()
    };
    let (tx, _rx) = codepack_core::progress_channel();

    let outcome = codepack_engine::run_export(
        &mut conn,
        source.path(),
        output.path(),
        &config,
        &HashMap::new(),
        &tx,
        &CancellationToken::new(),
    )
    .unwrap();

    assert!(outcome.successful, "copy_stats = {:?}", outcome.copy_stats);
    let paths = &outcome.paths;
    let insights = &paths.insights_dir;

    // Step 7: manifest + index.
    assert!(paths.manifest_file.is_file(), "manifest.json");
    assert!(paths.index_file.is_file(), "INDEX.md");
    assert!(paths.project_profile_file.is_file(), "PROJECT_PROFILE.json");

    // Steps 3-5: structure / git / text dump.
    assert!(paths.structure_report.is_file(), "01_structure.txt");
    assert!(paths.git_report.is_file(), "02_git.txt");
    assert!(paths.text_dump.is_file(), "03_text_dump.txt");

    // Step 1: plan + diff artifacts.
    assert!(insights.join("28_export_plan.json").is_file());
    assert!(insights.join("28_export_plan.md").is_file());
    assert!(insights.join("29_export_comparison_report.md").is_file());

    // Step 6: the full report catalog (BLUEPRINT §A.7 groups A-G), plus
    // `REPORT_PLUGINS.json` and the dashboard.
    assert!(insights.join("REPORT_PLUGINS.json").is_file());
    assert!(insights.join("REPORT_DASHBOARD.html").is_file());

    let numbered_reports = [
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
    ];
    for name in numbered_reports {
        assert!(
            insights.join(name).is_file(),
            "expected report artifact missing: {name}"
        );
    }

    // Step 6's AI bundle folders (Group F).
    assert!(insights.join("AI_CONTEXT").is_dir(), "AI_CONTEXT/");
    assert!(
        insights.join("AI_CONTEXT/00_PROJECT_OVERVIEW.md").is_file(),
        "AI_CONTEXT/00_PROJECT_OVERVIEW.md"
    );
    assert!(insights.join("AI_PROMPTS").is_dir(), "AI_PROMPTS/");
    assert!(
        insights.join("AI_PROMPTS/CUSTOM_PROMPT.md").is_file(),
        "AI_PROMPTS/CUSTOM_PROMPT.md"
    );

    // Step 8: archiving. `27_archive_plan.{md,json}` are written as part of building
    // the final archive(s), and the final archive itself must exist and contain the
    // manifest — a real, non-empty ZIP, not a stub.
    assert!(insights.join("27_archive_plan.md").is_file());
    assert!(insights.join("27_archive_plan.json").is_file());
    let primary = outcome
        .archive_result
        .primary_result()
        .expect("a successful run always produces a primary archive result");
    assert!(primary.exists());

    if !outcome.archive_result.split {
        let entries = zip_entry_names(primary);
        assert!(entries.iter().any(|name| name == "manifest.json"));
        assert!(entries.iter().any(|name| name == "INDEX.md"));
        // The default safe export mode excludes `.env`-shaped files: the secret in
        // this fixture's `.env` must never reach the archive.
        assert!(
            !entries.iter().any(|name| name.ends_with(".env")),
            "entries = {entries:?}"
        );
    }
}
