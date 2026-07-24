//! Archive planning: First-Fit-by-group bin packing (BLUEPRINT §A.8/§E.4, legacy
//! `archive_service.py::build_archive_plan`/`_plan_logical_parts`).

use std::collections::BTreeMap;

use codepack_core::{ArchiveBuildResult, CancellationToken, ExportPaths};

use crate::entry::{ArchiveEntry, collect_entries};
use crate::error::Result;

/// Target part size (BLUEPRINT §A.8): 500 MB.
const ARCHIVE_PART_TARGET_BYTES: u64 = 500 * 1024 * 1024;
/// Headroom subtracted from the hard limit to absorb ZIP container/compression
/// overhead before planning (BLUEPRINT §A.8): 8 MB.
const RESERVE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ArchivePartPlan {
    pub index: u32,
    pub group_hint: String,
    pub entries: Vec<ArchiveEntry>,
}

impl ArchivePartPlan {
    pub fn estimated_bytes(&self) -> u64 {
        self.entries.iter().map(|entry| entry.size).sum()
    }

    pub fn file_count(&self) -> u32 {
        self.entries.len() as u32
    }

    pub fn groups(&self) -> Vec<&'static str> {
        let mut groups: Vec<&'static str> = self.entries.iter().map(|entry| entry.group).collect();
        groups.sort_unstable();
        groups.dedup();
        groups
    }
}

#[derive(Debug, Clone)]
pub struct ArchivePlan {
    pub split: bool,
    pub limit_bytes: u64,
    pub target_bytes: u64,
    pub skipped_project_files: u32,
    pub entries: Vec<ArchiveEntry>,
    pub parts: Vec<ArchivePartPlan>,
}

impl ArchivePlan {
    pub fn estimated_bytes(&self) -> u64 {
        self.entries.iter().map(|entry| entry.size).sum()
    }

    pub fn file_count(&self) -> u32 {
        self.entries.len() as u32
    }
}

fn target_bytes_for(part_limit_bytes: u64) -> (u64, u64) {
    let limit_bytes = part_limit_bytes.max(1);
    let target_bytes =
        ARCHIVE_PART_TARGET_BYTES.min(limit_bytes.saturating_sub(RESERVE_BYTES).max(1));
    (limit_bytes, target_bytes)
}

/// First-Fit-by-group: entries are already sorted by `(group, arcname.casefold())`
/// (`plan_archive` does this once after collection); parts are flushed whenever the
/// group changes, whenever the next entry would exceed `target_bytes`, or immediately
/// in isolation when a single entry's size already reaches `target_bytes`.
pub(crate) fn plan_logical_parts(
    entries: &[ArchiveEntry],
    target_bytes: u64,
) -> Vec<ArchivePartPlan> {
    let mut grouped: BTreeMap<&'static str, Vec<ArchiveEntry>> = BTreeMap::new();
    for entry in entries {
        grouped.entry(entry.group).or_default().push(entry.clone());
    }

    let mut parts: Vec<ArchivePartPlan> = Vec::new();
    let mut current_entries: Vec<ArchiveEntry> = Vec::new();
    let mut current_size: u64 = 0;
    let mut current_group: Option<&'static str> = None;

    for (_group, group_entries) in grouped {
        for entry in group_entries {
            let entry_size = entry.size;
            let entry_group = entry.group;
            if !current_entries.is_empty()
                && (current_group != Some(entry_group) || current_size + entry_size > target_bytes)
            {
                flush_part(&mut parts, &mut current_entries);
                current_size = 0;
            }
            current_entries.push(entry);
            current_size += entry_size;
            current_group = Some(entry_group);
            if entry_size >= target_bytes {
                flush_part(&mut parts, &mut current_entries);
                current_size = 0;
                current_group = None;
            }
        }
    }
    flush_part(&mut parts, &mut current_entries);
    parts
}

fn flush_part(parts: &mut Vec<ArchivePartPlan>, current_entries: &mut Vec<ArchiveEntry>) {
    if current_entries.is_empty() {
        return;
    }
    let mut groups: Vec<&'static str> = current_entries.iter().map(|entry| entry.group).collect();
    groups.sort_unstable();
    groups.dedup();
    let group_hint = groups.first().copied().unwrap_or("empty").to_string();
    parts.push(ArchivePartPlan {
        index: parts.len() as u32 + 1,
        group_hint,
        entries: std::mem::take(current_entries),
    });
}

fn sort_entries(entries: &mut [ArchiveEntry]) {
    entries.sort_by(|a, b| {
        let a_key = (a.group, a.arcname.to_string_lossy().to_lowercase());
        let b_key = (b.group, b.arcname.to_string_lossy().to_lowercase());
        a_key.cmp(&b_key)
    });
}

