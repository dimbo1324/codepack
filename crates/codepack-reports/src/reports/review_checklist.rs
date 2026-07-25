//! `REVIEW_CHECKLIST.md` (BLUEPRINT §B.9, stage S12): a reviewer checklist plus a
//! diff-focused, file-level change summary. No legacy equivalent.
//!
//! **File-level links only, not `file:line`.** BLUEPRINT §B.9 asks for
//! `file:line`-anchored review mode; `codepack_diff::DiffFile` carries only
//! `relative_path`/`status`/`old_path` — no hunk or line-number data exists anywhere
//! in `codepack-diff` (confirmed by inspection during S12 planning). Adding that is a
//! new capability in a different crate (`git2` patch/hunk extraction), not a gap this
//! report can close on its own; recorded as an explicit open question rather than
//! silently narrowed.
//!
//! Degrades honestly when no diff is active (`DiffSelection::is_limited() == false`,
//! e.g. the default `all` mode): the checklist still prints — it is useful for a full
//! review too — but the change summary says so plainly instead of printing an empty
//! table, the same disclosed-degradation pattern [`crate::reports::dashboard`] already
//! established for missing optional siblings.

use std::path::Path;

use codepack_diff::FileStatus;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::plugin::ReportJob;
use crate::profile;

pub const JOB: ReportJob = ReportJob {
    filename: "REVIEW_CHECKLIST.md",
    profiles: profile::REVIEW_CHECKLIST_MD,
    description: "Reviewer checklist and a file-level summary of what changed.",
    run: write_review_checklist,
};

const CHECKLIST_ITEMS: &[&str] = &[
    "Read `PROJECT_OVERVIEW.html` first if you are new to this project.",
    "Check `06_security_scan.txt` — any new `critical`/`high` findings on changed files?",
    "Check `22_project_health_report.md` for areas the change might affect.",
    "For each added/modified file below, confirm it belongs in this change.",
    "For each deleted file below, confirm nothing else still depends on it.",
    "Confirm no secrets, credentials, or `.env`-like files were added.",
];

fn write_review_checklist(ctx: &ReportContext<'_>, output_file: &Path) -> Result<(), ReportError> {
    let mut out = String::new();
    out.push_str("# Review Checklist\n\n");
    out.push_str(&format!("Generated: {}\n\n", ctx.plan.generated_at));

    out.push_str("## Checklist\n\n");
    for item in CHECKLIST_ITEMS {
        out.push_str(&format!("- [ ] {item}\n"));
    }

    out.push_str("\n## What changed\n\n");
    match ctx.diff {
        Some(selection) if selection.is_limited() && !selection.files.is_empty() => {
            out.push_str(&format!(
                "Diff mode: `{}` against `{}`. {} file(s) changed.\n\n",
                selection.mode,
                selection.base,
                selection.files.len()
            ));
            out.push_str("| File | Change | Notes |\n");
            out.push_str("|---|---|---|\n");
            let mut files = selection.files.clone();
            files.sort_by(|a, b| {
                a.relative_path
                    .to_lowercase()
                    .cmp(&b.relative_path.to_lowercase())
            });
            for file in &files {
                let notes = match (file.status, &file.old_path) {
                    (FileStatus::Renamed, Some(old)) => format!("renamed from `{old}`"),
                    _ => String::new(),
                };
                out.push_str(&format!(
                    "| `{}` | {} | {notes} |\n",
                    file.relative_path,
                    file.status.as_str()
                ));
            }
            out.push_str(
                "\n_File-level only: line-level anchors are not yet available (open \
                 question, `codepack-diff` carries no hunk data)._\n",
            );
        }
        Some(selection) if selection.is_limited() => {
            out.push_str(&format!(
                "Diff mode: `{}` — no files matched.\n",
                selection.mode
            ));
        }
        _ => {
            out.push_str(
                "This export is not scoped to a diff (mode `all`, or diff mode not \
                 configured), so there is no change list — the checklist above still \
                 applies to a full review.\n",
            );
        }
    }

    std::fs::write(output_file, out).map_err(|source| ReportError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ReportContext;
    use crate::test_support::Fixture;
    use codepack_diff::{DiffFile, DiffSelection, FileStatus};
    use std::collections::HashSet;

    #[test]
    fn no_diff_active_prints_the_checklist_and_an_honest_fallback() {
        let fixture = Fixture::new(|_root| {});
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_review_checklist(&ctx, &output_file).unwrap();
        let content = std::fs::read_to_string(&output_file).unwrap();

        assert!(content.contains("## Checklist"));
        assert!(content.contains("is not scoped to a diff"));
        assert!(
            !content.contains('|'),
            "no table should render with no diff active"
        );
    }

    #[test]
    fn a_limited_diff_renders_a_file_level_table() {
        let fixture = Fixture::new(|_root| {});
        let base_ctx = fixture.context("full");
        let selection = DiffSelection {
            mode: "uncommitted".to_string(),
            base: "working tree".to_string(),
            paths: Some(HashSet::from(["src\\app.py".to_string()])),
            files: vec![
                DiffFile {
                    relative_path: "src\\app.py".to_string(),
                    status: FileStatus::Modified,
                    old_path: None,
                },
                DiffFile {
                    relative_path: "src\\new_name.py".to_string(),
                    status: FileStatus::Renamed,
                    old_path: Some("src\\old_name.py".to_string()),
                },
            ],
            warning: None,
        };
        let ctx = ReportContext {
            diff: Some(&selection),
            ..base_ctx
        };
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_review_checklist(&ctx, &output_file).unwrap();
        let content = std::fs::read_to_string(&output_file).unwrap();

        assert!(content.contains("2 file(s) changed"));
        assert!(content.contains("`src\\app.py` | modified"));
        assert!(content.contains("renamed from `src\\old_name.py`"));
        assert!(content.contains("line-level anchors are not yet available"));
    }
}
