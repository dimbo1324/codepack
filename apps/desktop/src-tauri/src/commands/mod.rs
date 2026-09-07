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

/// Checks that a path really is an export this installation produced, and returns it as
/// a [`ValidatedResultPath`] so the boundary this function holds cannot be bypassed by
/// forgetting to call it.
///
/// The paired rule to [`resolve_project_root`], and for the reason this module already
/// states about project paths: "the frontend only ever sends back a path the native
/// picker produced, but 'only ever' is a statement about today's UI, not about the
/// boundary". Six commands took a `result_path` straight from the webview and used it to
/// unpack archives and hand files to the OS opener — which is the whole of the isolation
/// the capability file is there to provide, given away by the commands meant to hold it.
///
/// The check is against a *fact* rather than a shape: the history database records the
/// `result_path` of every run, so a path is acceptable exactly when a run produced it.
/// No amount of string cleverness can forge that.
///
/// Comparison is on canonicalised paths, so `.`-segments, a different case on Windows and
/// a short 8.3 spelling all resolve to the same answer.
///
/// Resolves the real `AppPaths` and opens the real database — see
/// [`ValidatedResultPath::resolve`] for the form every unit test in this crate uses
/// instead, against an explicit, temporary one.
pub fn resolve_export_result(result_path: &str) -> CommandResult<ValidatedResultPath> {
    let paths = codepack_core::AppPaths::resolve()?;
    ValidatedResultPath::resolve(&paths, result_path)
}

/// A `result_path` the export history has confirmed this installation actually produced.
///
/// The mechanism audit 2026-09-07 (S-1/Q-1) replaced a comment with: five of six
/// bundle-opening commands called `extract_validated_bundle`/`open_bundle_report`
/// directly on a bare `&str`/`&Path` from the webview, skipping
/// [`resolve_export_result`] entirely, even though that function's own doc comment said
/// "nothing but [it] should call" the extraction step. A comment cannot be checked by the
/// compiler; a private field can. The only way to obtain one of these is
/// [`Self::resolve`], and every command that extracts, reads or opens a bundle now takes
/// one of these rather than a path — so a bundle command written tomorrow that forgets
/// the check will not compile, instead of compiling and quietly trusting the webview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedResultPath(PathBuf);

