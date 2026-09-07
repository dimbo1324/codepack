//! `codepack doctor` — what this installation can see and do.
//!
//! Read-only by default, and never fails on a finding: its whole job is to answer
//! questions before something goes wrong, so a missing optional piece is reported, not
//! raised. `--collect-logs DIR` is the one declared exception — it writes a copy of the
//! activity log to a directory the caller named, for attaching to a bug report.

use std::path::Path;

use codepack_core::AppPaths;
use codepack_core::config::ai_presets;
use serde::Serialize;

use crate::cli::DoctorArgs;
use crate::error::{CliError, Result};
use crate::exit::Outcome;
use crate::output::{self, Format};

#[derive(Debug, Serialize)]
pub(crate) struct DoctorReport {
    pub version: &'static str,
    pub json_schema_version: u32,
    pub paths: Paths,
    pub presets: Vec<PresetInfo>,
    pub profiles: Vec<String>,
    pub project_config_file: &'static str,
    /// Stated explicitly rather than implied: this is the product's central promise,
    /// and someone auditing the tool should be able to read it from its own output.
    pub network_access: &'static str,
    /// Present only when `--collect-logs` was passed (audit 2026-09-07, G-1). An added
    /// field, not a changed one: a `--json` consumer that has never heard of it reads
    /// exactly what it read before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_collection: Option<LogCollection>,
}

#[derive(Debug, Serialize)]
pub(crate) struct LogCollection {
    pub destination: String,
    pub files: Vec<String>,
    pub total_bytes: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct Paths {
    pub settings_file: String,
    pub settings_file_exists: bool,
    pub database: String,
    pub database_exists: bool,
    /// Set only while a database from before the 2026-09-06 move is still sitting in the
    /// old place, so `database` above names a file that does not exist yet.
    ///
    /// An added field rather than a changed one: a consumer of `doctor --json` that has
    /// never heard of it reads exactly what it read before. It disappears — as `null` —
    /// the moment the database is opened, because opening it performs the move.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_superseded: Option<String>,
    pub user_profiles_file: String,
    pub user_profiles_file_exists: bool,
    pub(crate) model_limits_file: String,
    pub(crate) model_limits_file_exists: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct PresetInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub export_profile: &'static str,
    pub safe_export_mode: &'static str,
}

pub(crate) fn run(args: &DoctorArgs, format: Format) -> Result<Outcome> {
    let report = build(args)?;
    if format.is_json() {
        output::emit_json("doctor", &report)?;
    } else {
        print_human(&report);
    }
    Ok(Outcome::Success)
}

fn build(args: &DoctorArgs) -> Result<DoctorReport> {
    let app_paths = AppPaths::resolve()?;
    let settings_file = app_paths.settings_file();
    let database = app_paths.db_file();
    let superseded = Some(app_paths.superseded_db_file()).filter(|old| *old != database);
    let user_profiles_file = app_paths.user_profiles_file();
    let model_limits_file = app_paths.model_limits_file();

    // A profiles file that exists but is corrupt is reported as "no profiles" here
    // rather than failing: `doctor` is what a user runs *because* something is wrong,
    // so it must survive a broken file long enough to show them the path to it.
    let profiles = codepack_core::profiles::load(&user_profiles_file)
        .map(|loaded| loaded.file.profiles.keys().cloned().collect())
        .unwrap_or_default();

    let log_collection = match &args.collect_logs {
        Some(destination) => Some(collect_logs(
            app_paths.log_dir(),
            app_paths.home_dir(),
            destination,
        )?),
        None => None,
    };

    Ok(DoctorReport {
        version: env!("CARGO_PKG_VERSION"),
        json_schema_version: crate::output::JSON_SCHEMA_VERSION,
        paths: Paths {
            settings_file_exists: settings_file.is_file(),
            settings_file: settings_file.display().to_string(),
            database_exists: database.is_file(),
            database: database.display().to_string(),
            // `doctor` is read-only by contract, so it reports the pending move rather
            // than performing it. Without this a user upgrading on Linux would be told a
            // path, look there, and find nothing.
            database_superseded: superseded
                .filter(|path| path.is_file())
                .map(|path| path.display().to_string()),
            user_profiles_file_exists: user_profiles_file.is_file(),
            user_profiles_file: user_profiles_file.display().to_string(),
            model_limits_file_exists: model_limits_file.is_file(),
            model_limits_file: model_limits_file.display().to_string(),
        },
        presets: ai_presets()
            .iter()
            .map(|preset| PresetInfo {
                name: preset.name,
                description: preset.description,
                export_profile: preset.export_profile,
                safe_export_mode: preset.safe_export_mode,
            })
            .collect(),
        profiles,
        project_config_file: codepack_core::config::PROJECT_CONFIG_FILE_NAME,
        network_access: "never; all analysis is local",
        log_collection,
    })
}

/// Copies every file in `log_dir` into `destination`, redacting the user's home
/// directory out of each line first.
///
/// The log files are already redacted for secrets at write time
/// (`codepack_engine::LogLine`) — this is a second, different redaction, for a
/// different audience: a log line legitimately says which files an export touched, and
/// that path starts with the local machine's home directory and username, neither of
/// which the person receiving a bug report needs. The same instinct
/// `codepack_core::config::disclosed_root` already applies to a project's own path.
///
/// A missing log directory (nothing has ever been logged yet) is not an error: it
/// yields an empty collection, exactly like a project doctor already treats a missing
/// profiles file as "no profiles" rather than a failure.
fn collect_logs(log_dir: &Path, home_dir: &Path, destination: &Path) -> Result<LogCollection> {
    std::fs::create_dir_all(destination).map_err(|source| CliError::Read {
        path: destination.to_path_buf(),
        source,
    })?;

    let home_needle = home_dir.display().to_string();
    let mut files = Vec::new();
    let mut total_bytes = 0u64;

    let entries = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LogCollection {
                destination: destination.display().to_string(),
                files,
                total_bytes,
            });
        }
        Err(source) => {
            return Err(CliError::Read {
                path: log_dir.to_path_buf(),
                source,
            });
        }
    };

    let mut source_paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| CliError::Read {
            path: log_dir.to_path_buf(),
            source,
        })?;
        if entry.path().is_file() {
            source_paths.push(entry.path());
        }
    }
    // Sorted so the report — and the test that reads it back — does not depend on the
    // order a directory listing happens to arrive in.
    source_paths.sort();

    for source_path in source_paths {
        let contents = std::fs::read_to_string(&source_path).map_err(|source| CliError::Read {
            path: source_path.clone(),
            source,
        })?;
        let redacted = contents.replace(&home_needle, "<home>");

        let file_name = source_path
            .file_name()
            .expect("a directory entry always has a file name")
            .to_owned();
        let target_path = destination.join(&file_name);
        std::fs::write(&target_path, &redacted).map_err(|source| CliError::Read {
            path: target_path.clone(),
            source,
        })?;

        total_bytes += redacted.len() as u64;
        files.push(file_name.to_string_lossy().into_owned());
    }

    Ok(LogCollection {
        destination: destination.display().to_string(),
        files,
        total_bytes,
    })
}

