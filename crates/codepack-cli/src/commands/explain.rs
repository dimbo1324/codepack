//! `codepack explain <file>` — why one file did or did not make it into the export.
//!
//! The export plan already records a `reason` for every file it classified, and
//! `preview` already builds that plan without writing anything. What was missing was a
//! way to ask about *one* path: `preview --list-files` prints what got in, which is the
//! wrong half of the question — a user chasing a missing file needs to know what
//! excluded it, and a user worried about a leak needs to know why something got in.
//!
//! Every answer is a success. "Excluded because it matched a sensitive name" is the
//! explanation working, not a failure, so the exit code stays 0 for all three outcomes
//! and 1 is reserved for actually failing to produce an answer.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use codepack_core::CancellationToken;
use serde::Serialize;

use crate::cli::ExplainArgs;
use crate::commands::{self, ProjectContext};
use crate::error::{CliError, Result};
use crate::exit::Outcome;
use crate::output::{self, Format};
use crate::settings::ResolutionTrace;

#[derive(Debug, Serialize)]
pub(crate) struct ExplainReport {
    pub project: String,
    /// The path as the plan spells it (backslash-joined, relative to the project), so
    /// the answer can be matched against `manifest.json` and the plan by eye.
    pub file: String,
    pub profile: String,
    pub safe_mode: String,
    pub diff_mode: String,
    /// `included`, `excluded`, `not_in_diff`, or `not_planned`.
    pub verdict: &'static str,
    /// The plan's own wording where it has one; otherwise an explanation assembled
    /// from what the plan does record about the path's directories.
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_human: Option<String>,
    /// The skipped directory on this path, when one explains a `not_planned` verdict.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_directory: Option<String>,
    /// Whether the file exists on disk at all. A `not_planned` verdict means something
    /// quite different for a typo than for a file the walk chose not to visit.
    pub exists_on_disk: bool,
    pub resolution: ResolutionTrace,
}

const VERDICT_INCLUDED: &str = "included";
const VERDICT_EXCLUDED: &str = "excluded";
/// In the plan, but outside the diff selection — so the copy step will skip it and it
/// reaches no bundle. A distinct verdict rather than a flavour of `excluded`, because
/// the fix is different: widen `--diff`, not `--safe-mode` or `--profile`.
const VERDICT_NOT_IN_DIFF: &str = "not_in_diff";
const VERDICT_NOT_PLANNED: &str = "not_planned";

pub(crate) fn run(args: &ExplainArgs, format: Format) -> Result<Outcome> {
    let context = commands::prepare(&args.project)?;
    let report = build(&context, &args.file)?;

    if format.is_json() {
        output::emit_json("explain", &report)?;
    } else {
        print_human(&report);
    }
    Ok(Outcome::Success)
}

