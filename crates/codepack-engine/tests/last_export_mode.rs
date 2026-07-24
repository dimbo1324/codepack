//! `last_export` mode round-trip (`task-checklist.md`'s S9 Verification section):
//! export once with no prior baseline (behaves like `"all"` mode, with
//! `codepack_diff::diff_against_snapshot`'s own documented "no previous snapshot"
//! warning — BLUEPRINT/`codepack-diff` reference), edit exactly one file, export again
//! against the same `conn`/`source_root`, and confirm only the edited file is selected.
//!
//! `ExportOutcome` does not expose `PlanOutcome`/`DiffSelection` directly (a deliberate,
//! minimal public surface — see `orchestrator.rs`'s own struct doc comment), so this
//! test asserts indirectly by reading the second run's own on-disk artifacts: the
//! copied project directory (which file actually reached the copy) and
//! `29_export_comparison_report.md` (which file the diff step itself reports as
//! selected), exactly as this pass's own instructions prefer over widening
//! `ExportOutcome`'s public shape for a single test's convenience.

use std::fs;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_storage::open;

fn last_export_config() -> Config {
    Config {
        diff_export_mode: "last_export".to_string(),
        keep_staging_folder: true,
        ..Config::default()
    }
}

#[test]
fn editing_one_file_between_two_last_export_runs_selects_only_that_file() {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("a.py"), "print('a v1')\n").unwrap();
    fs::write(source.path().join("b.py"), "print('b')\n").unwrap();
    fs::write(source.path().join("c.md"), "# c\n").unwrap();

    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut conn = open(&db_dir.path().join("codepack.db")).unwrap();
    let config = last_export_config();
    let (tx, _rx) = codepack_core::progress_channel();

    let first = codepack_engine::run_export(
        &mut conn,
        source.path(),
        output.path(),
        &config,
        &std::collections::HashMap::new(),
        &tx,
        &CancellationToken::new(),
    )
    .unwrap();

    assert!(
        first.successful,
        "the first last_export run has no prior baseline and must behave like an \
         all-mode run: copy_stats = {:?}",
        first.copy_stats
    );
    assert!(first.project_dir_has("a.py"));
    assert!(first.project_dir_has("b.py"));
    assert!(first.project_dir_has("c.md"));

    let first_report = fs::read_to_string(
        first
            .paths
            .insights_dir
            .join("29_export_comparison_report.md"),
    )
    .unwrap();
    assert!(
        first_report.contains("Предупреждение:"),
        "the first run must carry the documented \"no previous snapshot\" warning"
    );

    fs::write(source.path().join("a.py"), "print('a v2 — edited')\n").unwrap();

    let second = codepack_engine::run_export(
        &mut conn,
        source.path(),
        output.path(),
        &config,
        &std::collections::HashMap::new(),
        &tx,
        &CancellationToken::new(),
    )
    .unwrap();

    assert!(second.successful, "copy_stats = {:?}", second.copy_stats);

    // The copy step is the ground truth for "what actually got exported": only the
    // edited file should have reached the second run's own project directory.
    assert!(second.project_dir_has("a.py"));
    assert!(!second.project_dir_has("b.py"));
    assert!(!second.project_dir_has("c.md"));
    assert_eq!(
        fs::read_to_string(second.paths.project_dir.join("a.py")).unwrap(),
        "print('a v2 — edited')\n"
    );

    // `29_export_comparison_report.md` is the diff step's own artifact naming exactly
    // which files it selected — the second, independent confirmation this criterion
    // asks for, read from disk rather than through a private field.
    let second_report = fs::read_to_string(
        second
            .paths
            .insights_dir
            .join("29_export_comparison_report.md"),
    )
    .unwrap();
    assert!(
        !second_report.contains("Предупреждение:"),
        "a real baseline now exists; the no-previous-snapshot warning must be gone"
    );
    assert!(second_report.contains("## Изменённые"));
    let modified_section = second_report
        .split("## Изменённые")
        .nth(1)
        .and_then(|rest| rest.split("## ").next())
        .unwrap();
    assert!(modified_section.contains("`a.py`"));
    assert!(!modified_section.contains("`b.py`"));
    assert!(!modified_section.contains("`c.md`"));

    // `28_export_plan.json`/`.md` describe the underlying scanner plan (every file the
    // ignore rules would include), not the diff-narrowed selection — they are not the
    // right artifact to assert "only one file selected" against, and this test does
    // not claim otherwise. They are still checked for the `diff_export_mode` field the
    // task's own acceptance criterion cares about being recorded correctly.
    let plan_md = fs::read_to_string(second.paths.insights_dir.join("28_export_plan.md")).unwrap();
    assert!(plan_md.contains("Diff mode: `last_export`"));
}

trait ProjectDirHas {
    fn project_dir_has(&self, relative: &str) -> bool;
}

impl ProjectDirHas for codepack_engine::ExportOutcome {
    fn project_dir_has(&self, relative: &str) -> bool {
        self.paths.project_dir.join(relative).is_file()
    }
}
