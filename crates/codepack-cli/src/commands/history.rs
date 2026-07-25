//! `codepack history` — what has been exported before.

use codepack_core::time::UtcDateTime;
use serde::Serialize;

use crate::cli::HistoryArgs;
use crate::commands;
use crate::error::Result;
use crate::exit::Outcome;
use crate::output::{self, Format};

#[derive(Debug, Serialize)]
pub(crate) struct HistoryReport {
    /// Absolute path when `--path` narrowed the query, `null` when listing everything.
    pub project: Option<String>,
    pub runs: Vec<HistoryEntry>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HistoryEntry {
    pub run_id: i64,
    pub project_name: String,
    pub project_root: String,
    pub started_at: i64,
    pub started_at_utc: String,
    /// A run that recorded a snapshot baseline. See
    /// `codepack_storage::ExportRunRecord::produced_snapshot` for why this, and not
    /// `cancelled`, is what "successful" means.
    pub successful: bool,
    pub cancelled: bool,
    pub profile: Option<String>,
    pub safe_mode: Option<String>,
    pub diff_mode: Option<String>,
    pub files_copied: Option<i64>,
    pub bytes_total: Option<i64>,
    pub tokens_est: Option<i64>,
    pub redacted_count: Option<i64>,
    pub result_path: Option<String>,
}

pub(crate) fn run(args: &HistoryArgs, format: Format) -> Result<Outcome> {
    let conn = commands::open_history_db()?;

    // A `--path` that has never been exported must read as "no runs", not as an error
    // and not as "every run": `find_or_create_project` would create a row as a side
    // effect of a read-only command, so the project id is looked up without it.
    let (project_id, project) = match &args.path {
        Some(path) => {
            let root = commands::resolve_project_root(path)?;
            let display = root.display().to_string();
            (Some(lookup_project_id(&conn, &display)?), Some(display))
        }
        None => (None, None),
    };

    let runs = match project_id {
        // Narrowed to a project that has no row yet: nothing to show.
        Some(None) => Vec::new(),
        Some(Some(id)) => codepack_storage::list_export_runs(&conn, Some(id), args.limit)?,
        None => codepack_storage::list_export_runs(&conn, None, args.limit)?,
    };

    let report = HistoryReport {
        project,
        runs: runs.into_iter().map(to_entry).collect(),
    };

    if format.is_json() {
        output::emit_json("history", &report)?;
    } else {
        print_human(&report);
    }
    Ok(Outcome::Success)
}

fn lookup_project_id(conn: &codepack_storage::Connection, root_path: &str) -> Result<Option<i64>> {
    Ok(codepack_storage::find_project_id(conn, root_path)?)
}

fn to_entry(record: codepack_storage::ExportRunRecord) -> HistoryEntry {
    HistoryEntry {
        run_id: record.id,
        project_name: record.project_name,
        project_root: record.project_root,
        started_at: record.started_at,
        started_at_utc: UtcDateTime::from_unix_seconds(record.started_at).format_human_utc(),
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
    }
}

fn print_human(report: &HistoryReport) {
    if report.runs.is_empty() {
        match &report.project {
            Some(project) => output::line(format!("No exports recorded for {project}.")),
            None => output::line("No exports recorded yet."),
        }
        return;
    }

    for entry in &report.runs {
        let status = if entry.successful {
            "ok"
        } else if entry.cancelled {
            "cancelled"
        } else {
            "failed"
        };
        let size = entry
            .bytes_total
            .map(|bytes| codepack_tokens::format_bytes(bytes.max(0) as u64))
            .unwrap_or_else(|| "-".to_string());

        output::line(format!(
            "#{:<5} {}  {:<9} {:<12} {:>4} file(s)  {}",
            entry.run_id,
            entry.started_at_utc,
            status,
            entry.profile.as_deref().unwrap_or("-"),
            entry.files_copied.unwrap_or(0),
            size
        ));
        if let Some(path) = &entry.result_path {
            output::line(format!("        {path}"));
        }
    }
}