pub(crate) fn build(context: &ProjectContext, requested: &Path) -> Result<ExplainReport> {
    let relative = relative_to_project(&context.root, requested)?;
    let key = plan_spelling(&relative);
    // `symlink_metadata` rather than `exists()`: the walker never follows a symlink
    // (invariant I7), so reporting "it exists" by dereferencing one would answer about
    // a file outside the tree the plan describes.
    let exists_on_disk = std::fs::symlink_metadata(context.root.join(&relative)).is_ok();

    // Built exactly the way `preview` builds it, and for the same reason: explaining a
    // file must not write a bundle, a report, or a history row.
    let outcome = codepack_engine::plan_export(
        &context.root,
        &context.config,
        &HashMap::new(),
        None,
        &CancellationToken::new(),
    )?;
    let plan = &outcome.export_plan;

    // Lowercased rather than `eq_ignore_ascii_case`: this project ships Russian-named
    // artifacts, and Windows folds non-ASCII case too, so an ASCII-only comparison
    // would answer "not in the plan" about a file that is plainly in it.
    let folded_key = key.to_lowercase();
    let planned = plan
        .included_files
        .iter()
        .chain(plan.excluded_files.iter())
        .find(|file| file.relative_path.to_lowercase() == folded_key);

    let mut report = ExplainReport {
        project: context.root.display().to_string(),
        file: key.clone(),
        profile: context.config.normalized_export_profile().to_string(),
        safe_mode: context.config.normalized_safe_export_mode().to_string(),
        diff_mode: outcome.diff_selection.mode.clone(),
        verdict: VERDICT_NOT_PLANNED,
        reason: String::new(),
        group: None,
        severity: None,
        size: None,
        size_human: None,
        skipped_directory: None,
        exists_on_disk,
        resolution: context.resolution_for_output(),
    };

    if let Some(file) = planned {
        report.file = file.relative_path.clone();
        // Being in `included_files` is not enough to reach a bundle: under any diff
        // mode but `all`, the copy step further restricts itself to
        // `include_relative_paths` (see `codepack_engine::copy_project`). Reporting
        // "included" for a file the copy will skip would be a confident wrong answer —
        // and the `PR Review` preset makes `uncommitted` an everyday setting.
        let in_diff_selection = outcome
            .include_relative_paths
            .as_ref()
            .is_none_or(|selected| selected.contains(&file.relative_path));

        report.verdict = match (file.status.as_str(), in_diff_selection) {
            ("included", true) => VERDICT_INCLUDED,
            ("included", false) => VERDICT_NOT_IN_DIFF,
            _ => VERDICT_EXCLUDED,
        };
        report.reason = if report.verdict == VERDICT_NOT_IN_DIFF {
            format!(
                "the rules include it, but the `{}` diff selection does not",
                outcome.diff_selection.mode
            )
        } else if file.reason.is_empty() {
            default_reason(report.verdict).to_string()
        } else {
            file.reason.clone()
        };
        report.group = Some(file.group.clone());
        report.severity = Some(file.severity.clone());
        report.size = Some(file.size);
        report.size_human = Some(codepack_tokens::format_bytes(file.size));
        return Ok(report);
    }

    // The plan carries no per-file entry for anything under a directory the walk
    // skipped — it never descended. "Your file is under `node_modules`" is the answer
    // the user actually needs, so reconstruct it from `skipped_dirs`.
    if let Some(entry) = skipped_directory_on_path(&plan.skipped_dirs, &relative) {
        report.reason = format!("not visited: the directory {entry} was skipped");
        report.skipped_directory = Some(entry);
    } else if exists_on_disk {
        report.reason =
            "not in the plan: present on disk but not classified by this configuration".to_string();
    } else {
        report.reason = "not in the plan: no such file in this project".to_string();
    }
    Ok(report)
}

fn default_reason(verdict: &str) -> &'static str {
    if verdict == VERDICT_INCLUDED {
        "included by the current profile and safe mode"
    } else {
        "excluded by the current profile and safe mode"
    }
}

/// Accepts an absolute path, a path relative to the project, or the backslash-joined
/// form the plan itself stores — all three name the same file, and a user copying a
/// path out of `manifest.json` should not have to translate it.
fn relative_to_project(root: &Path, requested: &Path) -> Result<PathBuf> {
    let text = requested.to_string_lossy().replace('\\', "/");
    let normalized = PathBuf::from(text.trim_start_matches("./"));

    let relative = if normalized.is_absolute() {
        // Both sides are put through the same resolution before being compared. Anything
        // less fails on Windows in ways that are easy to miss: CI caught
        // `C:\Users\runneradmin\…` (the file, canonicalized) not matching
        // `C:\Users\RUNNER~1\…` (the root, as given) — an 8.3 short name, which no
        // amount of case-folding reconciles. The same applies to a junction or a mapped
        // drive on one side only.
        let candidate = resolve_through_existing_ancestor(&normalized)?;
        let base = resolve_through_existing_ancestor(root)?;
        strip_project_prefix(&base, &candidate).ok_or_else(|| {
            CliError::message(format!(
                "{} is not inside {}",
                candidate.display(),
                base.display()
            ))
        })?
    } else {
        normalized
    };

    if relative
        .components()
        .any(|part| part == Component::ParentDir)
    {
        return Err(CliError::message(format!(
            "{} escapes the project directory",
            requested.display()
        )));
    }
    // `.` survives `trim_start_matches("./")` as a `CurDir` component, so an empty
    // `OsStr` is not the only spelling of "the project root" that reaches here.
    if relative
        .components()
        .all(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(CliError::message(
            "name a file to explain, not the project root".to_string(),
        ));
    }
    Ok(relative)
}

