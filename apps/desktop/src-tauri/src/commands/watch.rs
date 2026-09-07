//! Watch mode: telling the UI when the project changed underneath it.
//!
//! This does **not** re-export on every keystroke. It emits `watch:changed`, and the UI
//! decides what to do with that — refresh the preview, or (if the user asked for it)
//! update the clipboard. Re-running a full export automatically would write archives
//! nobody asked for, which is the opposite of a tool built around deliberate handoff.
//!
//! Ignored directories are never subscribed to in the first place (audit 2026-09-07,
//! L-2), not merely filtered out of what gets reported. `notify` on Linux implements
//! recursive watching through inotify, which has no concept of recursion at all — the
//! library walks the tree itself and spends one watch descriptor per directory. A
//! recursive subscription on the project root would burn one on `node_modules`,
//! `target`, `.git` and every other directory the scanner already knows to skip, and
//! `/proc/sys/fs/inotify/max_user_watches` is a small, shared, per-user budget (8192 on
//! several distributions' defaults) that every other tool on the machine — an IDE,
//! systemd, a language server — draws from too. So this walks the tree itself, exactly
//! as [`codepack_scanner::walk_project`] does, and subscribes each surviving directory
//! non-recursively; a directory created later is picked up from its parent's `Create`
//! event and subscribed the same way, if it passes the same filter.
//!
//! Running out of descriptors anyway is reported, not swallowed: `notify::ErrorKind::
//! MaxFilesWatch` is `ENOSPC` from inotify specifically, which is not a transient
//! "a directory disappeared mid-walk" the way most watch errors are — it means part of
//! the tree will never be watched, silently, while the UI's indicator still shows the
//! watch as running.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_scanner::IgnoredDirMatcher;
use notify::{Event, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, State};

use crate::dto::{WatchChangedEvent, WatchDegradedEvent};
use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

pub const CHANGED_EVENT: &str = "watch:changed";
pub const DEGRADED_EVENT: &str = "watch:degraded";

/// How long the changes must go quiet before the UI is told.
///
/// Saving a file in an editor produces several events in quick succession (write,
/// rename, attribute change), and a build produces hundreds. Coalescing them into one
/// notification is the difference between a useful signal and a flood.
const DEBOUNCE: Duration = Duration::from_millis(400);

/// Most paths one notification carries.
///
/// A dependency install churns tens of thousands of files. Ignored directories catch most
/// of that, but not a `git checkout` of a large branch — and an unbounded buffer would
/// grow until the quiet arrived. Past this the notification says how many there were
/// rather than which, which is all a person can use at that scale anyway.
const MAX_PENDING_PATHS: usize = 10_000;

/// What has changed since the last notification.
#[derive(Default)]
struct Pending {
    paths: Vec<String>,
    /// True once [`MAX_PENDING_PATHS`] was reached and paths started being dropped, so
    /// the UI can say "too many to list" instead of quietly showing a partial list.
    truncated: bool,
    /// Set when the watch is being torn down, so the aggregator thread ends.
    stopped: bool,
}

/// Collects change notifications and releases them once they go quiet.
///
/// ## Why a thread rather than a timestamp
///
/// This used to be a throttle wearing a debounce's name: the callback emitted if
/// `DEBOUNCE` had passed since the last emission and otherwise dropped the paths into a
/// buffer and returned. Nothing ever drained that buffer on its own — there was no timer
/// — so the accumulated paths waited for the *next* change to push them out, and if none
/// came they waited forever.
///
/// That is the common case, not an edge one: a save produces three or four events in
/// fifty milliseconds, the first is emitted and the rest sit in the buffer. The UI then
/// shows a project state missing the very last thing the user did. It looks like it works
/// almost always, which is what makes it hard to notice (audit No. 17).
///
/// A trailing-edge debounce needs something that wakes up when *nothing* happens, so
/// there is a thread: the callback only records and signals, and the thread waits with a
/// timeout and emits when the quiet actually arrives. It also keeps the file-system
/// callback free of work, which is its own reason.
struct Coalescer {
    state: Arc<(Mutex<Pending>, Condvar)>,
}

