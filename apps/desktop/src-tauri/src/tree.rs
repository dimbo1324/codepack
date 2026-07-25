//! Turning a flat export plan into the tree the Preview page renders.
//!
//! `ExportPlan` is a list of files with a status each; the preview needs a directory
//! hierarchy where a collapsed folder still tells the user something is wrong inside it.
//! This module is the only place that conversion happens, and it is pure — given a plan,
//! it touches no filesystem.
//!
//! ## Why directories aggregate the way they do
//!
//! A folder's status answers "should I open this?". `warning` wins over everything,
//! because a single sensitive file is the thing the user must not miss. Otherwise a
//! folder is `included` if anything inside it survives, and `excluded` only when nothing
//! does — a folder that is half excluded is still contributing to the bundle, and
//! showing it as excluded would be a lie.

use std::collections::BTreeMap;

use codepack_scanner::{ExportPlan, PlannedFile};

use crate::dto::TreeNode;

/// The three statuses the UI knows about, matching `FileStatus` in `types.ts`.
const STATUS_INCLUDED: &str = "included";
const STATUS_EXCLUDED: &str = "excluded";
const STATUS_WARNING: &str = "warning";

/// Severities that make an exclusion a *warning* rather than routine housekeeping.
///
/// The same two `ExportPlan::sensitive_warnings` selects on: `node_modules` being
/// excluded is expected and uninteresting, `.env` being excluded is the entire reason
/// someone opens the preview.
fn is_warning_severity(severity: &str) -> bool {
    matches!(severity, "critical" | "high")
}

fn status_for(file: &PlannedFile) -> &'static str {
    if file.status == "included" {
        STATUS_INCLUDED
    } else if is_warning_severity(&file.severity) {
        STATUS_WARNING
    } else {
        STATUS_EXCLUDED
    }
}

/// A directory being assembled, before it becomes a [`TreeNode`].
#[derive(Default)]
struct Directory {
    directories: BTreeMap<String, Directory>,
    files: Vec<TreeNode>,
}

impl Directory {
    fn insert(&mut self, segments: &[&str], leaf: TreeNode) {
        match segments {
            [] => {}
            [_last] => self.files.push(leaf),
            [head, rest @ ..] => self
                .directories
                .entry((*head).to_string())
                .or_default()
                .insert(rest, leaf),
        }
    }

    /// Converts to a [`TreeNode`], computing this directory's aggregated status from
    /// whatever ended up inside it.
    fn into_node(self, name: String, path: String) -> TreeNode {
        let mut children: Vec<TreeNode> = self
            .directories
            .into_iter()
            .map(|(child_name, child)| {
                let child_path = if path.is_empty() {
                    child_name.clone()
                } else {
                    format!("{path}\\{child_name}")
                };
                child.into_node(child_name, child_path)
            })
            .collect();

        let mut files = self.files;
        // Directories first, then files, each alphabetically — the ordering a file
        // manager uses, so the tree reads the way the user expects.
        files.sort_by_key(|file| file.name.to_lowercase());
        children.extend(files);

        let status = aggregate_status(&children);
        TreeNode {
            name,
            path,
            is_dir: true,
            status,
            reason: None,
            severity: None,
            size: None,
            children: Some(children),
        }
    }
}

/// See the module doc: warning wins, then included, and excluded only when nothing
/// inside survived. An empty directory reports `included` — it contributes nothing, but
/// calling it excluded would suggest something was dropped.
fn aggregate_status(children: &[TreeNode]) -> String {
    if children.iter().any(|child| child.status == STATUS_WARNING) {
        return STATUS_WARNING.to_string();
    }
    if children.is_empty() || children.iter().any(|child| child.status == STATUS_INCLUDED) {
        return STATUS_INCLUDED.to_string();
    }
    STATUS_EXCLUDED.to_string()
}