/// Canonicalizes as much of `path` as exists, then re-appends the rest verbatim.
///
/// A path that does not exist cannot be canonicalized, and "this file is not in the
/// project" is one of the answers `explain` must be able to give — so refusing to
/// resolve a missing path would turn a legitimate question into an error. Resolving the
/// longest existing ancestor gets the real spelling of every component that is actually
/// on disk (short names expanded, symlinks followed, case as the filesystem stores it)
/// and leaves only the genuinely-absent tail as typed.
fn resolve_through_existing_ancestor(path: &Path) -> Result<PathBuf> {
    let mut existing = path;
    let mut tail: Vec<&std::ffi::OsStr> = Vec::new();
    loop {
        if existing.exists() {
            let mut resolved = commands::canonicalize_existing(existing)?;
            for part in tail.iter().rev() {
                resolved.push(part);
            }
            return Ok(resolved);
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name);
                existing = parent;
            }
            // Nothing on this path exists, not even its root — nothing to resolve
            // against, so the caller's spelling is the best available answer.
            _ => return Ok(path.to_path_buf()),
        }
    }
}

/// `Path::strip_prefix` compares components byte-wise apart from the drive letter. Both
/// sides reach here already resolved, so this is normally an exact match; the
/// case-folded fallback covers the tail components that did not exist on disk and could
/// therefore not be resolved — `C:\Proj\SRC\nope.rs` must still get the "no such file"
/// answer rather than a hard error.
fn strip_project_prefix(root: &Path, candidate: &Path) -> Option<PathBuf> {
    if let Ok(relative) = candidate.strip_prefix(root) {
        return Some(relative.to_path_buf());
    }

    let root_parts: Vec<String> = root
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    let candidate_parts: Vec<_> = candidate.components().collect();
    if candidate_parts.len() < root_parts.len() {
        return None;
    }
    let matches = root_parts
        .iter()
        .zip(candidate_parts.iter())
        .all(|(a, b)| *a == b.as_os_str().to_string_lossy().to_lowercase());
    matches.then(|| candidate_parts[root_parts.len()..].iter().collect())
}

