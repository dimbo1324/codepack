//! Process-wide state: the running exports, and the watcher.
//!
//! An export runs on a background thread for as long as it takes, while the UI stays
//! live. That means two things must be reachable from a later command: the token that
//! cancels the run, and the staging directory it is building. Both live here, keyed by
//! the run id the frontend was handed when it started the export.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use codepack_core::CancellationToken;

/// A single in-flight export.
struct ActiveRun {
    cancel: CancellationToken,
}

/// The registry of in-flight exports.
///
/// Keyed by a string id rather than an integer because the frontend receives it before
/// the database has assigned an `export_run.id` — the run does not exist in history
/// until it finishes, but it must be cancellable from the moment it starts.
#[derive(Default)]
pub struct RunRegistry {
    runs: Mutex<HashMap<String, ActiveRun>>,
    next_id: Mutex<u64>,
}

impl RunRegistry {
    /// Registers a new run and returns `(run_id, cancel_token)`.
    pub fn start(&self) -> (String, CancellationToken) {
        let cancel = CancellationToken::new();
        let run_id = {
            // A monotonic counter, not a timestamp: two exports started inside the same
            // second must not collide, and the id is never shown to the user.
            let mut next = self
                .next_id
                .lock()
                .expect("run id counter is never poisoned");
            *next += 1;
            format!("run-{next}")
        };
        self.runs
            .lock()
            .expect("run registry is never poisoned")
            .insert(
                run_id.clone(),
                ActiveRun {
                    cancel: cancel.clone(),
                },
            );
        (run_id, cancel)
    }

    /// Cancels `run_id` if it is still running. A missing id is not an error: the run
    /// may have finished between the user pressing the button and this arriving.
    pub fn cancel(&self, run_id: &str) {
        if let Some(run) = self
            .runs
            .lock()
            .expect("run registry is never poisoned")
            .get(run_id)
        {
            run.cancel.cancel();
        }
    }

    /// Removes a finished run. Called on every exit path, including the error one.
    pub fn finish(&self, run_id: &str) {
        self.runs
            .lock()
            .expect("run registry is never poisoned")
            .remove(run_id);
    }

    pub fn is_running(&self, run_id: &str) -> bool {
        self.runs
            .lock()
            .expect("run registry is never poisoned")
            .contains_key(run_id)
    }

    /// Cancels every in-flight run. Used when the window closes: a background export
    /// holding a half-built staging directory must be told to stop, not left orphaned.
    pub fn cancel_all(&self) {
        for run in self
            .runs
            .lock()
            .expect("run registry is never poisoned")
            .values()
        {
            run.cancel.cancel();
        }
    }
}

/// The active filesystem watcher, if watch mode is on.
///
/// Holds the `notify` watcher alive — dropping it stops the watch, which is exactly what
/// `stop_watch` needs and what must also happen if a second `start_watch` arrives for a
/// different project.
#[derive(Default)]
pub struct WatchState {
    watcher: Mutex<Option<Box<dyn std::any::Any + Send>>>,
}

impl WatchState {
    /// Installs `watcher`, dropping any previous one.
    pub fn replace(&self, watcher: Box<dyn std::any::Any + Send>) {
        *self.watcher.lock().expect("watch state is never poisoned") = Some(watcher);
    }

    pub fn clear(&self) {
        *self.watcher.lock().expect("watch state is never poisoned") = None;
    }

    pub fn is_watching(&self) -> bool {
        self.watcher
            .lock()
            .expect("watch state is never poisoned")
            .is_some()
    }
}

/// Everything a command may reach for, installed once at startup with `app.manage`.
#[derive(Default)]
pub struct AppState {
    pub runs: Arc<RunRegistry>,
    pub watch: Arc<WatchState>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_run_gets_a_distinct_id() {
        let registry = RunRegistry::default();
        let (first, _) = registry.start();
        let (second, _) = registry.start();
        assert_ne!(first, second);
    }

    #[test]
    fn cancelling_a_run_trips_the_token_the_export_thread_is_holding() {
        let registry = RunRegistry::default();
        let (run_id, token) = registry.start();
        assert!(!token.is_cancelled());

        registry.cancel(&run_id);
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancelling_an_unknown_run_is_not_an_error() {
        // The user can press cancel in the instant a run finishes; that race must be
        // harmless rather than a visible failure.
        let registry = RunRegistry::default();
        registry.cancel("run-does-not-exist");
    }

    #[test]
    fn a_finished_run_is_no_longer_cancellable_and_leaves_no_entry() {
        let registry = RunRegistry::default();
        let (run_id, token) = registry.start();
        assert!(registry.is_running(&run_id));

        registry.finish(&run_id);
        assert!(!registry.is_running(&run_id));

        // Cancelling after the fact must not reach the (now detached) token.
        registry.cancel(&run_id);
        assert!(!token.is_cancelled());
    }

    #[test]
    fn cancel_all_stops_every_in_flight_run() {
        // What window-close does: no background export may be left building a staging
        // directory nobody will clean up.
        let registry = RunRegistry::default();
        let (_, first) = registry.start();
        let (_, second) = registry.start();

        registry.cancel_all();

        assert!(first.is_cancelled());
        assert!(second.is_cancelled());
    }

    #[test]
    fn replacing_a_watcher_drops_the_previous_one() {
        // Switching projects must not leave the old project still being watched.
        let state = WatchState::default();
        assert!(!state.is_watching());

        state.replace(Box::new(()));
        assert!(state.is_watching());

        state.replace(Box::new(()));
        assert!(state.is_watching());

        state.clear();
        assert!(!state.is_watching());
    }
}