impl Coalescer {
    /// Starts the aggregator thread. `emit` is called from it, once per burst.
    fn start(
        emit: impl Fn(Vec<String>, bool) + Send + 'static,
    ) -> (Self, std::thread::JoinHandle<()>) {
        let state = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let thread_state = Arc::clone(&state);

        let handle = std::thread::spawn(move || {
            let (mutex, condvar) = &*thread_state;
            loop {
                let mut pending = lock(mutex);
                // Nothing to do: sleep until something arrives or the watch stops.
                while pending.paths.is_empty() && !pending.stopped {
                    pending = condvar.wait(pending).unwrap_or_else(|e| e.into_inner());
                }
                if pending.stopped {
                    // Whatever arrived in the same instant as the stop is dropped
                    // deliberately: the watch is going away and nobody is listening.
                    return;
                }

                // Something is pending. Wait for the quiet, restarting the wait each time
                // more arrives — this is the trailing edge.
                loop {
                    let before = pending.paths.len();
                    let (next, timeout) = condvar
                        .wait_timeout(pending, DEBOUNCE)
                        .unwrap_or_else(|e| e.into_inner());
                    pending = next;
                    if pending.stopped {
                        return;
                    }
                    if timeout.timed_out() || pending.paths.len() == before {
                        break;
                    }
                }

                let paths = std::mem::take(&mut pending.paths);
                let truncated = std::mem::take(&mut pending.truncated);
                // Released before emitting: the callback must never be blocked behind a
                // consumer, and `emit` reaches Tauri.
                drop(pending);
                if !paths.is_empty() {
                    emit(paths, truncated);
                }
            }
        });

        (Self { state }, handle)
    }

    /// Records paths and wakes the aggregator. Called from the file-system callback, so
    /// it does no work beyond this.
    fn push(&self, paths: impl IntoIterator<Item = String>) {
        let (mutex, condvar) = &*self.state;
        let mut pending = lock(mutex);
        for path in paths {
            if pending.paths.len() >= MAX_PENDING_PATHS {
                pending.truncated = true;
                break;
            }
            pending.paths.push(path);
        }
        condvar.notify_all();
    }

    /// Ends the aggregator thread.
    fn stop(&self) {
        let (mutex, condvar) = &*self.state;
        lock(mutex).stopped = true;
        condvar.notify_all();
    }
}

