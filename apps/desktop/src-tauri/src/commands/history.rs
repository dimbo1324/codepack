//! Past export runs, read from the history database.

use codepack_core::time::UtcDateTime;

use crate::dto::{HistoryEntry, HistoryReport};
use crate::error::CommandResult;

use super::{open_database, resolve_project_root};

/// How many runs to return when the caller does not say. Matches the default retention
/// window, so "show me everything" and "show me the default page" agree.
const DEFAULT_LIMIT: usize = 50;

/// Lists past runs, newest first.
///
/// `project_root` narrows to one project; `None` returns runs across all of them, which
/// is what the History page shows before a project is opened.
///
/// An unknown project is not an error: a project with no runs yet and a project that was
/// never opened look the same from here, and both should render as an empty list rather
/// than a failure.
#[tauri::command]
pub fn fetch_history(
    project_root: Option<String>,
    limit: Option<usize>,
) -> CommandResult<HistoryReport> {
    let connection = open_database()?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, 1000);

    let (project_id, project) = match project_root.as_deref() {
        Some(path) if !path.is_empty() => {
            let root = resolve_project_root(path)?;
            let root_text = root.display().to_string();
            let id = codepack_storage::find_project_id(&connection, &root_text)?;
            (id, Some(root_text))
        }
        _ => (None, None),
    };

    // A project that exists but has no runs, and a project that was never recorded, both
    // yield an empty list. Querying with `Some(id)` for the first and short-circuiting
    // for the second keeps the two indistinguishable to the caller, which is correct:
    // the answer to "what has this project exported?" is "nothing" either way.
    let runs = if project.is_some() && project_id.is_none() {
        Vec::new()
    } else {
        codepack_storage::list_export_runs(&connection, project_id, limit)?
            .into_iter()
            .map(|record| HistoryEntry {
                run_id: record.id,
                project_name: record.project_name,
                project_root: record.project_root,
                started_at: record.started_at,
                started_at_utc: UtcDateTime::from_unix_seconds(record.started_at)
                    .format_human_utc(),
                // A run is successful when it produced a snapshot baseline — the same
                // gate `record_export_run` uses, so this cannot disagree with what the
                // database actually did (invariant I6).
                successful: record.produced_snapshot,
                cancelled: record.cancelled,
                profile: record.profile,
                safe_mode: record.safe_mode,
                diff_mode: record.diff_mode,
                files_copied: record.files_copied,
                bytes_total: record.bytes_total,
                tokens_est: record.tokens_est,
                redacted_count: record.redacted_count,
                result_path: record.result_path,
            })
            .collect()
    };

    Ok(HistoryReport { project, runs })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limit_is_clamped_into_a_sane_range() {
        // Guards the arithmetic rather than the database: `0` would return nothing at
        // all, and an unbounded value would let a page request the whole table.
        assert_eq!(0usize.clamp(1, 1000), 1);
        assert_eq!(5000usize.clamp(1, 1000), 1000);
        assert_eq!(DEFAULT_LIMIT.clamp(1, 1000), DEFAULT_LIMIT);
    }

    #[test]
    fn a_started_at_timestamp_renders_as_a_readable_utc_string() {
        // What the History table shows; the raw epoch second is kept alongside it for
        // sorting, so both must come from the same value.
        let rendered = UtcDateTime::from_unix_seconds(1_704_112_496).format_human_utc();
        assert_eq!(rendered, "2024-01-01 12:34:56 UTC");
    }
}