impl ValidatedResultPath {
    /// Validates `result_path` against `paths`'s history database.
    ///
    /// Takes an explicit [`codepack_core::AppPaths`] rather than resolving one itself,
    /// which is what makes this testable without writing into whoever runs `cargo test`'s
    /// real history database (audit 2026-09-07, S-7/T-2): [`super::resolve_export_result`]
    /// is the production entry point that resolves the real environment and calls
    /// straight through to this; every test calls this directly against
    /// `codepack_core::AppPaths::for_root(&tempdir)` instead. Before this split existed,
    /// every test on this check ran against the developer's actual profile — which is
    /// also how two test threads once raced to create the same fresh database (see
    /// `docs/__arch__/open-questions.md`, 2026-09-06, "Две гонки при первом открытии
    /// базы").
    pub fn resolve(
        paths: &codepack_core::AppPaths,
        result_path: &str,
    ) -> CommandResult<ValidatedResultPath> {
        let raw = Path::new(result_path);
        if raw.as_os_str().is_empty() {
            return Err(CommandError::new("no export result was given"));
        }
        let resolved = raw.canonicalize().map_err(|_| {
            CommandError::new(format!(
                "the export result is no longer where it was recorded: {result_path}"
            ))
        })?;

        let connection = open_database_at(paths)?;
        // Audit 2026-09-07, S-2: this used to be `list_export_runs(&connection, None, 0)`
        // followed by a search of the result — and `LIMIT 0` is SQLite for "zero rows",
        // so that call always returned an empty list and every path was rejected
        // unconditionally, including ones this installation had just produced. The
        // acceptance branch had no test, so nothing noticed. See
        // `export_run_result_path_matches`'s own doc comment for why this is also no
        // longer "fetch the whole history and canonicalize every row".
        let known = codepack_storage::export_run_result_path_matches(&connection, &resolved)
            .map_err(CommandError::new)?;

        if !known {
            return Err(CommandError::new(format!(
                "{} is not an export this installation produced, so it will not be opened",
                resolved.display()
            )));
        }
        Ok(ValidatedResultPath(resolved))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Opens the history database at `paths`, creating it and its parent directory if
/// needed.
///
/// `paths` is a parameter rather than resolved internally for the same reason
/// [`ValidatedResultPath::resolve`] takes one: it is what lets a test open a temporary
/// database instead of the real one. [`open_database`] is the production entry point
/// that resolves the real environment and calls straight through.
pub(crate) fn open_database_at(
    paths: &codepack_core::AppPaths,
) -> CommandResult<codepack_storage::Connection> {
    // See `codepack_core::migrated_db_file`: on Linux the database moved out of the
    // settings directory, and an older installation still has its history there.
    let db_file = codepack_core::migrated_db_file(paths);
    if let Some(parent) = db_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(codepack_storage::open(&db_file)?)
}

/// Opens the history database, creating it and its parent directory if needed.
///
/// The path comes from `AppPaths`, never from the frontend: which database an export is
/// recorded in is not a decision the webview gets to make.
pub fn open_database() -> CommandResult<codepack_storage::Connection> {
    let paths = codepack_core::AppPaths::resolve()?;
    open_database_at(&paths)
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

#[cfg(test)]
mod export_result_tests {
    use super::*;
    use codepack_core::AppPaths;
    use codepack_storage::NewExportRun;

    /// A fresh, isolated `AppPaths` under a tempdir, so these tests never touch whoever
    /// runs `cargo test`'s real settings directory or history database (audit
    /// 2026-09-07, S-7/T-2). `AppPaths::for_root` lays the three directories out for
    /// whichever OS the test actually runs on, exactly as `AppPaths::resolve` would.
    fn isolated_paths(root: &std::path::Path) -> AppPaths {
        AppPaths::for_root(root)
    }

    /// Records one export run against `paths`'s database and returns the canonicalized
    /// path it was recorded under — the shape every acceptance test below needs, so it
    /// is not repeated four times.
    fn record_a_run_with_result_path(
        paths: &AppPaths,
        result_path: &std::path::Path,
    ) -> std::path::PathBuf {
        let mut connection = open_database_at(paths).unwrap();
        let project =
            codepack_storage::find_or_create_project(&connection, "/tmp/project", "project", None)
                .unwrap();
        codepack_storage::record_export_run(
            &mut connection,
            NewExportRun {
                project_id: project,
                started_at: 10,
                finished_at: Some(11),
                profile: Some("full".to_string()),
                safe_mode: Some("safe".to_string()),
                diff_mode: Some("all".to_string()),
                files_copied: Some(1),
                bytes_total: Some(10),
                tokens_est: Some(1),
                redacted_count: None,
                cancelled: false,
                result_path: Some(result_path.display().to_string()),
            },
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();
        result_path.canonicalize().unwrap()
    }

    /// The acceptance branch that had no test at all before this pass (audit S-2/T-1):
    /// every other test on this check exercises rejection, and a query that always
    /// returns "reject everything" — which `LIMIT 0` did — passes every one of them.
    /// This is the test that would have caught it immediately.
    #[test]
    fn a_path_a_recorded_run_produced_is_accepted() {
        let root = tempfile::tempdir().unwrap();
        let paths = isolated_paths(root.path());
        let bundle_dir = tempfile::tempdir().unwrap();
        let bundle = bundle_dir.path().join("bundle.zip");
        std::fs::write(&bundle, b"a real export artifact").unwrap();
        let canonical = record_a_run_with_result_path(&paths, &bundle);

        let validated = ValidatedResultPath::resolve(&paths, &bundle.display().to_string())
            .expect("a run this installation produced must be accepted");
        assert_eq!(validated.as_path(), canonical);
    }

    /// The point of the check: a path the webview invents is refused, however well formed
    /// it looks. A freshly created temporary file cannot be in anyone's export history —
    /// checked here against a database that is genuinely empty, not merely one this test
    /// hopes has nothing matching in it.
    #[test]
    fn a_path_no_run_produced_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let paths = isolated_paths(root.path());
        let bundle_dir = tempfile::tempdir().unwrap();
        let stranger = bundle_dir.path().join("bundle.zip");
        std::fs::write(&stranger, b"not a real export").unwrap();

        let error = ValidatedResultPath::resolve(&paths, &stranger.display().to_string())
            .expect_err("an unrecorded path must not be opened");
        assert!(
            format!("{error:?}").contains("not an export this installation produced"),
            "{error:?}"
        );
    }

    /// A path resembling one that *was* recorded — same file name, different directory —
    /// must not be accepted on the strength of the name alone. This is the test that
    /// proves the `LIKE`-based candidate narrowing in `export_run_result_path_matches`
    /// is only ever a pre-filter, never the actual comparison.
    #[test]
    fn a_path_sharing_a_recorded_runs_file_name_but_not_its_directory_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let paths = isolated_paths(root.path());
        let real_dir = tempfile::tempdir().unwrap();
        let real_bundle = real_dir.path().join("bundle.zip");
        std::fs::write(&real_bundle, b"a real export artifact").unwrap();
        record_a_run_with_result_path(&paths, &real_bundle);

        let impostor_dir = tempfile::tempdir().unwrap();
        let impostor = impostor_dir.path().join("bundle.zip");
        std::fs::write(&impostor, b"not the same file").unwrap();

        let error = ValidatedResultPath::resolve(&paths, &impostor.display().to_string())
            .expect_err("a same-named file in a different directory is not the recorded run");
        assert!(
            format!("{error:?}").contains("not an export this installation produced"),
            "{error:?}"
        );
    }

    #[test]
    fn an_empty_or_missing_path_is_refused_before_the_database_is_touched() {
        let root = tempfile::tempdir().unwrap();
        let paths = isolated_paths(root.path());
        assert!(ValidatedResultPath::resolve(&paths, "").is_err());
        let dir = tempfile::tempdir().unwrap();
        assert!(
            ValidatedResultPath::resolve(
                &paths,
                &dir.path().join("absent.zip").display().to_string()
            )
            .is_err()
        );
    }

    /// The production entry point resolves the real environment and calls straight
    /// through — proven here by asking it about a path that cannot possibly be in
    /// anyone's real history, so the assertion holds regardless of whose machine runs it.
    #[test]
    fn the_production_entry_point_reaches_the_same_check() {
        let dir = tempfile::tempdir().unwrap();
        let stranger = dir.path().join("definitely-not-a-real-export.zip");
        std::fs::write(&stranger, b"not a real export").unwrap();

        let error = resolve_export_result(&stranger.display().to_string())
            .expect_err("an unrecorded path must not be opened");
        assert!(
            format!("{error:?}").contains("not an export this installation produced"),
            "{error:?}"
        );
    }
}
