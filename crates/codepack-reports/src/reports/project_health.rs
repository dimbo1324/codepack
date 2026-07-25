//! `22_project_health_report.md`, ported from legacy
//! `reports/insights/health_score.py::write_project_health_report`. Every starting
//! score, weight, and clamp below is legacy's exact formula, not an approximation.
//!
//! One deliberate, documented deviation: legacy's "AI Readiness" area checks whether
//! `(copied_root.parent / "PROJECT_PROFILE.json").exists()` — a check against its own
//! process's directory layout (the artifact sits next to the reports folder at that
//! point in its pipeline). [`crate::context::ReportContext`] carries no equivalent
//! "where do sibling artifacts land" concept, and this crate's own pipeline
//! unconditionally produces `PROJECT_PROFILE.json` on every run
//! ([`crate::project_profile::write_project_profile_json`]), so this port always
//! awards that `+8` rather than probing a filesystem location this crate has no model
//! of — the same signal legacy intends ("is machine-readable project context
//! available"), reproduced without the removed, no-longer-meaningful existence check.
//!
//! This report also uses [`codepack_tokens::estimate_tokens_fallback`] alongside the
//! existing [`codepack_tokens::format_bytes`] size line — a deliberate addition beyond
//! legacy (which has no token estimate anywhere), per this stage's task scope.

use std::path::Path;

use codepack_tokens::{estimate_tokens_fallback, format_bytes};

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::paths::{extension_key, file_name_of};
use crate::plugin::ReportJob;
use crate::profile;
use crate::reports::config::CONFIG_FILE_NAMES;
use crate::reports::layout::SOURCE_CODE_EXTENSIONS;

pub const JOB: ReportJob = ReportJob {
    filename: "22_project_health_report.md",
    profiles: profile::PROJECT_HEALTH_REPORT_MD,
    description: "Heuristic health score across architecture, security, maintainability, testing, docs, AI readiness, export safety.",
    run: write_project_health_report,
};

const LARGE_FILE_BYTES: u64 = 500_000;
const LOCKFILE_NAMES: &[&str] = &[
    "pnpm-lock.yaml",
    "package-lock.json",
    "yarn.lock",
    "poetry.lock",
    "uv.lock",
    "go.sum",
    "Cargo.lock",
];
const DOC_FILE_NAMES: &[&str] = &["readme.md", "license", "changelog.md"];
const DOC_EXTENSIONS: &[&str] = &["md", "rst", "adoc"];

fn clamp(value: i64) -> i64 {
    value.clamp(0, 100)
}

fn bar(score: i64) -> String {
    let filled = ((score as f64 / 10.0).round() as i64).clamp(0, 10) as usize;
    "█".repeat(filled) + &"░".repeat(10 - filled)
}

/// One scored area (Architecture, Security, ...): name, clamped 0-100 score, and the
/// heuristic signals that produced it.
pub(crate) struct AreaScore {
    pub name: &'static str,
    pub score: i64,
    pub reasons: Vec<String>,
}

pub(crate) struct Scores {
    pub areas: Vec<AreaScore>,
}

impl Scores {
    /// The report's own "Overall score" line: the mean of every area, rounded.
    /// Exposed so [`crate::reports::overview`] shows the identical number rather than
    /// re-deriving it — two "the score" values that could silently disagree would be
    /// worse than one computation reused twice.
    pub(crate) fn overall(&self) -> i64 {
        let total: i64 = self.areas.iter().map(|area| area.score).sum();
        (total as f64 / self.areas.len().max(1) as f64).round() as i64
    }
}

