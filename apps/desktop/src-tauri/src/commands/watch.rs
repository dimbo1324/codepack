//! Watch mode: telling the UI when the project changed underneath it.
//!
//! This does **not** re-export on every keystroke. It emits `watch:changed`, and the UI
//! decides what to do with that — refresh the preview, or (if the user asked for it)
//! update the clipboard. Re-running a full export automatically would write archives
//! nobody asked for, which is the opposite of a tool built around deliberate handoff.
//!
//! Only the project root is watched, and only for content changes. Ignored directories
//! are filtered here rather than at the OS level because `notify` has no portable way to
//! exclude a subtree, and `node_modules` churning during an install would otherwise
//! drown the channel.

use std::path::Path;
use std::time::{Duration, Instant};

use codepack_core::config::Config;
use notify::{Event, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, State};

use crate::dto::WatchChangedEvent;
use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

pub const CHANGED_EVENT: &str = "watch:changed";

/// How long to wait after a change before telling the UI.
///
/// Saving a file in an editor produces several events in quick succession (write,
/// rename, attribute change), and a build produces hundreds. Coalescing them into one
/// notification is the difference between a useful signal and a flood.
const DEBOUNCE: Duration = Duration::from_millis(400);

/// Starts watching `project_root`. Replaces any previous watch.
#[tauri::command]
pub fn start_watch(
    app: AppHandle,
    state: State<'_, AppState>,
    project_root: String,
    config: Config,
) -> CommandResult<()> {
    let root = super::resolve_project_root(&project_root)?;

    let ignored = ignored_directory_names(&root, &config);
    let watch_root = root.clone();
    let mut last_emitted = Instant::now() - DEBOUNCE;
    let mut pending: Vec<String> = Vec::new();

    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
        let Ok(event) = result else {
            // A watch error (a directory disappearing mid-walk, a permission change) is
            // not worth interrupting the user over: the watch keeps running, and the
            // next real change still reports.
            return;
        };
        if !event.kind.is_create() && !event.kind.is_modify() && !event.kind.is_remove() {
            return;
        }

        for path in &event.paths {
            if is_ignored(path, &watch_root, &ignored) {
                continue;
            }
            pending.push(path.display().to_string());
        }
        if pending.is_empty() || last_emitted.elapsed() < DEBOUNCE {
            return;
        }

        last_emitted = Instant::now();
        let changed_paths = std::mem::take(&mut pending);
        let _ = app.emit(CHANGED_EVENT, WatchChangedEvent { changed_paths });
    })
    .map_err(CommandError::new)?;

    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(CommandError::new)?;

    state.watch.replace(Box::new(watcher));
    Ok(())
}

/// Stops watching. Idempotent: stopping when nothing is watched is not an error.
#[tauri::command]
pub fn stop_watch(state: State<'_, AppState>) -> CommandResult<()> {
    state.watch.clear();
    Ok(())
}

/// The directory names a change inside is not worth reporting.
///
/// The same set the scanner prunes with, so the watch agrees with the export about what
/// counts as part of the project.
fn ignored_directory_names(root: &Path, config: &Config) -> Vec<String> {
    let mut names: Vec<String> = codepack_scanner::IGNORED_DIR_NAMES
        .iter()
        .map(|name| name.to_lowercase())
        .collect();
    names.extend(
        config
            .extra_ignored_dirs
            .iter()
            .map(|name| name.to_lowercase()),
    );
    names.extend(
        codepack_scanner::merged_extra_ignored_dirs(&codepack_scanner::detect_stacks(root))
            .into_iter()
            .map(|name| name.to_lowercase()),
    );
    names
}

/// True when any path segment below `root` is an ignored directory name.
fn is_ignored(path: &Path, root: &Path, ignored: &[String]) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        // Outside the watched tree: `notify` can report the root itself on some
        // platforms, and that is never an interesting change.
        return true;
    };
    relative.components().any(|component| {
        let name = component.as_os_str().to_string_lossy().to_lowercase();
        ignored.iter().any(|ignored_name| ignored_name == &name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_inside_an_ignored_directory_is_not_reported() {
        // A dependency install churns thousands of files; reporting them would make the
        // signal useless.
        let root = Path::new("/project");
        let ignored = vec!["node_modules".to_string(), "target".to_string()];

        assert!(is_ignored(
            Path::new("/project/node_modules/react/index.js"),
            root,
            &ignored
        ));
        assert!(is_ignored(
            Path::new("/project/target/debug/app"),
            root,
            &ignored
        ));
    }

    #[test]
    fn a_change_to_a_real_source_file_is_reported() {
        let root = Path::new("/project");
        let ignored = vec!["node_modules".to_string()];
        assert!(!is_ignored(
            Path::new("/project/src/main.rs"),
            root,
            &ignored
        ));
        assert!(!is_ignored(Path::new("/project/README.md"), root, &ignored));
    }

    #[test]
    fn matching_is_case_insensitive_because_two_of_the_three_platforms_are() {
        let root = Path::new("/project");
        let ignored = vec!["node_modules".to_string()];
        assert!(is_ignored(
            Path::new("/project/Node_Modules/pkg/index.js"),
            root,
            &ignored
        ));
    }

    #[test]
    fn a_path_outside_the_watched_tree_is_ignored_rather_than_reported() {
        let root = Path::new("/project");
        assert!(is_ignored(Path::new("/elsewhere/file.rs"), root, &[]));
    }

    #[test]
    fn the_ignore_set_includes_the_scanners_own_defaults() {
        // The watch must agree with the export about what counts as project content.
        let dir = tempfile::tempdir().unwrap();
        let names = ignored_directory_names(dir.path(), &Config::default());
        assert!(names.iter().any(|name| name == "node_modules"));
        assert!(names.iter().any(|name| name == ".git"));
    }

    #[test]
    fn a_user_configured_extra_directory_is_honoured() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            extra_ignored_dirs: vec!["Vendor".to_string()],
            ..Config::default()
        };
        let names = ignored_directory_names(dir.path(), &config);
        assert!(names.iter().any(|name| name == "vendor"));
    }

    #[test]
    fn a_stack_detected_directory_is_honoured() {
        // A Rust project's `target` is not listed in the base defaults; the stack
        // detector is what adds it.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        let names = ignored_directory_names(dir.path(), &Config::default());
        assert!(
            names.iter().any(|name| name == "target"),
            "stack-detected directories are missing: {names:?}"
        );
    }
}