/// Builds the preview tree from every file the plan considered — included *and*
/// excluded.
///
/// Both lists are needed: a preview that showed only what survives could not tell the
/// user that their `.env` was caught, which is the feature.
pub fn build(plan: &ExportPlan, root_name: &str) -> TreeNode {
    let mut root = Directory::default();

    for file in plan.included_files.iter().chain(plan.excluded_files.iter()) {
        // `relative_path` is backslash-joined on every platform (this project's display
        // convention), so splitting on `\` is correct here and needs no OS branch.
        let segments: Vec<&str> = file.relative_path.split('\\').collect();
        let Some(name) = segments.last() else {
            continue;
        };

        let leaf = TreeNode {
            name: (*name).to_string(),
            path: file.relative_path.clone(),
            is_dir: false,
            status: status_for(file).to_string(),
            reason: (!file.reason.is_empty()).then(|| file.reason.clone()),
            severity: (file.status != "included").then(|| file.severity.clone()),
            size: Some(file.size),
            children: None,
        };
        root.insert(&segments, leaf);
    }

    root.into_node(root_name.to_string(), String::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_scanner::{ExportIgnoreRules, ScanOptions, build_export_plan};

    fn planned(relative_path: &str, status: &str, severity: &str, reason: &str) -> PlannedFile {
        PlannedFile {
            relative_path: relative_path.to_string(),
            size: 10,
            status: status.to_string(),
            reason: reason.to_string(),
            severity: severity.to_string(),
            group: "other".to_string(),
        }
    }

    fn plan_with(included: Vec<PlannedFile>, excluded: Vec<PlannedFile>) -> ExportPlan {
        let dir = tempfile::tempdir().unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let mut plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &codepack_scanner::plan::no_safety_classification,
            &codepack_core::CancellationToken::new(),
        )
        .unwrap();
        plan.included_files = included;
        plan.excluded_files = excluded;
        plan
    }

    fn child<'a>(node: &'a TreeNode, name: &str) -> &'a TreeNode {
        node.children
            .as_ref()
            .expect("node has children")
            .iter()
            .find(|child| child.name == name)
            .unwrap_or_else(|| panic!("no child named {name}"))
    }

    #[test]
    fn nested_paths_become_a_real_hierarchy() {
        let plan = plan_with(
            vec![
                planned("src\\main.rs", "included", "info", ""),
                planned("src\\lib\\helper.rs", "included", "info", ""),
                planned("README.md", "included", "info", ""),
            ],
            Vec::new(),
        );

        let tree = build(&plan, "demo");
        assert!(tree.is_dir);
        assert_eq!(tree.name, "demo");

        let src = child(&tree, "src");
        assert!(src.is_dir);
        assert_eq!(src.path, "src");

        let helper = child(child(src, "lib"), "helper.rs");
        assert!(!helper.is_dir);
        assert_eq!(helper.path, "src\\lib\\helper.rs");
        assert_eq!(helper.size, Some(10));
    }

    #[test]
    fn a_sensitive_exclusion_marks_every_ancestor_as_a_warning() {
        // The whole point of the preview: a collapsed folder must still show that
        // something inside it needs looking at.
        let plan = plan_with(
            vec![planned("src\\main.rs", "included", "info", "")],
            vec![planned(
                "config\\secrets\\.env",
                "excluded",
                "critical",
                "high-risk credential filename",
            )],
        );

        let tree = build(&plan, "demo");
        assert_eq!(tree.status, "warning");

        let config = child(&tree, "config");
        assert_eq!(config.status, "warning");
        assert_eq!(child(config, "secrets").status, "warning");

        let env = child(child(config, "secrets"), ".env");
        assert_eq!(env.status, "warning");
        assert_eq!(env.severity.as_deref(), Some("critical"));
        assert_eq!(env.reason.as_deref(), Some("high-risk credential filename"));
    }

    #[test]
    fn a_routine_exclusion_is_not_a_warning() {
        // `node_modules` being dropped is expected; showing it as a warning would bury
        // the exclusions that actually matter.
        let plan = plan_with(
            Vec::new(),
            vec![planned("build\\out.js", "excluded", "medium", "*.js rule")],
        );

        let tree = build(&plan, "demo");
        assert_eq!(tree.status, "excluded");
        assert_eq!(child(&tree, "build").status, "excluded");
    }

    #[test]
    fn a_partly_excluded_directory_still_reports_as_included() {
        // It is contributing files to the bundle; calling it excluded would be wrong.
        let plan = plan_with(
            vec![planned("src\\keep.rs", "included", "info", "")],
            vec![planned("src\\drop.log", "excluded", "medium", "*.log rule")],
        );

        let tree = build(&plan, "demo");
        assert_eq!(child(&tree, "src").status, "included");
    }

    #[test]
    fn directories_sort_before_files_and_both_sort_case_insensitively() {
        let plan = plan_with(
            vec![
                planned("zebra.rs", "included", "info", ""),
                planned("Alpha.rs", "included", "info", ""),
                planned("src\\main.rs", "included", "info", ""),
            ],
            Vec::new(),
        );

        let tree = build(&plan, "demo");
        let names: Vec<&str> = tree
            .children
            .as_ref()
            .unwrap()
            .iter()
            .map(|child| child.name.as_str())
            .collect();
        assert_eq!(names, vec!["src", "Alpha.rs", "zebra.rs"]);
    }

    #[test]
    fn an_included_file_carries_no_severity_and_no_reason() {
        // Those fields are what the UI uses to decide whether to explain an exclusion;
        // populating them for an included file would produce a meaningless tooltip.
        let plan = plan_with(vec![planned("main.rs", "included", "info", "")], Vec::new());
        let tree = build(&plan, "demo");
        let file = child(&tree, "main.rs");
        assert_eq!(file.severity, None);
        assert_eq!(file.reason, None);
    }

    #[test]
    fn an_empty_plan_yields_a_root_with_no_children() {
        let plan = plan_with(Vec::new(), Vec::new());
        let tree = build(&plan, "demo");
        assert_eq!(tree.status, "included");
        assert_eq!(tree.children.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn a_file_at_the_root_is_a_direct_child() {
        let plan = plan_with(
            vec![planned("Cargo.toml", "included", "info", "")],
            Vec::new(),
        );
        let tree = build(&plan, "demo");
        let file = child(&tree, "Cargo.toml");
        assert!(!file.is_dir);
        assert_eq!(file.path, "Cargo.toml");
    }
}