pub(crate) fn compute_scores(ctx: &ReportContext<'_>) -> Scores {
    let inventory = ctx.inventory;
    let stack = crate::context::detect_stack(&ctx.staging_root, inventory);

    let source_files: Vec<&crate::context::InventoryFile> = inventory
        .files
        .iter()
        .filter(|file| SOURCE_CODE_EXTENSIONS.contains(&file.extension.as_str()))
        .collect();
    let test_files: Vec<&crate::context::InventoryFile> = inventory
        .files
        .iter()
        .filter(|file| {
            file.relative_path.to_lowercase().contains("test")
                || file.relative_path.split('\\').any(|part| part == "tests")
        })
        .collect();
    let docs: Vec<&crate::context::InventoryFile> = inventory
        .files
        .iter()
        .filter(|file| {
            let name_lower = file_name_of(&file.relative_path).to_lowercase();
            DOC_FILE_NAMES.contains(&name_lower.as_str())
                || DOC_EXTENSIONS.contains(&extension_key(&file.relative_path).as_str())
        })
        .collect();
    let configs: Vec<&crate::context::InventoryFile> = inventory
        .files
        .iter()
        .filter(|file| CONFIG_FILE_NAMES.contains(&file_name_of(&file.relative_path)))
        .collect();
    let large_files: Vec<&crate::context::InventoryFile> = inventory
        .files
        .iter()
        .filter(|file| file.size >= LARGE_FILE_BYTES)
        .collect();
    let suspicious_names: Vec<&crate::context::InventoryFile> = inventory
        .files
        .iter()
        .filter(|file| {
            let name_lower = file_name_of(&file.relative_path).to_lowercase();
            name_lower.starts_with(".env")
                || name_lower.contains("secret")
                || name_lower.contains("credential")
        })
        .collect();
    let lockfiles: Vec<&crate::context::InventoryFile> = inventory
        .files
        .iter()
        .filter(|file| LOCKFILE_NAMES.contains(&file_name_of(&file.relative_path)))
        .collect();

    let mut areas: Vec<AreaScore> = Vec::new();

    let mut architecture = 62i64;
    let mut architecture_reasons = Vec::new();
    if inventory
        .files
        .iter()
        .any(|file| file.relative_path.split('\\').any(|part| part == "src"))
    {
        architecture += 12;
        architecture_reasons.push("src/ style source layout detected".to_string());
    }
    if inventory.files.iter().any(|file| {
        file.relative_path
            .split('\\')
            .any(|part| matches!(part, "services" | "domain" | "core" | "utils" | "reports"))
    }) {
        architecture += 10;
        architecture_reasons.push("separated service/core utility modules detected".to_string());
    }
    if !source_files.is_empty() && source_files.len() < inventory.files.len().max(1) {
        architecture += 6;
        architecture_reasons
            .push("source files are not mixed with every exported file".to_string());
    }
    let large_source_files = large_files
        .iter()
        .filter(|file| SOURCE_CODE_EXTENSIONS.contains(&file.extension.as_str()))
        .count();
    if large_source_files > 3 {
        architecture -= 8;
        architecture_reasons.push("several large source files need decomposition".to_string());
    }
    areas.push(AreaScore {
        name: "Architecture",
        score: architecture,
        reasons: architecture_reasons,
    });

    let mut security = 82i64;
    let mut security_reasons = Vec::new();
    if !suspicious_names.is_empty() {
        security -= (12 * suspicious_names.len() as i64).min(45);
        security_reasons.push(format!(
            "{} sensitive-looking filenames detected",
            suspicious_names.len()
        ));
    } else {
        security += 5;
        security_reasons.push("no obvious sensitive filenames in exported copy".to_string());
    }
    if !lockfiles.is_empty() {
        security += 3;
        security_reasons.push("dependency lockfile(s) detected".to_string());
    }
    areas.push(AreaScore {
        name: "Security",
        score: security,
        reasons: security_reasons,
    });

    let mut maintainability = 64i64;
    let mut maintainability_reasons = Vec::new();
    if !docs.is_empty() {
        maintainability += 10;
        maintainability_reasons.push("documentation files are present".to_string());
    }
    if !configs.is_empty() {
        maintainability += 8;
        maintainability_reasons.push("standard config files are present".to_string());
    }
    maintainability -= (large_files.len() as i64 * 2).min(18);
    if !large_files.is_empty() {
        maintainability_reasons.push(format!("{} files are relatively large", large_files.len()));
    }
    areas.push(AreaScore {
        name: "Maintainability",
        score: maintainability,
        reasons: maintainability_reasons,
    });

    let mut testing = 30 + (test_files.len() as i64 * 6).min(42);
    let mut testing_reasons = Vec::new();
    if !stack.testing.is_empty() {
        testing += 18;
        testing_reasons.push("testing framework detected".to_string());
    }
    if !test_files.is_empty() {
        testing_reasons.push(format!("{} test-like files detected", test_files.len()));
    } else {
        testing_reasons.push("no test-like files detected".to_string());
    }
    areas.push(AreaScore {
        name: "Testing",
        score: testing,
        reasons: testing_reasons,
    });

    let mut documentation = 30 + (docs.len() as i64 * 8).min(48);
    let mut documentation_reasons = Vec::new();
    if ctx.staging_root.join("README.md").exists() {
        documentation += 15;
        documentation_reasons.push("README.md exists".to_string());
    }
    if ctx.staging_root.join("docs").exists() {
        documentation += 7;
        documentation_reasons.push("docs/ folder exists".to_string());
    }
    if docs.is_empty() {
        documentation_reasons.push("documentation is minimal or absent".to_string());
    }
    areas.push(AreaScore {
        name: "Documentation",
        score: documentation,
        reasons: documentation_reasons,
    });

    let mut dep_hygiene = 62i64;
    let mut dep_hygiene_reasons = Vec::new();
    if !lockfiles.is_empty() {
        dep_hygiene += 18;
        dep_hygiene_reasons.push("lockfiles improve reproducibility".to_string());
    }
    if !stack.package_managers.is_empty() {
        dep_hygiene += 8;
        dep_hygiene_reasons.push("package manager detected".to_string());
    }
    if stack.package_managers.len() > 2 {
        dep_hygiene -= 8;
        dep_hygiene_reasons
            .push("multiple package managers may increase maintenance cost".to_string());
    }
    areas.push(AreaScore {
        name: "Dependency Hygiene",
        score: dep_hygiene,
        reasons: dep_hygiene_reasons,
    });

    let mut ai_readiness = 78i64;
    let mut ai_readiness_reasons = Vec::new();
    // See the module doc comment: this crate's pipeline always produces
    // `PROJECT_PROFILE.json`, unlike legacy's own conditional filesystem check.
    ai_readiness += 8;
    ai_readiness_reasons.push("PROJECT_PROFILE.json is available".to_string());
    if !source_files.is_empty() {
        ai_readiness += 5;
        ai_readiness_reasons.push("source files are included".to_string());
    }
    if !suspicious_names.is_empty() {
        ai_readiness -= 12;
        ai_readiness_reasons
            .push("sensitive-looking files reduce safe sharing readiness".to_string());
    }
    areas.push(AreaScore {
        name: "AI Readiness",
        score: ai_readiness,
        reasons: ai_readiness_reasons,
    });

    let mut export_safety = 92i64;
    let mut export_safety_reasons = Vec::new();
    if !suspicious_names.is_empty() {
        export_safety -= (10 * suspicious_names.len() as i64).min(30);
        export_safety_reasons.push("review sensitive-looking files before sharing".to_string());
    } else {
        export_safety_reasons.push("exported copy appears safe by filename heuristics".to_string());
    }
    areas.push(AreaScore {
        name: "Export Safety",
        score: export_safety,
        reasons: export_safety_reasons,
    });

    let areas: Vec<AreaScore> = areas
        .into_iter()
        .map(|area| AreaScore {
            name: area.name,
            score: clamp(area.score),
            reasons: area.reasons,
        })
        .collect();

    Scores { areas }
}