/// Walks the staging tree via [`collect_entries`] and produces an [`ArchivePlan`].
/// Mirrors legacy's `build_archive_plan`, which is called multiple times across a
/// single build (each call re-walks, deliberately picking up files written between
/// calls — see `build::build_final_archives`).
pub fn plan_archive(
    paths: &ExportPaths,
    include_project: bool,
    part_limit_bytes: u64,
    cancel: &CancellationToken,
) -> Result<ArchivePlan> {
    let (limit_bytes, target_bytes) = target_bytes_for(part_limit_bytes);
    let (mut entries, skipped_project_files) = collect_entries(paths, include_project, cancel)?;
    sort_entries(&mut entries);

    let estimated_total: u64 = entries.iter().map(|entry| entry.size).sum();
    if estimated_total <= target_bytes {
        let part = ArchivePartPlan {
            index: 1,
            group_hint: "single".to_string(),
            entries: entries.clone(),
        };
        return Ok(ArchivePlan {
            split: false,
            limit_bytes,
            target_bytes,
            skipped_project_files,
            entries,
            parts: vec![part],
        });
    }

    let parts = plan_logical_parts(&entries, target_bytes);
    Ok(ArchivePlan {
        split: true,
        limit_bytes,
        target_bytes,
        skipped_project_files,
        entries,
        parts,
    })
}

/// The [`ArchiveBuildResult`] a plan predicts, before any bytes are actually written —
/// used to feed `on_plan_ready` ahead of the real write (legacy
/// `_predicted_result_for_plan`).
pub fn predicted_result_for_plan(paths: &ExportPaths, plan: &ArchivePlan) -> ArchiveBuildResult {
    if !plan.split {
        return ArchiveBuildResult {
            archives: vec![paths.final_zip.clone()],
            output_dir: None,
            split: false,
            file_count: plan.file_count(),
            skipped_project_files: plan.skipped_project_files,
            oversized_files: Vec::new(),
        };
    }
    let archives = plan
        .parts
        .iter()
        .map(|part| {
            paths.archive_set_dir.join(format!(
                "{}_part_{:03}_{}.zip",
                paths.bundle_name, part.index, part.group_hint
            ))
        })
        .collect();
    ArchiveBuildResult {
        archives,
        output_dir: Some(paths.archive_set_dir.clone()),
        split: true,
        file_count: plan.file_count(),
        skipped_project_files: plan.skipped_project_files,
        oversized_files: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn entry(arcname: &str, size: u64, group: &'static str) -> ArchiveEntry {
        ArchiveEntry {
            path: PathBuf::from(arcname),
            arcname: PathBuf::from(arcname),
            size,
            group,
        }
    }

    #[test]
    fn same_group_entries_bundle_into_one_part_while_under_target() {
        let entries = vec![
            entry("a.rs", 10, "42_backend_or_system_source"),
            entry("b.rs", 10, "42_backend_or_system_source"),
        ];
        let parts = plan_logical_parts(&entries, 1000);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].entries.len(), 2);
    }

    #[test]
    fn group_switch_forces_a_flush_even_under_target() {
        let entries = vec![
            entry("a.rs", 10, "42_backend_or_system_source"),
            entry("readme.md", 10, "30_docs"),
        ];
        let parts = plan_logical_parts(&entries, 1000);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].group_hint, "30_docs");
        assert_eq!(parts[1].group_hint, "42_backend_or_system_source");
    }

    #[test]
    fn oversized_single_entry_gets_its_own_isolated_part() {
        let entries = vec![
            entry("a.rs", 10, "42_backend_or_system_source"),
            entry("huge.bin", 1000, "60_assets_and_binary_docs"),
            entry("b.rs", 10, "42_backend_or_system_source"),
        ];
        let parts = plan_logical_parts(&entries, 100);
        assert_eq!(parts.len(), 2);
        let huge_part = parts
            .iter()
            .find(|part| {
                part.entries
                    .iter()
                    .any(|e| e.arcname == Path::new("huge.bin"))
            })
            .expect("huge part");
        assert_eq!(huge_part.entries.len(), 1);
    }

    #[test]
    fn part_indices_are_sequential() {
        let entries = vec![
            entry("a.rs", 60, "42_backend_or_system_source"),
            entry("b.rs", 60, "42_backend_or_system_source"),
            entry("c.rs", 60, "42_backend_or_system_source"),
        ];
        let parts = plan_logical_parts(&entries, 100);
        let indices: Vec<u32> = parts.iter().map(|p| p.index).collect();
        assert_eq!(indices, (1..=indices.len() as u32).collect::<Vec<_>>());
    }

    #[test]
    fn small_total_under_target_produces_no_split() {
        let temp = tempfile::tempdir().expect("tempdir");
        let staging = temp.path().join("staging");
        std::fs::create_dir_all(&staging).expect("mkdir");
        std::fs::write(staging.join("a.txt"), b"hello").expect("write");

        let paths = ExportPaths {
            desktop: temp.path().to_path_buf(),
            source_root: temp.path().to_path_buf(),
            project_name: "demo".to_string(),
            bundle_name: "demo_export".to_string(),
            staging_dir: staging.clone(),
            final_zip: temp.path().join("out.zip"),
            archive_set_dir: temp.path().join("archive_set"),
            project_dir: staging.join("demo"),
            reports_dir: staging.join("reports"),
            insights_dir: staging.join("reports"),
            manifest_file: staging.join("manifest.json"),
            project_profile_file: staging.join("PROJECT_PROFILE.json"),
            index_file: staging.join("INDEX.md"),
            structure_report: staging.join("structure.md"),
            git_report: staging.join("git.md"),
            text_dump: staging.join("dump.txt"),
        };
        let cancel = CancellationToken::new();
        let plan = plan_archive(&paths, true, 512 * 1024 * 1024, &cancel).expect("plan");
        assert!(!plan.split);
        assert_eq!(plan.parts.len(), 1);
    }
}
