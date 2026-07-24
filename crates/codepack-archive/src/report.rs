//! `27_archive_plan.md`/`.json` — self-contained (no `codepack-reports` dependency),
//! ported from `archive_service.py::write_archive_plan_report`.

use std::path::Path;

use serde::Serialize;

use crate::error::{ArchiveError, Result};
use crate::plan::ArchivePlan;
use crate::timestamp::current_timestamp_utc;

/// Duplicated from `codepack_tokens::format_bytes` for the same reason as
/// `crate::timestamp`: this crate's scope boundary (`ROADMAP.md` S8) is
/// `codepack-core` only, so it cannot depend on `codepack-tokens` for a single
/// formatting helper used in this report's and `build::write_restore_files`'s text.
pub(crate) fn format_bytes(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let last_index = UNITS.len() - 1;
    let mut value = size as f64;
    for (index, unit) in UNITS.into_iter().enumerate() {
        if value < 1024.0 || index == last_index {
            if unit == "B" {
                return format!("{} {unit}", value as u64);
            }
            return format!("{value:.2} {unit}");
        }
        value /= 1024.0;
    }
    unreachable!("\"TB\" is the last unit and its branch always returns from the loop")
}

#[derive(Serialize)]
struct ArchivePlanPartJson {
    index: u32,
    group_hint: String,
    groups: Vec<String>,
    files: u32,
    estimated_bytes: u64,
    estimated_human: String,
}

#[derive(Serialize)]
struct ArchivePlanJson {
    generated_at: String,
    split_planned: bool,
    limit_bytes: u64,
    limit_human: String,
    target_bytes: u64,
    target_human: String,
    estimated_total_bytes: u64,
    estimated_total_human: String,
    files: u32,
    skipped_project_files: u32,
    parts: Vec<ArchivePlanPartJson>,
}

pub fn write_archive_plan_report(plan: &ArchivePlan, output_file: &Path) -> Result<()> {
    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent).map_err(|source| ArchiveError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let data = ArchivePlanJson {
        generated_at: current_timestamp_utc(),
        split_planned: plan.split,
        limit_bytes: plan.limit_bytes,
        limit_human: format_bytes(plan.limit_bytes),
        target_bytes: plan.target_bytes,
        target_human: format_bytes(plan.target_bytes),
        estimated_total_bytes: plan.estimated_bytes(),
        estimated_total_human: format_bytes(plan.estimated_bytes()),
        files: plan.file_count(),
        skipped_project_files: plan.skipped_project_files,
        parts: plan
            .parts
            .iter()
            .map(|part| ArchivePlanPartJson {
                index: part.index,
                group_hint: part.group_hint.clone(),
                groups: part.groups().into_iter().map(str::to_string).collect(),
                files: part.file_count(),
                estimated_bytes: part.estimated_bytes(),
                estimated_human: format_bytes(part.estimated_bytes()),
            })
            .collect(),
    };

    let json_path = output_file.with_extension("json");
    let json_text = serde_json::to_string_pretty(&data)?;
    std::fs::write(&json_path, json_text).map_err(|source| ArchiveError::Write {
        path: json_path.clone(),
        source,
    })?;

    let mut markdown = String::new();
    markdown.push_str("# Archive Plan\n\n");
    markdown.push_str(&format!("Generated: {}\n\n", data.generated_at));
    markdown.push_str(&format!("- Split planned: `{}`\n", data.split_planned));
    markdown.push_str(&format!("- Hard limit: {}\n", data.limit_human));
    markdown.push_str(&format!("- Planning target: {}\n", data.target_human));
    markdown.push_str(&format!(
        "- Estimated archive input size: {}\n",
        data.estimated_total_human
    ));
    markdown.push_str(&format!("- Files to archive: {}\n", data.files));
    markdown.push_str(&format!(
        "- Planned archive count: {}\n\n",
        data.parts.len()
    ));
    markdown.push_str("## Parts\n\n");
    for part in &data.parts {
        markdown.push_str(&format!(
            "- Part {:03} `{}` — {} files — {} — groups: {}\n",
            part.index,
            part.group_hint,
            part.files,
            part.estimated_human,
            part.groups.join(", ")
        ));
    }

    std::fs::write(output_file, markdown).map_err(|source| ArchiveError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::ArchiveEntry;
    use crate::plan::ArchivePartPlan;
    use std::path::PathBuf;

    #[test]
    fn writes_markdown_and_json_reports() {
        let temp = tempfile::tempdir().expect("tempdir");
        let output_file = temp.path().join("reports").join("27_archive_plan.md");

        let entries = vec![ArchiveEntry {
            path: PathBuf::from("a.rs"),
            arcname: PathBuf::from("a.rs"),
            size: 10,
            group: "42_backend_or_system_source",
        }];
        let plan = ArchivePlan {
            split: false,
            limit_bytes: 512 * 1024 * 1024,
            target_bytes: 500 * 1024 * 1024,
            skipped_project_files: 0,
            entries: entries.clone(),
            parts: vec![ArchivePartPlan {
                index: 1,
                group_hint: "single".to_string(),
                entries,
            }],
        };

        write_archive_plan_report(&plan, &output_file).expect("write report");

        let markdown = std::fs::read_to_string(&output_file).expect("read md");
        assert!(markdown.starts_with("# Archive Plan"));
        assert!(markdown.contains("Split planned: `false`"));

        let json_path = output_file.with_extension("json");
        let json_text = std::fs::read_to_string(&json_path).expect("read json");
        let parsed: serde_json::Value = serde_json::from_str(&json_text).expect("parse json");
        assert_eq!(parsed["split_planned"], false);
        assert_eq!(parsed["files"], 1);
    }
}