fn write_project_health_report(
    ctx: &ReportContext<'_>,
    output_file: &Path,
) -> Result<(), ReportError> {
    let scores = compute_scores(ctx);
    let overall = scores.overall();

    let mut out = String::new();
    out.push_str("# Project Health Report\n\n");
    out.push_str(&format!("Generated: {}\n\n", ctx.plan.generated_at));
    out.push_str(&format!("Overall score: **{overall}/100**\n\n"));
    out.push_str("| Area | Score | Signal |\n");
    out.push_str("|---|---:|---|\n");
    for area in &scores.areas {
        out.push_str(&format!(
            "| {} | {}/100 | `{}` |\n",
            area.name,
            area.score,
            bar(area.score)
        ));
    }

    out.push_str("\n## Why these scores\n\n");
    for area in &scores.areas {
        out.push_str(&format!("### {}: {}/100\n\n", area.name, area.score));
        if area.reasons.is_empty() {
            out.push_str("- No specific signal.\n");
        } else {
            for reason in &area.reasons {
                out.push_str(&format!("- {reason}\n"));
            }
        }
        out.push('\n');
    }

    out.push_str("## Raw signals\n\n");
    out.push_str(&format!("- Files: {}\n", ctx.inventory.files.len()));
    out.push_str(&format!(
        "- Total copied size: {} (~{} tokens)\n",
        format_bytes(ctx.inventory.total_size),
        estimate_tokens_fallback(ctx.inventory.total_size)
    ));
    out.push_str("\nThis is a heuristic triage score, not a formal audit.\n");

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
    fn computes_an_overall_score_and_per_area_table() {
        let fixture = Fixture::new(|root| {
            std::fs::create_dir_all(root.join("src").join("tests")).unwrap();
            std::fs::write(root.join("src").join("main.py"), "print(1)\n").unwrap();
            std::fs::write(
                root.join("src").join("tests").join("test_main.py"),
                "x = 1\n",
            )
            .unwrap();
            std::fs::write(root.join("README.md"), "# hi\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_project_health_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.starts_with("# Project Health Report"));
        assert!(content.contains("Overall score: **"));
        assert!(content.contains("| Architecture |"));
        assert!(content.contains("### Documentation:"));
        assert!(content.contains("README.md exists"));
        assert!(content.contains("tokens"));
    }

    #[test]
    fn penalizes_security_and_export_safety_for_sensitive_filenames() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join(".env"), "SECRET=1\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_project_health_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("sensitive-looking filenames detected"));
        assert!(content.contains("review sensitive-looking files before sharing"));
    }
}