/// Takes the lock, recovering from poisoning.
///
/// The same argument `state.rs` records for its own mutexes: what this guards is a list
/// of paths to notify about, and a panic elsewhere cannot make it *wrong* — only, at
/// worst, short by one. Turning that into a dead watch for the rest of the process would
/// be strictly worse.
fn lock(mutex: &Mutex<Pending>) -> std::sync::MutexGuard<'_, Pending> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// What `WatchState` holds: the watcher, and the aggregator that belongs to it.
///
/// One value so the two cannot outlive each other. `stop_watch` and closing the window
/// both drop this, and the `Drop` below is what ends the thread — the audit's point that
/// the aggregator must be tied to the same ownership as the watcher itself.
struct ActiveWatch {
    coalescer: Coalescer,
    aggregator: Option<std::thread::JoinHandle<()>>,
    /// This side is never cloned into the file-system callback (see
    /// [`SubscriberCommand`]'s doc comment for why that distinction matters): dropping
    /// it is not what ends [`spawn_subscriber`]'s thread, sending [`SubscriberCommand::
    /// Stop`] through it is.
    commands: Option<std::sync::mpsc::Sender<SubscriberCommand>>,
    /// Owns the actual `notify` watcher. Ending this thread (by sending `Stop` through
    /// `commands` above) is what stops file-system notifications, the same property
    /// `_watcher`'s `Drop` used to give when the watcher was stored directly — see
    /// [`spawn_subscriber`] for why it is not stored directly any more.
    subscriber_thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for ActiveWatch {
    fn drop(&mut self) {
        self.coalescer.stop();
        if let Some(handle) = self.aggregator.take() {
            // Joined rather than detached: a thread still holding an `AppHandle` after the
            // window is gone is how a shutdown turns into a hang.
            let _ = handle.join();
        }
        // A closed channel would also end the subscriber thread's `for` loop, but this
        // side is never the last sender: the file-system callback's own clone lives
        // inside the watcher, which lives inside that same thread, so this side being
        // dropped would never actually bring the sender count to zero — an explicit
        // `Stop` is what ends the loop deterministically instead.
        if let Some(commands) = self.commands.take() {
            let _ = commands.send(SubscriberCommand::Stop);
        }
        if let Some(handle) = self.subscriber_thread.take() {
            let _ = handle.join();
        }
    }
}

/// A request to [`spawn_subscriber`]'s thread, sent either from [`start_watch`] itself
/// (the initial tree) or from the file-system callback (a directory a `Create` event
/// revealed).
///
/// Not a bare `PathBuf` with the channel's own closing as the stop signal: the
/// callback's own clone of the sender lives *inside* the watcher, which lives inside the
/// very thread that would be waiting for every sender to be dropped — a stop condition
/// that thread can only satisfy by first dropping the watcher, which is the one thing
/// still keeping that clone alive. `Stop` breaks that cycle by ending the loop on an
/// explicit message rather than on a sender count this design can never actually bring
/// to zero from the inside.
enum SubscriberCommand {
    Watch(PathBuf),
    Stop,
}

/// Owns the `notify` watcher exclusively and performs every `.watch()` call — the
/// initial tree's remaining directories, then whatever [`start_watch`]'s callback sends
/// as `Create` events reveal new ones — so the callback itself never needs a handle back
/// to the watcher it belongs to.
///
/// That "back to itself" shape was the alternative design, and it was rejected on
/// purpose: `notify::recommended_watcher` takes ownership of the callback before
/// returning the watcher, so making the callback able to call `.watch()` on that same
/// watcher needs either `Arc::new_cyclic` (and `notify::recommended_watcher` is
/// fallible, which does not fit that constructor's signature) or an `Arc<Mutex<Option<
/// Watcher>>>` the callback and the watcher's owner both hold strong references to —
/// which is a genuine reference cycle: the watcher owns the callback, and the callback
/// would hold a strong `Arc` back to a cell that owns the watcher. `Arc` cycles are not
/// collected in Rust; every watch session started and stopped over a long-running
/// desktop process would leak one. A channel has no such cycle: the callback holds only
/// a `Sender`, this thread holds the watcher and the matching `Receiver`, and dropping
/// the `Sender` (in `ActiveWatch::drop`) ends the thread and its watcher cleanly.
fn spawn_subscriber(
    mut watcher: notify::RecommendedWatcher,
    initial: Vec<PathBuf>,
    commands: std::sync::mpsc::Receiver<SubscriberCommand>,
    app: AppHandle,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut degraded_reported = false;
        let initial = initial.into_iter().map(SubscriberCommand::Watch);
        for command in initial.chain(commands) {
            let path = match command {
                SubscriberCommand::Watch(path) => path,
                // Ends the loop deterministically — see `SubscriberCommand`'s doc
                // comment for why the channel closing on its own is not this thread's
                // stop signal.
                SubscriberCommand::Stop => break,
            };
            if degraded_reported {
                // The kernel-wide limit was already hit once; every further `.watch()`
                // call fails the identical way until something elsewhere frees
                // descriptors. Trying anyway would spend a syscall per discovered
                // directory for the rest of the session to learn the same fact again.
                continue;
            }
            if let Err(error) = watcher.watch(&path, RecursiveMode::NonRecursive)
                && matches!(error.kind, notify::ErrorKind::MaxFilesWatch)
            {
                degraded_reported = true;
                let _ = app.emit(
                    DEGRADED_EVENT,
                    WatchDegradedEvent {
                        directory: Some(path.display().to_string()),
                    },
                );
            }
            // Any other error (the directory disappeared between being discovered and
            // being subscribed to, a permission change) is a transient race on that one
            // directory, not a reason to stop watching everything else.
        }
        // Drops `watcher` — and with it the callback and the sender clone the callback
        // held — which is what actually ends the file-system subscription.
        drop(watcher);
    })
}

