//! Process-wide state: the running exports, and the watcher.
//!
//! An export runs on a background thread for as long as it takes, while the UI stays
//! live. That means two things must be reachable from a later command: the token that
//! cancels the run, and the staging directory it is building. Both live here, keyed by
//! the run id the frontend was handed when it started the export.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use codepack_core::CancellationToken;

/// Locks `mutex`, taking the data back even if a previous holder panicked.
///
/// Every lock here used to be `.expect("… is never poisoned")`. That message asserted an
/// invariant nobody had proved: a mutex is poisoned when a thread panics *while holding
/// it*, and the code inside these sections touches a `HashMap` and a
/// `Box<dyn Any + Send>` — nothing that cannot panic in principle. One such panic would
/// have made every later `lock()` return `Err`, so the `expect` would fire
/// unconditionally from then on and the run registry would be dead for the rest of the
/// process: no starting an export, no cancelling one, no closing the window cleanly.
/// The user's only cure would be restarting the app, with no clue why.
///
/// Ignoring the poison is right *for this data specifically*, and that is the argument
/// `.ai/project/12-domain-rules.md` asks for rather than an assertion. What these
/// mutexes guard is a lookup table of live runs and one optional watcher handle. A panic
/// elsewhere cannot leave either in a state that is *wrong* — only, at worst, missing an
/// insert or a removal that the panicking operation was about to make. A stale entry is
/// already an ordinary case the registry handles (`cancel` on a finished run is
/// documented as harmless), so recovering and carrying on is strictly better than
/// bringing down every subsequent export.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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
            let mut next = lock(&self.next_id);
            *next += 1;
            format!("run-{next}")
        };
        lock(&self.runs).insert(
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
        if let Some(run) = lock(&self.runs).get(run_id) {
            run.cancel.cancel();
        }
    }

    /// Removes a finished run. Called on every exit path, including the error one.
    pub fn finish(&self, run_id: &str) {
        lock(&self.runs).remove(run_id);
    }

    pub fn is_running(&self, run_id: &str) -> bool {
        lock(&self.runs).contains_key(run_id)
    }

    /// Cancels every in-flight run. Used when the window closes: a background export
    /// holding a half-built staging directory must be told to stop, not left orphaned.
    pub fn cancel_all(&self) {
        for run in lock(&self.runs).values() {
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
        *lock(&self.watcher) = Some(watcher);
    }

    pub fn clear(&self) {
        *lock(&self.watcher) = None;
    }

    pub fn is_watching(&self) -> bool {
        lock(&self.watcher).is_some()
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
    fn the_registry_still_works_after_a_thread_panics_holding_the_lock() {
        // The defect this replaced: with `.expect("never poisoned")`, one panic inside
        // any critical section killed the registry for the whole process — every later
        // start/cancel/close panicked too, and the user's only cure was restarting the
        // app. Poisoning it deliberately here is the only way to prove the recovery is
        // real rather than asserted.
        let registry = Arc::new(RunRegistry::default());
        let (existing_id, existing_token) = registry.start();

        let poisoner = Arc::clone(&registry);
        let result = std::thread::spawn(move || {
            let _guard = lock(&poisoner.runs);
            panic!("simulated panic while holding the registry lock");
        })
        .join();
        assert!(
            result.is_err(),
            "the helper thread must actually have panicked"
        );

        // Everything the app needs must still work.
        assert!(registry.is_running(&existing_id));
        let (new_id, new_token) = registry.start();
        assert_ne!(new_id, existing_id);

        registry.cancel_all();
        assert!(existing_token.is_cancelled());
        assert!(new_token.is_cancelled());

        registry.finish(&existing_id);
        assert!(!registry.is_running(&existing_id));
    }

    #[test]
    fn the_watch_state_still_works_after_a_thread_panics_holding_the_lock() {
        let state = Arc::new(WatchState::default());
        state.replace(Box::new(()));

        let poisoner = Arc::clone(&state);
        let result = std::thread::spawn(move || {
            let _guard = lock(&poisoner.watcher);
            panic!("simulated panic while holding the watch lock");
        })
        .join();
        assert!(result.is_err());

        assert!(state.is_watching());
        state.clear();
        assert!(!state.is_watching());
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
