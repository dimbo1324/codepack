//! Shared, read-only `git2` helpers for Group C (`05_git_deep.txt`,
//! `21_git_timeline_report.md`). Both reports open the **original** `ReportContext`
//! `source_root` (never `staging_root`) exactly like legacy's own
//! `write_git_deep_report`/`write_git_timeline_report`, which are handed `source_root`
//! by `orchestrator.py` — the copied/staging tree is never a Git working tree with a
//! meaningful history of its own.
//!
//! [`open_repository`] uses [`git2::Repository::open`], not `::discover`: legacy's own
//! not-a-repository check is an exact `(source_root / ".git").exists()`, never a walk
//! up parent directories (unlike `codepack-diff`'s `discover_repository`, which is a
//! deliberately different, documented choice for its own diff-mode selection use
//! case). This module reproduces legacy's own semantics for these two reports.
//!
//! `unsafe`, subprocess `git` invocations, and any network-capable `git2` feature are
//! all forbidden here (invariant I1/domain rules) — see `Cargo.toml`'s
//! `default-features = false` + `vendored-libgit2` line and the crate-scope doc in
//! `lib.rs`.

use std::path::Path;

use git2::{Oid, Repository, Signature};

/// `None` when `source_root` is not a Git working tree (or any other `git2` failure
/// opening it) — the caller's job is to degrade gracefully (a documented "not
/// available" note), never to treat this as a report failure.
pub(crate) fn open_repository(source_root: &Path) -> Option<Repository> {
    Repository::open(source_root).ok()
}

/// Legacy `git branch --show-current`: reads the `HEAD` symbolic reference target
/// directly, so it still resolves on a freshly initialized repository with no commits
/// yet (an "unborn" branch) — unlike [`Repository::head`], which requires `HEAD` to
/// resolve to an actual commit.
pub(crate) fn current_branch_name(repo: &Repository) -> Option<String> {
    let head_ref = repo.find_reference("HEAD").ok()?;
    let target = head_ref.symbolic_target().ok()??;
    target
        .strip_prefix("refs/heads/")
        .map(|name| name.to_string())
}

/// First 7 hex characters of `oid`, matching `git`'s default abbreviated SHA length
/// for typical repository sizes (legacy's `%h`/`rev-parse --short`).
pub(crate) fn short_oid(oid: Oid) -> String {
    let full = oid.to_string();
    full.chars().take(7).collect()
}

/// `YYYY-MM-DD HH:MM:SS UTC` from a commit timestamp — the `git log --date=iso-strict`
/// level of detail rather than legacy's `--date=short`.
///
/// Which of a commit's two timestamps is passed in is the caller's decision, and the
/// two callers differ deliberately: `21_git_timeline_report.md` passes the **author**
/// time, matching the legacy report it reproduces, while `05_git_deep.txt` passes the
/// **committer** time, which is what its graph log orders by.
///
/// Legacy truncated to the day. Owner decision 2026-08-05 reverses that: several
/// commits a day is the normal rhythm of this project, and a day-granular stamp cannot
/// tell a reader which of them came first. The timezone offset Git stores alongside the
/// timestamp is still dropped, for the reason every timestamp in this workspace renders
/// UTC (see [`codepack_core::time`]).
pub(crate) fn format_commit_datetime(seconds_since_epoch: i64) -> String {
    codepack_core::time::UtcDateTime::from_unix_seconds(seconds_since_epoch).format_human_utc()
}

/// `Name <email>`, from a `git2::Signature` — falls back to `"(unknown)"` for a
/// non-UTF-8 name (`git2::Signature::name` returns `None` for non-UTF-8 rather than a
/// lossy string).
pub(crate) fn signature_display(signature: &Signature<'_>) -> String {
    let name = signature.name().unwrap_or("(unknown)");
    let email = signature.email().unwrap_or("(unknown)");
    format!("{name} <{email}>")
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::fs;
    use std::path::Path;

    pub(crate) fn init_repo(dir: &Path) -> Repository {
        Repository::init(dir).expect("git2 init in a fresh tempdir cannot fail")
    }

    pub(crate) fn commit_all(repo: &Repository, message: &str) -> Oid {
        let mut index = repo
            .index()
            .expect("freshly opened repo has a usable index");
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .expect("adding all paths in a tempdir cannot fail");
        index.write().expect("writing the index cannot fail here");
        let tree_id = index
            .write_tree()
            .expect("writing the tree from a valid index cannot fail");
        let tree = repo
            .find_tree(tree_id)
            .expect("the tree was just written by this same call");
        let signature = Signature::now("Test User", "test@example.local")
            .expect("a fixed, valid signature cannot fail");
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<git2::Commit<'_>> = parent.into_iter().collect();
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent_refs,
        )
        .expect("committing a valid tree in a fresh repo cannot fail")
    }

    pub(crate) fn write_file(dir: &Path, relative: &str, contents: &str) {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_repository_returns_none_outside_any_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        assert!(open_repository(dir.path()).is_none());
    }

    #[test]
    fn open_repository_finds_an_initialized_repo() {
        let dir = tempfile::tempdir().unwrap();
        test_support::init_repo(dir.path());
        assert!(open_repository(dir.path()).is_some());
    }

    #[test]
    fn current_branch_name_resolves_even_on_an_unborn_head() {
        let dir = tempfile::tempdir().unwrap();
        let repo = test_support::init_repo(dir.path());
        // No commits yet: `Repository::head()` would fail, but the symbolic-ref target
        // still resolves, matching `git branch --show-current`'s own behavior.
        assert!(repo.head().is_err());
        assert!(current_branch_name(&repo).is_some());
    }

    #[test]
    fn short_oid_truncates_to_seven_characters() {
        let dir = tempfile::tempdir().unwrap();
        let repo = test_support::init_repo(dir.path());
        test_support::write_file(dir.path(), "a.txt", "hi\n");
        let oid = test_support::commit_all(&repo, "init");
        assert_eq!(short_oid(oid).len(), 7);
    }

    #[test]
    fn format_commit_datetime_carries_the_full_moment() {
        assert_eq!(format_commit_datetime(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(
            format_commit_datetime(1_704_112_496),
            "2024-01-01 12:34:56 UTC"
        );
    }

    #[test]
    fn two_commits_on_the_same_day_render_distinguishably() {
        // The whole point of the 2026-08-05 change: a day-granular stamp made these
        // two identical, and the timeline report lists them in the same block.
        let morning = format_commit_datetime(1_704_112_496);
        let evening = format_commit_datetime(1_704_112_496 + 3_600);
        assert_ne!(morning, evening);
    }
}