/// Starts watching `project_root`. Replaces any previous watch.
///
/// Subscribes non-recursively to `root` and every surviving subdirectory
/// [`codepack_scanner::watched_directories`] finds (audit 2026-09-07, L-2) — never to
/// `node_modules`, `target`, `.git` and the rest of what the scanner already prunes, so
/// this does not spend the OS's small, shared inotify budget on directories nobody
/// wants watched. Only `root` itself is subscribed to synchronously, so a genuine
/// failure there (the directory vanished, permission denied) still surfaces as this
/// command's own error exactly as before; every other directory — the rest of the
/// initial tree, and any created afterward — is subscribed to from a background thread
/// (see [`spawn_subscriber`]), and running out of watch descriptors partway through is
/// reported as a `watch:degraded` event rather than silently leaving part of the
/// project unwatched.
#[tauri::command]
pub fn start_watch(
    app: AppHandle,
    state: State<'_, AppState>,
    project_root: String,
    config: Config,
) -> CommandResult<()> {
    let root = super::resolve_project_root(&project_root)?;
    let matcher = ignored_dir_matcher(&root, &config);

    let mut directories =
        codepack_scanner::watched_directories(&root, &matcher, &CancellationToken::new())
            .map_err(CommandError::new)?;
    // `watched_directories` always returns `root` first; watched synchronously below,
    // it must not also be sent down the channel `spawn_subscriber` drains.
    if !directories.is_empty() {
        directories.remove(0);
    }

    let watch_root = root.clone();
    let coalescer_app = app.clone();
    let callback_app = app.clone();

    let (coalescer, aggregator) = Coalescer::start(move |changed_paths, truncated| {
        let _ = coalescer_app.emit(
            CHANGED_EVENT,
            WatchChangedEvent {
                changed_paths,
                truncated,
            },
        );
    });

    let sink = Coalescer {
        state: Arc::clone(&coalescer.state),
    };
    let (commands_tx, commands_rx) = std::sync::mpsc::channel::<SubscriberCommand>();
    // `ActiveWatch`'s own handle, kept apart from the clone the callback below moves
    // in: see `SubscriberCommand`'s doc comment for why this side must never be the
    // sender the subscriber thread's loop is implicitly waiting to see dropped.
    let active_watch_commands = commands_tx.clone();
    let callback_matcher = matcher.clone();
    let mut stream_error_reported = false;
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
        let event = match result {
            Ok(event) => event,
            Err(error) => {
                if matches!(error.kind, notify::ErrorKind::MaxFilesWatch) && !stream_error_reported
                {
                    stream_error_reported = true;
                    let _ =
                        callback_app.emit(DEGRADED_EVENT, WatchDegradedEvent { directory: None });
                }
                // Any other error (a directory disappearing mid-walk, a permission
                // change) is not worth interrupting the user over: the watch keeps
                // running, and the next real change still reports.
                return;
            }
        };
        if !event.kind.is_create() && !event.kind.is_modify() && !event.kind.is_remove() {
            return;
        }

        if event.kind.is_create() {
            for path in &event.paths {
                let Ok(relative) = path.strip_prefix(&watch_root) else {
                    continue;
                };
                if path.is_dir() && !callback_matcher.is_ignored(relative) {
                    // The subscriber thread, not this callback, actually calls
                    // `.watch()` — see `spawn_subscriber`'s doc comment for why a
                    // callback holding a handle back to its own watcher is a reference
                    // cycle this design avoids on purpose. A closed receiver (the watch
                    // was already stopped) makes this a no-op, which is correct.
                    let _ = commands_tx.send(SubscriberCommand::Watch(path.clone()));
                }
            }
        }

        // Recorded, not emitted. Deciding when to speak is the aggregator's job, and a
        // file-system callback should return promptly.
        sink.push(
            event
                .paths
                .iter()
                .filter(|path| is_reportable(path, &watch_root, &callback_matcher))
                .map(|path| path.display().to_string()),
        );
    })
    .map_err(CommandError::new)?;

    watcher
        .watch(&root, RecursiveMode::NonRecursive)
        .map_err(CommandError::new)?;

    let subscriber_thread = spawn_subscriber(watcher, directories, commands_rx, app);

    state.watch.replace(Box::new(ActiveWatch {
        coalescer,
        aggregator: Some(aggregator),
        commands: Some(active_watch_commands),
        subscriber_thread: Some(subscriber_thread),
    }));
    Ok(())
}

/// Stops watching. Idempotent: stopping when nothing is watched is not an error.
#[tauri::command]
pub fn stop_watch(state: State<'_, AppState>) -> CommandResult<()> {
    state.watch.clear();
    Ok(())
}

