//! `write_export_plan_files()`: JSON (via `ExportPlan`'s serde field order) and
//! Markdown renderers, ported from legacy `write_export_plan_files`.
//!
//! Every size in the Markdown output is rendered as a raw byte count. Legacy used
//! `format_bytes()`; `codepack-core` has no byte-formatter yet (S6), and this stage
//! deliberately does not invent one — see `plan/mod.rs`'s `PlanSummary` doc comment.

use std::fs;
use std::path::Path;

use crate::error::{Result, ScannerError};

use super::ExportPlan;

pub fn write_export_plan_files(plan: &ExportPlan, json_path: &Path, md_path: &Path) -> Result<()> {
    if let Some(parent) = json_path.parent() {
        fs::create_dir_all(parent).map_err(|source| ScannerError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    if let Some(parent) = md_path.parent() {
        fs::create_dir_all(parent).map_err(|source| ScannerError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let json = serde_json::to_string_pretty(plan).map_err(|source| ScannerError::InvalidJson {
        field: "ExportPlan",
        source,
    })?;
    fs::write(json_path, json).map_err(|source| ScannerError::Write {
        path: json_path.to_path_buf(),
        source,
    })?;

    let markdown = render_markdown(plan);
    fs::write(md_path, markdown).map_err(|source| ScannerError::Write {
        path: md_path.to_path_buf(),
        source,
    })?;

    Ok(())
}

fn render_markdown(plan: &ExportPlan) -> String {
    let mut out = String::new();
    out.push_str("# Export Plan\n\n");
    out.push_str(&format!("Generated: {}\n\n", plan.generated_at));
    out.push_str(&format!("- Project: `{}`\n", plan.project_name));
    out.push_str(&format!("- Profile: `{}`\n", plan.profile));
    out.push_str(&format!("- Safe mode: `{}`\n", plan.safe_export_mode));
    out.push_str(&format!("- Diff mode: `{}`\n", plan.diff_export_mode));
    out.push_str(&format!("- Incremental: `{}`\n", plan.incremental_enabled));
    out.push_str(&format!(
        "- Included files: {}\n",
        format_count(plan.summary.included_count as u64)
    ));
    out.push_str(&format!(
        "- Excluded files: {}\n",
        format_count(plan.summary.excluded_count as u64)
    ));
    out.push_str(&format!(
        "- Skipped directories: {}\n",
        format_count(plan.skipped_dirs.len() as u64)
    ));
    out.push_str(&format!(
        "- Estimated selected project size: {} bytes\n\n",
        format_count(plan.summary.estimated_included_bytes)
    ));

    if !plan.warnings.is_empty() {
        out.push_str("## Warnings\n\n");
        for warning in &plan.warnings {
            out.push_str(&format!("- {warning}\n"));
        }
        out.push('\n');
    }

    out.push_str("## Included files by group\n\n");
    for (group, count) in group_counts_most_common_first(plan) {
        out.push_str(&format!("- {group}: {}\n", format_count(count)));
    }

    out.push_str("\n## Sensitive / high-risk excluded files\n\n");
    let sensitive = plan.sensitive_warnings();
    if sensitive.is_empty() {
        out.push_str("- none\n");
    } else {
        for item in sensitive.into_iter().take(200) {
            out.push_str(&format!(
                "- [{}] `{}` — {} ({} bytes)\n",
                item.severity, item.relative_path, item.reason, item.size
            ));
        }
    }

    out.push_str("\n## Large files\n\n");
    let mut large = plan.large_files();
    large.sort_by_key(|item| std::cmp::Reverse(item.size));
    if large.is_empty() {
        out.push_str("- none >= 100 MB\n");
    } else {
        for item in large.into_iter().take(100) {
            let reason = if item.reason.is_empty() {
                item.group.as_str()
            } else {
                item.reason.as_str()
            };
            out.push_str(&format!(
                "- `{}` — {} bytes — {} — {}\n",
                item.relative_path, item.size, item.status, reason
            ));
        }
    }

    out.push_str("\n## First included files\n\n");
    for item in plan.included_files.iter().take(300) {
        out.push_str(&format!(
            "- `{}` ({} bytes)\n",
            item.relative_path, item.size
        ));
    }
    if plan.included_files.len() > 300 {
        out.push_str(&format!(
            "- ... and {} more\n",
            format_count((plan.included_files.len() - 300) as u64)
        ));
    }

    out
}

/// Legacy `Counter.most_common()`: stable-sorted by count descending, ties broken by
/// first-seen order (matching CPython's stable sort over insertion-ordered items).
fn group_counts_most_common_first(plan: &ExportPlan) -> Vec<(String, u64)> {
    let mut order: Vec<String> = Vec::new();
    let mut counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for item in &plan.included_files {
        if !counts.contains_key(&item.group) {
            order.push(item.group.clone());
        }
        *counts.entry(item.group.clone()).or_insert(0) += 1;
    }
    let mut pairs: Vec<(String, u64)> = order
        .into_iter()
        .map(|group| (group.clone(), counts[&group]))
        .collect();
    pairs.sort_by_key(|pair| std::cmp::Reverse(pair.1));
    pairs
}

fn format_count(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ignore::{ExportIgnoreRules, ScanOptions};
    use crate::plan::build_export_plan;
    use codepack_core::CancellationToken;

    #[test]
    fn format_count_inserts_thousands_separators() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1000), "1,000");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }

    #[test]
    fn write_export_plan_files_produces_readable_json_and_markdown() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.py"), "print(1)").unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &crate::plan::no_safety_classification,
            &CancellationToken::new(),
        )
        .unwrap();

        let json_path = dir.path().join("out/plan.json");
        let md_path = dir.path().join("out/plan.md");
        write_export_plan_files(&plan, &json_path, &md_path).unwrap();

        let json = std::fs::read_to_string(&json_path).unwrap();
        assert!(json.contains("\"main.py\""));
        let markdown = std::fs::read_to_string(&md_path).unwrap();
        assert!(markdown.starts_with("# Export Plan"));
        assert!(markdown.contains("## Included files by group"));
        assert!(markdown.contains("main.py"));
    }
}