/// The plan stores paths backslash-joined regardless of platform (invariant I5), so a
/// lookup key has to be built the same way rather than by `Path::display`.
fn plan_spelling(relative: &Path) -> String {
    relative
        .components()
        .filter_map(|part| match part {
            Component::Normal(text) => Some(text.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\\")
}

/// Finds the skipped directory that contains this path, if any.
///
/// `skipped_dirs` holds already-rendered display strings — `.\node_modules` or
/// `.\node_modules (ignored directory)` — so the structure has to be recovered from
/// presentation. Splitting each entry on `" ("` would be ambiguous the moment a
/// directory is itself named `my folder (v2)`: it would lose the real answer, and could
/// match a *different* directory whose name happens to be the truncated prefix. So the
/// direction is reversed — the path's own ancestors are rendered the way the plan would
/// have rendered them, and an entry matches only if it is that ancestor exactly or that
/// ancestor followed by a parenthesised reason. Both spellings are generated here from
/// the same `format!` the scanner uses, so nothing is parsed at all.
/// Residual ambiguity, named rather than hidden: a project containing both a skipped
/// `build (old)` and a live `build` would, when asked about a *non-existent* file under
/// the live one, be told the skipped sibling explains it. Exact matches are therefore
/// preferred over parenthesised ones, which resolves the case that actually loses an
/// answer (`my folder (v2)` really being skipped); what remains is a wrong explanatory
/// sentence about a file that does not exist, and it costs a structured `SkippedDir` in
/// a contract-frozen artifact to remove entirely.
fn skipped_directory_on_path(skipped_dirs: &[String], relative: &Path) -> Option<String> {
    let folded: Vec<String> = skipped_dirs.iter().map(|dir| dir.to_lowercase()).collect();
    let rendered_ancestors = ancestor_renderings(relative);

    for rendered in &rendered_ancestors {
        if let Some(index) = folded.iter().position(|entry| entry == rendered) {
            return Some(skipped_dirs[index].clone());
        }
    }
    for rendered in &rendered_ancestors {
        let with_reason = format!("{rendered} (");
        if let Some(index) = folded
            .iter()
            .position(|entry| entry.starts_with(&with_reason))
        {
            return Some(skipped_dirs[index].clone());
        }
    }
    None
}

/// Each directory on the path, rendered exactly the way `skipped_dirs` renders one, so
/// nothing has to be parsed back out of a display string.
fn ancestor_renderings(relative: &Path) -> Vec<String> {
    let components: Vec<_> = relative.components().collect();
    let mut ancestor = PathBuf::new();
    let mut rendered = Vec::new();
    for part in components.iter().take(components.len().saturating_sub(1)) {
        let Component::Normal(name) = part else {
            continue;
        };
        ancestor.push(name);
        rendered.push(format!(".\\{}", plan_spelling(&ancestor)).to_lowercase());
    }
    rendered
}

fn print_human(report: &ExplainReport) {
    output::line(format!("Project:   {}", report.project));
    output::line(format!(
        "Settings:  profile={} safe-mode={} diff={}",
        report.profile, report.safe_mode, report.diff_mode
    ));
    output::line("");
    output::line(format!("File:      {}", report.file));
    output::line(format!("Verdict:   {}", report.verdict));
    output::line(format!("Reason:    {}", report.reason));

    if let (Some(group), Some(severity)) = (&report.group, &report.severity) {
        output::line(format!("Group:     {group}"));
        output::line(format!("Severity:  {severity}"));
    }
    if let Some(size_human) = &report.size_human {
        output::line(format!("Size:      {size_human}"));
    }
    if report.verdict == VERDICT_NOT_PLANNED && !report.exists_on_disk {
        output::line("");
        output::line("This path does not exist in the project.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ResolutionTrace;
    use codepack_core::config::Config;

    fn context(root: &Path, safe_mode: &str) -> ProjectContext {
        let config = Config {
            safe_export_mode: safe_mode.to_string(),
            ..Config::default()
        };
        ProjectContext {
            root: root.to_path_buf(),
            config,
            trace: ResolutionTrace::default(),
        }
    }

    /// Commits everything in the tree, so a `uncommitted` diff selection has something
    /// to exclude. Built through `git2`, never a `git` binary (project rule).
    fn commit_everything(root: &Path) {
        use git2::{IndexAddOption, Repository, Signature};

        let repository = Repository::init(root).unwrap();
        let mut index = repository.index().unwrap();
        index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repository.find_tree(tree_id).unwrap();
        let signature = Signature::now("Test", "test@example.local").unwrap();
        repository
            .commit(Some("HEAD"), &signature, &signature, "seed", &tree, &[])
            .unwrap();
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.path().join(".env"), "TOKEN=x\n").unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/left-pad")).unwrap();
        std::fs::write(
            dir.path().join("node_modules/left-pad/index.js"),
            "module.exports = 1;\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn an_included_file_is_explained_with_its_group() {
        let dir = project();
        let report = build(&context(dir.path(), "safe"), Path::new("src/main.rs")).unwrap();

        assert_eq!(report.verdict, VERDICT_INCLUDED);
        assert_eq!(report.file, "src\\main.rs");
        assert!(report.group.is_some());
        assert!(report.size.is_some());
        assert!(report.exists_on_disk);
    }

    #[test]
    fn a_file_excluded_by_safe_mode_says_which_rule_excluded_it() {
        let dir = project();
        let report = build(&context(dir.path(), "safe"), Path::new(".env")).unwrap();

        assert_eq!(report.verdict, VERDICT_EXCLUDED);
        // Naming the rule, not merely "a non-empty string": the fallback wording is
        // always non-empty, so a weaker assertion would keep passing if the plan ever
        // stopped saying *why* — which is the whole gap this command closes.
        assert!(
            report.reason.to_lowercase().contains("credential"),
            "expected the credential-filename rule to be named, got {:?}",
            report.reason
        );
        assert_eq!(report.severity.as_deref(), Some("critical"));
    }

    #[test]
    fn a_file_outside_the_diff_selection_is_not_reported_as_included() {
        // The `PR Review` preset makes `uncommitted` an everyday setting: the rules
        // include a committed, unmodified file, but the copy step will skip it because
        // it is not in the diff selection. Saying "included" would be a confident wrong
        // answer about a file that reaches no bundle.
        let dir = project();
        commit_everything(dir.path());

        let mut context = context(dir.path(), "safe");
        context.config.diff_export_mode = "uncommitted".to_string();

        let report = build(&context, Path::new("src/main.rs")).unwrap();

        assert_eq!(report.verdict, VERDICT_NOT_IN_DIFF, "{}", report.reason);
        assert!(report.reason.contains("diff"), "{}", report.reason);
    }

    #[test]
    fn an_uncommitted_file_is_included_under_the_same_diff_mode() {
        // The other half of the previous test: `not_in_diff` must reflect the diff
        // selection, not simply "this mode always says no".
        let dir = project();
        commit_everything(dir.path());
        std::fs::write(dir.path().join("src/fresh.rs"), "fn fresh() {}\n").unwrap();

        let mut context = context(dir.path(), "safe");
        context.config.diff_export_mode = "uncommitted".to_string();

        let report = build(&context, Path::new("src/fresh.rs")).unwrap();
        assert_eq!(report.verdict, VERDICT_INCLUDED, "{}", report.reason);
    }

    #[test]
    fn a_directory_whose_name_contains_parentheses_still_explains_its_files() {
        // `skipped_dirs` entries are display strings — `.\dir (reason)` — so splitting
        // one on `" ("` would truncate a directory that is itself named `x (v2)` and
        // lose the answer entirely.
        let skipped = vec![".\\my folder (v2)".to_string()];
        assert_eq!(
            skipped_directory_on_path(&skipped, Path::new("my folder (v2)/a.txt")),
            Some(".\\my folder (v2)".to_string())
        );
    }

    #[test]
    fn a_skipped_directory_with_a_reason_is_matched_by_its_path_not_its_text() {
        let skipped = vec![".\\vendor (.exportignore/custom directory rule: vendor)".to_string()];
        assert!(
            skipped_directory_on_path(&skipped, Path::new("vendor/lib/x.go")).is_some(),
            "the parenthesised reason form must still match"
        );
        assert!(
            skipped_directory_on_path(&skipped, Path::new("vendors/lib/x.go")).is_none(),
            "a longer sibling name must not match"
        );
    }

    #[test]
    fn an_absolute_path_that_does_not_exist_still_gets_an_answer() {
        // The root is canonical in production while the user's spelling need not be,
        // and a missing path cannot be canonicalized to match — a case difference must
        // not turn "no such file" into a hard error.
        let dir = tempfile::tempdir().unwrap();
        let root = crate::commands::canonicalize_existing(dir.path()).unwrap();
        let shouted = PathBuf::from(root.to_string_lossy().to_uppercase()).join("src/nope.rs");

        let relative = relative_to_project(&root, &shouted).unwrap();
        assert_eq!(plan_spelling(&relative), "src\\nope.rs");
    }

    #[test]
    fn the_project_root_itself_is_refused_in_every_spelling() {
        let dir = project();
        for spelling in [".", "./", ""] {
            assert!(
                relative_to_project(dir.path(), Path::new(spelling)).is_err(),
                "`{spelling}` names the project root, not a file"
            );
        }
    }

    #[test]
    fn a_file_under_a_skipped_directory_is_told_which_directory() {
        let dir = project();
        let report = build(
            &context(dir.path(), "safe"),
            Path::new("node_modules/left-pad/index.js"),
        )
        .unwrap();

        assert_eq!(report.verdict, VERDICT_NOT_PLANNED);
        assert!(
            report
                .skipped_directory
                .as_deref()
                .is_some_and(|entry| entry.contains("node_modules")),
            "reason was {:?}",
            report.reason
        );
    }

    #[test]
    fn a_path_that_does_not_exist_gets_an_answer_not_an_error() {
        let dir = project();
        let report = build(&context(dir.path(), "safe"), Path::new("src/nope.rs")).unwrap();

        assert_eq!(report.verdict, VERDICT_NOT_PLANNED);
        assert!(!report.exists_on_disk);
        assert!(report.reason.contains("no such file"), "{}", report.reason);
    }

    #[test]
    fn absolute_relative_and_plan_spellings_all_agree() {
        let dir = project();
        let context = context(dir.path(), "safe");

        let relative = build(&context, Path::new("src/main.rs")).unwrap();
        let absolute = build(&context, &dir.path().join("src/main.rs")).unwrap();
        let plan_form = build(&context, Path::new("src\\main.rs")).unwrap();
        let dotted = build(&context, Path::new("./src/main.rs")).unwrap();

        for other in [&absolute, &plan_form, &dotted] {
            assert_eq!(other.file, relative.file);
            assert_eq!(other.verdict, relative.verdict);
        }
    }

    /// Regression for the CI failure of 2026-07-29: on the GitHub Windows runner the
    /// project root arrived as `C:\Users\RUNNER~1\…` (an 8.3 short name) while the
    /// canonicalized file was `C:\Users\runneradmin\…`, and the two did not match. The
    /// fix resolves both sides the same way, so this asserts on the helper rather than
    /// on a machine that happens to have short names enabled.
    #[test]
    fn both_sides_of_the_comparison_are_resolved_the_same_way() {
        let dir = project();
        let raw = dir.path();
        let canonical = crate::commands::canonicalize_existing(raw).unwrap();

        // Whichever spelling the caller has, the answer is the same file.
        let from_raw = relative_to_project(raw, &canonical.join("src/main.rs")).unwrap();
        let from_canonical = relative_to_project(&canonical, &raw.join("src/main.rs")).unwrap();

        assert_eq!(plan_spelling(&from_raw), "src\\main.rs");
        assert_eq!(plan_spelling(&from_canonical), "src\\main.rs");
    }

    #[test]
    fn a_missing_tail_is_kept_verbatim_while_its_existing_ancestor_is_resolved() {
        let dir = project();
        let resolved = resolve_through_existing_ancestor(&dir.path().join("src/deep/nope.rs"));

        let resolved = resolved.unwrap();
        assert!(resolved.ends_with("src/deep/nope.rs"), "{resolved:?}");
        assert!(
            resolved.starts_with(crate::commands::canonicalize_existing(dir.path()).unwrap()),
            "the existing ancestor should have been canonicalized: {resolved:?}"
        );
    }

    #[test]
    fn a_path_outside_the_project_is_refused_rather_than_answered_about() {
        let dir = project();
        let outside = tempfile::tempdir().unwrap();
        let error = build(
            &context(dir.path(), "safe"),
            &outside.path().join("elsewhere.rs"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("not inside"), "{error}");
    }

    #[test]
    fn a_traversal_attempt_is_refused() {
        let dir = project();
        let error = build(&context(dir.path(), "safe"), Path::new("../secrets.txt")).unwrap_err();
        assert!(error.to_string().contains("escapes"), "{error}");
    }
}