fn print_human(report: &DoctorReport) {
    output::line(format!(
        "codepack {} (json schema v{})",
        report.version, report.json_schema_version
    ));
    output::line(format!("Network:  {}", report.network_access));
    output::line("");

    output::line("Paths:");
    for (label, path, exists) in [
        (
            "settings",
            &report.paths.settings_file,
            report.paths.settings_file_exists,
        ),
        (
            "history",
            &report.paths.database,
            report.paths.database_exists,
        ),
        (
            "profiles",
            &report.paths.user_profiles_file,
            report.paths.user_profiles_file_exists,
        ),
        (
            "model limits",
            &report.paths.model_limits_file,
            report.paths.model_limits_file_exists,
        ),
    ] {
        let mark = if exists { "present" } else { "absent" };
        output::line(format!("  {label:<13} {path} ({mark})"));
    }

    if let Some(superseded) = &report.paths.database_superseded {
        output::line(format!(
            "  {:<13} {superseded} (moves here on first use)",
            "history (old)"
        ));
    }

    output::line("");
    output::line(format!("Project config: {}", report.project_config_file));

    output::line("");
    output::line("Presets:");
    for preset in &report.presets {
        output::line(format!(
            "  {:<18} {} [{}, {}]",
            preset.name, preset.description, preset.export_profile, preset.safe_export_mode
        ));
    }

    output::line("");
    if report.profiles.is_empty() {
        output::line("Custom profiles: none");
    } else {
        output::line(format!("Custom profiles: {}", report.profiles.join(", ")));
    }

    if let Some(collection) = &report.log_collection {
        output::line("");
        output::line(format!("Logs collected into: {}", collection.destination));
        if collection.files.is_empty() {
            output::line("  (nothing has been logged yet)");
        } else {
            for file in &collection.files {
                output::line(format!("  {file}"));
            }
            output::line(format!("  {} byte(s) total", collection.total_bytes));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_log_directory_yields_an_empty_collection_not_an_error() {
        let log_dir = tempfile::tempdir().unwrap();
        let home_dir = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        // The directory itself is never created — nothing has been logged yet.
        let never_opened = log_dir.path().join("does-not-exist");

        let collection = collect_logs(&never_opened, home_dir.path(), destination.path()).unwrap();

        assert!(collection.files.is_empty());
        assert_eq!(collection.total_bytes, 0);
    }

    #[test]
    fn the_home_directory_is_replaced_in_every_copied_line() {
        let log_dir = tempfile::tempdir().unwrap();
        let home_dir = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();

        let leaked_path = home_dir.path().join("projects").join("secret-client");
        std::fs::write(
            log_dir.path().join("codepack-2026-09-08.log"),
            format!(
                "2026-09-08T00:00:00Z WARN run=r-1 could not refresh dashboard for {}\n",
                leaked_path.display()
            ),
        )
        .unwrap();

        let collection = collect_logs(log_dir.path(), home_dir.path(), destination.path()).unwrap();

        assert_eq!(collection.files, vec!["codepack-2026-09-08.log"]);
        let copied =
            std::fs::read_to_string(destination.path().join("codepack-2026-09-08.log")).unwrap();
        assert!(
            !copied.contains(&home_dir.path().display().to_string()),
            "the home directory should not survive collection: {copied}"
        );
        assert!(copied.contains("<home>"));
        assert!(copied.contains("could not refresh dashboard"));
    }

    #[test]
    fn every_file_in_the_log_directory_is_collected_and_counted() {
        let log_dir = tempfile::tempdir().unwrap();
        let home_dir = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();

        std::fs::write(log_dir.path().join("codepack-2026-09-07.log"), "a\n").unwrap();
        std::fs::write(log_dir.path().join("codepack-panics.log"), "bb\n").unwrap();

        let collection = collect_logs(log_dir.path(), home_dir.path(), destination.path()).unwrap();

        assert_eq!(
            collection.files,
            vec!["codepack-2026-09-07.log", "codepack-panics.log"]
        );
        assert_eq!(collection.total_bytes, 2 + 3);
    }
}