/// The same directory-name rules the scanner prunes with, so the watch agrees with the
/// export about what counts as part of the project — and, since audit 2026-09-07 L-2,
/// so [`codepack_scanner::watched_directories`] and the callback in [`start_watch`]
/// answer "is this directory worth watching" identically. `IgnoredDirMatcher` already
/// includes `codepack_scanner::IGNORED_DIR_NAMES` internally; only the config- and
/// stack-detected extras need to be supplied here.
fn ignored_dir_matcher(root: &Path, config: &Config) -> IgnoredDirMatcher {
    let mut extra: Vec<String> = config.extra_ignored_dirs.clone();
    extra.extend(codepack_scanner::merged_extra_ignored_dirs(
        &codepack_scanner::detect_stacks(root),
    ));
    IgnoredDirMatcher::new(extra)
}

/// True when a changed path is worth telling the UI about.
///
/// With subscriptions now non-recursive and scoped to surviving directories only (audit
/// 2026-09-07, L-2), `notify` structurally cannot report an event for a path nested
/// inside an ignored directory — nothing was ever watching in there. What remains to
/// filter here is narrower: an event for the ignored directory's *own* creation (a
/// `node_modules` a fresh `npm install` just created is real, but not something a user
/// asked to be told about), and a path `notify` reports outside the watched tree
/// entirely, which some platforms do for the root itself.
fn is_reportable(path: &Path, root: &Path, matcher: &IgnoredDirMatcher) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    !matcher.is_ignored(relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- The debounce (audit No. 17) --------------------------------------------------
    //
    // The old code was a leading-edge throttle: it emitted the first event of a burst and
    // left the rest in a buffer nothing drained. There was no test, which is why the
    // defect survived — so these describe the behaviour rather than the implementation.

    /// One burst of N changes must produce exactly one notification carrying all N.
    #[test]
    fn a_burst_produces_one_notification_carrying_every_path() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorder = Arc::clone(&seen);
        let (coalescer, aggregator) = Coalescer::start(move |paths, truncated| {
            recorder
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((paths, truncated));
        });

        // What an editor save looks like: several events in immediate succession. Pushed
        // without sleeping between them on purpose — a sleep here would make the test
        // assert that the machine scheduled two threads within the debounce window, which
        // a loaded CI box does not promise. The gap between bursts is what
        // `two_separated_bursts_are_two_notifications` covers, and it uses a margin
        // several times the window.
        for index in 0..4 {
            coalescer.push([format!("/project/src/file{index}.rs")]);
        }

        // Past the quiet period, with room for the thread to be scheduled.
        std::thread::sleep(DEBOUNCE * 3);
        coalescer.stop();
        let _ = aggregator.join();

        let seen = seen.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(seen.len(), 1, "one burst is one notification: {seen:?}");
        assert_eq!(seen[0].0.len(), 4, "every path in the burst: {seen:?}");
        assert!(!seen[0].1, "nothing was dropped");
    }

    /// The defect itself: the *last* change of a burst must arrive even when nothing
    /// follows it. The old throttle left it in a buffer until the next change, which
    /// might never come.
    #[test]
    fn the_final_change_of_a_burst_is_not_left_behind() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorder = Arc::clone(&seen);
        let (coalescer, aggregator) = Coalescer::start(move |paths, _| {
            recorder
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend(paths);
        });

        coalescer.push(["/project/first.rs".to_string()]);
        coalescer.push(["/project/last.rs".to_string()]);

        std::thread::sleep(DEBOUNCE * 3);
        coalescer.stop();
        let _ = aggregator.join();

        let seen = seen.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            seen.iter().any(|path| path.ends_with("last.rs")),
            "the last change of a burst never reached the UI: {seen:?}"
        );
    }

    /// Two bursts separated by more than the quiet period are two notifications, not one
    /// — otherwise the debounce would swallow a genuinely separate edit.
    #[test]
    fn two_separated_bursts_are_two_notifications() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorder = Arc::clone(&seen);
        let (coalescer, aggregator) = Coalescer::start(move |paths, _| {
            recorder
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(paths);
        });

        coalescer.push(["/project/a.rs".to_string()]);
        std::thread::sleep(DEBOUNCE * 3);
        coalescer.push(["/project/b.rs".to_string()]);
        std::thread::sleep(DEBOUNCE * 3);

        coalescer.stop();
        let _ = aggregator.join();

        let seen = seen.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(seen.len(), 2, "{seen:?}");
    }

    /// A checkout of a large branch must not grow the buffer without limit; past the cap
    /// the notification says so rather than showing a partial list as if it were whole.
    #[test]
    fn an_enormous_burst_is_capped_and_says_that_it_was() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorder = Arc::clone(&seen);
        let (coalescer, aggregator) = Coalescer::start(move |paths, truncated| {
            recorder
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((paths.len(), truncated));
        });

        coalescer.push((0..MAX_PENDING_PATHS + 500).map(|index| format!("/project/f{index}")));

        std::thread::sleep(DEBOUNCE * 3);
        coalescer.stop();
        let _ = aggregator.join();

        let seen = seen.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, MAX_PENDING_PATHS);
        assert!(seen[0].1, "the truncation must be reported, not hidden");
    }

    /// Stopping ends the aggregator thread. `ActiveWatch::drop` relies on this: a thread
    /// still holding an `AppHandle` after the window closes is how shutdown hangs.
    #[test]
    fn stopping_ends_the_aggregator_thread() {
        let (coalescer, aggregator) = Coalescer::start(|_, _| {});
        coalescer.push(["/project/a.rs".to_string()]);
        coalescer.stop();

        // `join` returning at all is the assertion; a leaked thread would hang here.
        assert!(aggregator.join().is_ok());
    }

    // With subscriptions non-recursive and scoped to survivor directories (audit
    // 2026-09-07, L-2), `notify` cannot structurally deliver an event for a path nested
    // inside an ignored directory — nothing was ever watching there. What `is_reportable`
    // still has to filter is narrower: an ignored directory's *own* creation event
    // (reported at the level of its already-watched parent), and a path outside the
    // watched tree entirely.

    #[test]
    fn an_ignored_directorys_own_creation_is_not_reported() {
        // A dependency install churns thousands of files inside it; reporting even its
        // own creation would be the first line of a flood.
        let root = Path::new("/project");
        let matcher = IgnoredDirMatcher::new(["target".to_string()]);

        assert!(!is_reportable(
            Path::new("/project/node_modules"),
            root,
            &matcher
        ));
        assert!(!is_reportable(Path::new("/project/target"), root, &matcher));
    }

    #[test]
    fn a_change_to_a_real_source_file_is_reported() {
        let root = Path::new("/project");
        let matcher = IgnoredDirMatcher::new(std::iter::empty());
        assert!(is_reportable(
            Path::new("/project/src/main.rs"),
            root,
            &matcher
        ));
        assert!(is_reportable(
            Path::new("/project/README.md"),
            root,
            &matcher
        ));
    }

    #[test]
    fn matching_is_case_insensitive_because_two_of_the_three_platforms_are() {
        let root = Path::new("/project");
        let matcher = IgnoredDirMatcher::new(std::iter::empty());
        assert!(!is_reportable(
            Path::new("/project/Node_Modules"),
            root,
            &matcher
        ));
    }

    #[test]
    fn a_path_outside_the_watched_tree_is_not_reported() {
        let root = Path::new("/project");
        let matcher = IgnoredDirMatcher::new(std::iter::empty());
        assert!(!is_reportable(
            Path::new("/elsewhere/file.rs"),
            root,
            &matcher
        ));
    }

    #[test]
    fn the_matcher_includes_the_scanners_own_defaults() {
        // The watch must agree with the export about what counts as project content.
        let dir = tempfile::tempdir().unwrap();
        let matcher = ignored_dir_matcher(dir.path(), &Config::default());
        assert!(matcher.is_ignored(Path::new("node_modules")));
        assert!(matcher.is_ignored(Path::new(".git")));
    }

    #[test]
    fn a_user_configured_extra_directory_is_honoured() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            extra_ignored_dirs: vec!["Vendor".to_string()],
            ..Config::default()
        };
        let matcher = ignored_dir_matcher(dir.path(), &config);
        assert!(matcher.is_ignored(Path::new("vendor")));
    }

    #[test]
    fn a_stack_detected_directory_is_honoured() {
        // A Rust project's `target` is not listed in the base defaults; the stack
        // detector is what adds it.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        let matcher = ignored_dir_matcher(dir.path(), &Config::default());
        assert!(
            matcher.is_ignored(Path::new("target")),
            "stack-detected directories are not ignored"
        );
    }
}
