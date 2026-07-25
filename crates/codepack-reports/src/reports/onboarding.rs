//! `ONBOARDING_GUIDE.md` (BLUEPRINT §B.9, stage S12): "where do I start reading this
//! project" — a numbered route through [`crate::reports::key_files::ranked_key_files`]'s
//! own ranking, the ranking `16_key_files_report.md` already publishes in full. No
//! legacy equivalent; this is new composition, not a new heuristic.
//!
//! **Deliberately does not re-derive setup instructions or the architecture map.**
//! `13_runbook.md` and `24_architecture_map.md` already own that logic; duplicating
//! either here would create two places that could quietly disagree about how to run
//! this project. This report links to them instead of re-deriving their content —
//! composition, not a second implementation of the same heuristic.

use std::path::Path;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::plugin::ReportJob;
use crate::profile;
use crate::reports::key_files::ranked_key_files;

pub const JOB: ReportJob = ReportJob {
    filename: "ONBOARDING_GUIDE.md",
    profiles: profile::ONBOARDING_GUIDE_MD,
    description: "A suggested reading order through the project, ranked by importance.",
    run: write_onboarding_guide,
};

/// How many files to route a new reader through. Wider than
/// [`crate::humanize::STARTING_POINTS_LIMIT`]'s dashboard-card list (this is the
/// standalone artifact whose whole job is a longer walk), narrower than
/// `16_key_files_report.md`'s own 80-entry cap — a route a person actually follows,
/// not the full ranked list.
const ROUTE_LIMIT: usize = 15;

fn write_onboarding_guide(ctx: &ReportContext<'_>, output_file: &Path) -> Result<(), ReportError> {
    let project_name = ctx
        .staging_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ranked = ranked_key_files(ctx);

    let mut out = String::new();
    out.push_str(&format!("# Onboarding Guide: {project_name}\n\n"));
    out.push_str(&format!("Generated: {}\n\n", ctx.plan.generated_at));
    out.push_str(
        "A suggested reading order, ranked by the same importance signals as \
         `16_key_files_report.md` (entrypoints, configuration, import fan-in, size, \
         architectural location). Start at the top; each step says why it matters.\n\n",
    );
    out.push_str(
        "Before reading code, see `13_runbook.md` for how to set this project up and \
         run it, and `24_architecture_map.md` for the full per-layer file map.\n\n",
    );

    if ranked.is_empty() {
        out.push_str("## Reading order\n\nNo files stood out as a clear starting point.\n");
    } else {
        out.push_str("## Reading order\n\n");
        for (step, (score, file, reasons)) in ranked.iter().take(ROUTE_LIMIT).enumerate() {
            out.push_str(&format!(
                "{}. `{}` (importance {score})\n",
                step + 1,
                file.relative_path
            ));
            if let Some(reason) = reasons.first() {
                out.push_str(&format!("   - {reason}\n"));
            }
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
    use crate::test_support::Fixture;

    #[test]
    fn the_reading_order_matches_ranked_key_files_order() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("main.py"), "print(1)\n").unwrap();
            std::fs::write(root.join("random.txt"), "x".repeat(20)).unwrap();
        });
        let ctx = fixture.context("full");
        let ranked = ranked_key_files(&ctx);
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_onboarding_guide(&ctx, &output_file).unwrap();
        let content = std::fs::read_to_string(&output_file).unwrap();

        // The first-ranked file must appear as step "1.", not merely appear anywhere —
        // this pins the derivation, not just the presence of the name.
        let (_, first_file, _) = &ranked[0];
        let marker = format!("1. `{}`", first_file.relative_path);
        assert!(
            content.contains(&marker),
            "expected {marker:?} as step 1, got:\n{content}"
        );
    }

    #[test]
    fn points_to_the_runbook_and_architecture_map_instead_of_repeating_them() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("main.py"), "print(1)\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_onboarding_guide(&ctx, &output_file).unwrap();
        let content = std::fs::read_to_string(&output_file).unwrap();

        assert!(content.contains("13_runbook.md"));
        assert!(content.contains("24_architecture_map.md"));
    }

    #[test]
    fn an_empty_project_degrades_honestly_rather_than_panicking() {
        let fixture = Fixture::new(|_root| {});
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_onboarding_guide(&ctx, &output_file).unwrap();
        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("No files stood out"));
    }
}
