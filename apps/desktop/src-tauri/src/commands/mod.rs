//! The command surface the webview may call.
//!
//! Every function here is the *only* way the frontend can reach the filesystem, the
//! database or the engine — the webview itself is granted no `fs` permission at all
//! (`capabilities/default.json`). That is ROADMAP §3's isolation requirement, enforced
//! by capability rather than by convention.
//!
//! Each module owns one area, and each command is a thin adapter: resolve inputs, call
//! the already-tested core crate, shape the result into a [`crate::dto`] type. Logic
//! that belongs to the domain stays in the domain crates, so the GUI and the CLI cannot
//! drift apart in what an export actually means.

pub mod ai;
pub mod app_info;
pub mod export;
pub mod history;
pub mod project;
pub mod sanitize;
pub mod settings;
pub mod watch;
pub mod window;

use std::path::{Path, PathBuf};

use crate::error::{CommandError, CommandResult};

/// Validates a path the frontend supplied and turns it into an absolute directory.
///
/// The frontend only ever sends back a path the native picker produced, but "only ever"
/// is a statement about today's UI, not about the boundary. A command is a public entry
/// point: it validates what it is given.
pub fn resolve_project_root(path: &str) -> CommandResult<PathBuf> {
    let raw = Path::new(path);
    if raw.as_os_str().is_empty() {
        return Err(CommandError::new("no project directory was given"));
    }

    let resolved = raw
        .canonicalize()
        .map_err(|_| CommandError::new(format!("cannot open project directory: {path}")))?;

    if !resolved.is_dir() {
        return Err(CommandError::new(format!(
            "not a directory: {}",
            resolved.display()
        )));
    }
    Ok(resolved)
}

/// Opens the history database, creating it and its parent directory if needed.
///
/// The path comes from `AppPaths`, never from the frontend: which database an export is
/// recorded in is not a decision the webview gets to make.
pub fn open_database() -> CommandResult<codepack_storage::Connection> {
    let paths = codepack_core::AppPaths::resolve()?;
    let db_file = paths.db_file();
    if let Some(parent) = db_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(codepack_storage::open(&db_file)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_existing_directory_resolves_to_an_absolute_path() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve_project_root(&dir.path().display().to_string()).unwrap();
        assert!(resolved.is_absolute());
        assert!(resolved.is_dir());
    }

    #[test]
    fn an_empty_path_is_rejected_with_a_readable_message() {
        let error = resolve_project_root("").unwrap_err();
        assert!(error.message.contains("no project directory"));
    }

    #[test]
    fn a_missing_directory_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        let error = resolve_project_root(&missing.display().to_string()).unwrap_err();
        assert!(error.message.contains("cannot open project directory"));
    }

    #[test]
    fn a_file_is_rejected_because_a_project_is_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("main.rs");
        std::fs::write(&file, "fn main() {}").unwrap();

        let error = resolve_project_root(&file.display().to_string()).unwrap_err();
        assert!(
            error.message.contains("not a directory"),
            "unexpected message: {}",
            error.message
        );
    }
}
