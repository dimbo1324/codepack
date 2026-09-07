//! The application's own activity log: what an export did, what it skipped and why,
//! and what went wrong (audit 2026-09-07, G-1). This is not a report and not the
//! progress a user watches live — it exists so a "why isn't this file in my bundle" or
//! "the app disappeared" question has an answer after the window that would have shown
//! it is long closed.
//!
//! ## Why this is a `LogSink` subscribing to the existing channel, not a new facade
//!
//! `tracing`/`tracing-subscriber`/`tracing-appender` were the audit's own suggested
//! library, for the spans they would give `run=`/`step=` context automatically. They
//! were set aside here: this pipeline already threads that context by hand through
//! [`ProgressEvent`] (`StepStarted { step }`, and so on), not through spans, so adopting
//! `tracing` now would mean either running two parallel context systems or replacing the
//! existing channel outright — a materially larger, riskier change than what the audit
//! itself frames this feature as: "the channel already exists, the events are already
//! structured; subscribing a file consumer to it is an addition, not a rewrite." This
//! module is that addition, and the hand-rolled form the audit names as its own
//! minimalist alternative.
//!
//! No domain crate changes: every event this writes already flows through the channel
//! `codepack-engine` itself emits; this module only reads it. `codepack-cli` and
//! `codepack-desktop` both already drain that channel for their own purposes (stderr, the
//! webview) — a [`LogSink`] is one more consumer of the same stream, run from the same
//! place those consumers already are, which is why this lives in `codepack-engine`
//! rather than being duplicated once per shell: both shells already depend on it, and it
//! already depends on `codepack-core` (for [`codepack_core::progress`] and
//! [`codepack_core::paths::AppPaths::log_dir`]) and `codepack-security` (for redaction).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use codepack_core::time::UtcDateTime;
use codepack_core::{LogLevel, ProgressEvent};

const BYTES_PER_MEBIBYTE: u64 = 1024 * 1024;

/// A log message that has already been redacted — the only thing [`LogSink`] ever
/// writes, and [`LogLine::of`] the only way to build one. A private field makes
/// "something logged a raw, unredacted string" a compile error rather than a discipline
/// lapse to remember — the project's own established shape for exactly this problem
/// (`RenderedError` in `codepack-cli/src/main.rs` is the sibling instance): invariant I3
/// forbids a secret's value from reaching a log, and this is what makes that true by
/// construction rather than by convention.
#[derive(Debug, Clone)]
pub struct LogLine(String);

impl LogLine {
    /// Redacts with [`codepack_security::redact_secrets`] — the narrow, keyword-driven
    /// form, not `redacted_line`'s wider one. The same choice `codepack-cli/src/main.rs`
    /// makes and explains for its own error rendering: this text is the pipeline's own
    /// prose and file paths, not someone else's source code, and the wide redactor would
    /// turn ordinary words in that prose into `<REDACTED>`.
    pub fn of(message: impl AsRef<str>) -> Self {
        Self(codepack_security::redact_secrets(message.as_ref()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// How much of what the progress channel carries reaches the file. Mirrors
/// [`codepack_core::progress::LogLevel`]'s four levels, applied per the audit's own
/// table: `DEBUG` is the per-file narration ("copied: …"), off by default so a run's log
/// is on the order of a hundred lines rather than one per file; `WARN` is a skip with a
/// reason; `ERROR` is something that did not work; `INFO` is everything else (step
/// boundaries, run configuration, final counts).
fn level_allowed(level: LogLevel, verbose: bool) -> bool {
    match level {
        LogLevel::Debug => verbose,
        LogLevel::Info | LogLevel::Warn | LogLevel::Error => true,
    }
}

fn level_label(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    }
}

struct OpenFile {
    /// The calendar day this file was opened for, so a write past midnight starts the
    /// next day's file rather than silently continuing yesterday's.
    day: (i64, u32, u32),
    /// Which `.N` suffix is currently open — `0` for the bare `codepack-YYYY-MM-DD.log`.
    part: u32,
    file: File,
    bytes_written: u64,
}

struct SinkState {
    dir: PathBuf,
    verbose: bool,
    max_file_bytes: u64,
    open: Option<OpenFile>,
}

/// Writes [`LogLine`]s to `AppPaths::log_dir()`, one file per calendar day
/// (`codepack-YYYY-MM-DD.log`, continuing as `.1`, `.2`, … past `max_file_mb`), gated by
/// level and tagged with a `run=` identifier so two concurrent runs (P-6) do not
/// interleave unreadably in one file.
///
/// A single [`Mutex`] around all mutable state rather than one lock per field: every
/// write already goes through one thread in each shell (the same thread that already
/// drains the progress channel for its other consumers), so contention is not a
/// concern, and a torn day-rollover-plus-write is a correctness bug a finer lock would
/// not prevent anyway.
pub struct LogSink {
    state: Mutex<SinkState>,
}

impl LogSink {
    /// Opens (creating if needed) the log directory and sweeps files past
    /// `retention_days`/`total_cap_mb` — done at construction, which every shell calls
    /// once at startup, so the sweep happens before anything is written for the day
    /// rather than only when someone remembers to run it.
    pub fn open(
        dir: &Path,
        max_file_mb: u32,
        retention_days: u32,
        total_cap_mb: u32,
    ) -> std::io::Result<Self> {
        fs::create_dir_all(dir)?;
        sweep(dir, retention_days, total_cap_mb)?;
        Ok(Self {
            state: Mutex::new(SinkState {
                dir: dir.to_path_buf(),
                verbose: false,
                max_file_bytes: u64::from(max_file_mb) * BYTES_PER_MEBIBYTE,
                open: None,
            }),
        })
    }

    /// `verbose` mirrors `Config::log_verbose`/`CODEPACK_LOG=debug` — the caller
    /// resolves which one wins (the environment variable does, being the more
    /// deliberate per-invocation choice) and passes the single answer here.
    pub fn set_verbose(&self, verbose: bool) {
        let mut state = lock(&self.state);
        state.verbose = verbose;
    }

    /// Translates one [`ProgressEvent`] into zero or one log line, tagged with `run_id`.
    /// A step-boundary event becomes an `INFO` line naming the step; a [`ProgressEvent::
    /// StepProgress`] is not written at all — it exists for a progress bar, not a
    /// narrative, and would be the single largest source of noise in the file if it
    /// were.
    pub fn record(&self, run_id: &str, event: &ProgressEvent) {
        match event {
            ProgressEvent::Log(log) => {
                self.write(run_id, log.level, None, LogLine::of(&log.message));
            }
            ProgressEvent::StepStarted { step } => {
                self.write(
                    run_id,
                    LogLevel::Info,
                    Some(step),
                    LogLine::of(format!("{step}: started")),
                );
            }
            ProgressEvent::StepFinished { step } => {
                self.write(
                    run_id,
                    LogLevel::Info,
                    Some(step),
                    LogLine::of(format!("{step}: finished")),
                );
            }
            ProgressEvent::StepProgress { .. } => {}
        }
    }

    fn write(&self, run_id: &str, level: LogLevel, step: Option<&str>, line: LogLine) {
        let mut state = lock(&self.state);
        if !level_allowed(level, state.verbose) {
            return;
        }
        let now = UtcDateTime::now();
        let rendered = match step {
            Some(step) => format!(
                "{} {:<5} run={run_id} step={step:<12} {}\n",
                now.format_iso8601_utc(),
                level_label(level),
                line.as_str()
            ),
            None => format!(
                "{} {:<5} run={run_id} {}\n",
                now.format_iso8601_utc(),
                level_label(level),
                line.as_str()
            ),
        };
        if state.ensure_file_for(&now).is_ok() {
            state.append(&rendered);
        }
        // A log write that fails (disk full, permissions) is deliberately not
        // propagated as an error: the pipeline it is narrating must keep running
        // whether or not its own narration could be written down.
    }
}

impl SinkState {
    fn ensure_file_for(&mut self, now: &UtcDateTime) -> std::io::Result<()> {
        let day = (now.year, now.month, now.day);
        let needs_new = match &self.open {
            Some(open) => open.day != day || open.bytes_written >= self.max_file_bytes,
            None => true,
        };
        if !needs_new {
            return Ok(());
        }
        let part = match &self.open {
            Some(open) if open.day == day => open.part + 1,
            _ => 0,
        };
        let path = day_file_path(&self.dir, day, part);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let bytes_written = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        self.open = Some(OpenFile {
            day,
            part,
            file,
            bytes_written,
        });
        Ok(())
    }

    fn append(&mut self, rendered: &str) {
        if let Some(open) = &mut self.open {
            // Best effort, matching `write`'s own reasoning: the pipeline this narrates
            // must not fail because its own diary could not be updated.
            if open.file.write_all(rendered.as_bytes()).is_ok() {
                open.bytes_written += rendered.len() as u64;
            }
        }
    }
}

/// Installs a panic hook that writes the panic's message and location to
/// `log_dir/codepack-panics.log` before running the previous hook (Rust's own default,
/// unless something upstream already replaced it — either way, stderr still gets the
/// panic exactly as it would have without this).
///
/// Audit 2026-09-07, G-1's minimum ask for step 3 ("what went wrong"): `[profile.
/// release] strip = "symbols"` means a release panic already carries no useful stack,
/// and on Windows `windows_subsystem = "windows"` hides the console the message would
/// otherwise have appeared on — a crash in a release build was, before this, completely
/// silent. This does not restore the stack trace (that needs `split-debuginfo` and a
/// symbol-publishing channel, a bigger piece of work the audit records as needing S-6
/// first); it only makes sure the fact of a panic, its message, and its source location
/// survive the process that produced them.
///
/// Deliberately its own small append, not a full [`LogSink`]: a panic can happen before
/// any `LogSink` exists, during one's own construction, or with this process's memory
/// or locks in a state a `Mutex`-guarded, rotating writer should not be trusted to
/// negotiate. One flat file, opened fresh and appended to in a handful of lines, is the
/// form most likely to still work at the moment it is needed.
pub fn install_panic_hook(log_dir: &Path) {
    let log_dir = log_dir.to_path_buf();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let now = UtcDateTime::now();
        let location = info
            .location()
            .map(|location| location.to_string())
            .unwrap_or_else(|| "unknown location".to_string());
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|value| (*value).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(panic payload was not a string)".to_string());
        let line = format!(
            "{} PANIC {}\n",
            now.format_iso8601_utc(),
            LogLine::of(format!("{message} at {location}")).as_str()
        );
        if fs::create_dir_all(&log_dir).is_ok()
            && let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_dir.join("codepack-panics.log"))
        {
            let _ = file.write_all(line.as_bytes());
        }
        previous(info);
    }));
}

fn day_file_path(dir: &Path, (year, month, day): (i64, u32, u32), part: u32) -> PathBuf {
    if part == 0 {
        dir.join(format!("codepack-{year:04}-{month:02}-{day:02}.log"))
    } else {
        dir.join(format!("codepack-{year:04}-{month:02}-{day:02}.log.{part}"))
    }
}

fn lock(mutex: &Mutex<SinkState>) -> std::sync::MutexGuard<'_, SinkState> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Deletes log files past `retention_days` (a file's own mtime, not the date in its
/// name, so a file copied or restored still ages out honestly), then — regardless of
/// age — deletes the oldest survivors until the total is under `total_cap_mb`.
/// `retention_days == 0` deletes every `codepack-*.log*` file: "keep none at all" has to
/// be reachable, not merely "keep very little".
fn sweep(dir: &Path, retention_days: u32, total_cap_mb: u32) -> std::io::Result<()> {
    let mut entries: Vec<(PathBuf, std::time::SystemTime, u64)> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("codepack-") || !name.contains(".log") {
            continue;
        }
        let metadata = entry.metadata()?;
        let modified = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        entries.push((entry.path(), modified, metadata.len()));
    }

    let now = std::time::SystemTime::now();
    let retention = std::time::Duration::from_secs(u64::from(retention_days) * 24 * 60 * 60);
    entries.retain(|(path, modified, _)| {
        let age = now.duration_since(*modified).unwrap_or_default();
        let expired = retention_days == 0 || age > retention;
        if expired {
            let _ = fs::remove_file(path);
        }
        !expired
    });

    entries.sort_by_key(|(_, modified, _)| *modified);
    let cap_bytes = u64::from(total_cap_mb) * BYTES_PER_MEBIBYTE;
    let mut total: u64 = entries.iter().map(|(_, _, size)| size).sum();
    let mut index = 0;
    while total > cap_bytes && index < entries.len() {
        let (path, _, size) = &entries[index];
        if fs::remove_file(path).is_ok() {
            total = total.saturating_sub(*size);
        }
        index += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_core::LogEvent;

    fn no_log_at(dir: &Path) -> LogSink {
        LogSink::open(dir, 50, 14, 200).unwrap()
    }

    fn read_log(dir: &Path) -> String {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("codepack-"))
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        entries
            .into_iter()
            .map(|entry| fs::read_to_string(entry.path()).unwrap())
            .collect()
    }

    // --- Redaction (audit 2026-09-07, G-1's central requirement) -----------------------

    #[test]
    fn a_planted_secret_never_reaches_the_log_file() {
        let dir = tempfile::tempdir().unwrap();
        let sink = no_log_at(dir.path());
        sink.record(
            "r-test",
            &ProgressEvent::Log(LogEvent {
                level: LogLevel::Info,
                message: "found API_KEY=sk-live-abcdef0123456789abcdef0123456789".to_string(),
            }),
        );
        let contents = read_log(dir.path());
        assert!(
            !contents.contains("sk-live-abcdef0123456789abcdef0123456789"),
            "the secret value reached the log file: {contents}"
        );
        assert!(contents.contains("<REDACTED>"), "{contents}");
    }

    #[test]
    fn ordinary_product_prose_is_not_mistaken_for_a_secret() {
        // The reason the narrow redactor was chosen over the wide one: this exact
        // sentence must survive intact.
        let dir = tempfile::tempdir().unwrap();
        let sink = no_log_at(dir.path());
        sink.record(
            "r-test",
            &ProgressEvent::Log(LogEvent {
                level: LogLevel::Info,
                message: "pass --hook to install the pre-commit hook".to_string(),
            }),
        );
        let contents = read_log(dir.path());
        assert!(contents.contains("pass --hook to install the pre-commit hook"));
    }

    // --- Levels --------------------------------------------------------------------

    #[test]
    fn debug_is_dropped_by_default_and_kept_when_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let sink = no_log_at(dir.path());
        sink.record(
            "r-test",
            &ProgressEvent::Log(LogEvent {
                level: LogLevel::Debug,
                message: "copied: src/main.rs".to_string(),
            }),
        );
        assert!(!read_log(dir.path()).contains("copied: src/main.rs"));

        sink.set_verbose(true);
        sink.record(
            "r-test",
            &ProgressEvent::Log(LogEvent {
                level: LogLevel::Debug,
                message: "copied: src/lib.rs".to_string(),
            }),
        );
        assert!(read_log(dir.path()).contains("copied: src/lib.rs"));
    }

    #[test]
    fn warn_and_error_are_never_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let sink = no_log_at(dir.path());
        sink.record(
            "r-test",
            &ProgressEvent::Log(LogEvent {
                level: LogLevel::Warn,
                message: "skipped by safety mode: .env".to_string(),
            }),
        );
        sink.record(
            "r-test",
            &ProgressEvent::Log(LogEvent {
                level: LogLevel::Error,
                message: "cannot write part 2: No space left on device".to_string(),
            }),
        );
        let contents = read_log(dir.path());
        assert!(contents.contains("WARN"));
        assert!(contents.contains("ERROR"));
    }

    #[test]
    fn step_progress_events_write_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let sink = no_log_at(dir.path());
        sink.record(
            "r-test",
            &ProgressEvent::StepProgress {
                step: "copy".to_string(),
                current: 5,
                total: Some(10),
            },
        );
        assert!(read_log(dir.path()).is_empty());
    }

    #[test]
    fn step_boundaries_name_the_step_and_carry_the_run_id() {
        let dir = tempfile::tempdir().unwrap();
        let sink = no_log_at(dir.path());
        sink.record(
            "r-abc123",
            &ProgressEvent::StepStarted {
                step: "2/8: copy".to_string(),
            },
        );
        let contents = read_log(dir.path());
        assert!(contents.contains("run=r-abc123"));
        assert!(contents.contains("step=2/8: copy"));
        assert!(contents.contains("started"));
    }

    #[test]
    fn every_line_carries_an_rfc3339_utc_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let sink = no_log_at(dir.path());
        sink.record(
            "r-test",
            &ProgressEvent::Log(LogEvent {
                level: LogLevel::Info,
                message: "hello".to_string(),
            }),
        );
        let contents = read_log(dir.path());
        let year = UtcDateTime::now().year;
        assert!(
            contents.contains(&format!("{year:04}-"))
                && contents.contains('T')
                && contents.contains('Z'),
            "{contents}"
        );
    }

    // --- Rotation and retention --------------------------------------------------------

    #[test]
    fn a_file_past_the_size_ceiling_continues_as_dot_one() {
        let dir = tempfile::tempdir().unwrap();
        // A 1-byte-per-line effective ceiling forces every write into its own file.
        let sink = LogSink::open(dir.path(), 0, 14, 200).unwrap();
        for index in 0..3 {
            sink.record(
                "r-test",
                &ProgressEvent::Log(LogEvent {
                    level: LogLevel::Info,
                    message: format!("line {index}"),
                }),
            );
        }
        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().any(|name| name.ends_with(".log")), "{names:?}");
        assert!(
            names.iter().any(|name| name.ends_with(".log.1")),
            "{names:?}"
        );
        assert!(
            names.iter().any(|name| name.ends_with(".log.2")),
            "{names:?}"
        );
    }

    #[test]
    fn retention_zero_removes_every_existing_log_file_on_open() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("codepack-2020-01-01.log"), "old").unwrap();
        LogSink::open(dir.path(), 50, 0, 200).unwrap();
        assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
    }

    #[test]
    fn the_oldest_files_are_removed_first_once_the_total_cap_is_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("codepack-2020-01-01.log");
        let newer = dir.path().join("codepack-2020-01-02.log");
        fs::write(&old, vec![b'a'; 2 * 1024 * 1024]).unwrap();
        fs::write(&newer, vec![b'b'; 2 * 1024 * 1024]).unwrap();
        // Both files are well within the 14-day retention window; only the 3 MiB total
        // cap should act, and it should remove the older file first.
        let old_mtime = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        set_mtime(&old, old_mtime);

        LogSink::open(dir.path(), 50, 14, 3).unwrap();

        assert!(!old.exists(), "the older file should have been removed");
        assert!(newer.exists(), "the newer file should survive");
    }

    fn set_mtime(path: &Path, time: std::time::SystemTime) {
        let file = OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(time).unwrap();
    }

    // --- The panic hook (audit 2026-09-07, G-1 step 3) ---------------------------------

    /// `set_hook`/`take_hook` fully replace the active hook rather than nesting it, so
    /// `install_panic_hook`'s own hook has to stay the one installed when the panic
    /// fires — it chains to whatever ran before *it*, which prints the default dump to
    /// stderr for this deliberately triggered panic. That is expected test output, not
    /// a failure. The hook is left in place afterwards: it does nothing but log and then
    /// defer to the previous hook, so every later panic in this binary — including a
    /// real test failure — still behaves exactly as it would have.
    #[test]
    fn a_panic_is_written_to_the_panic_log_with_its_message_and_location() {
        let dir = tempfile::tempdir().unwrap();
        install_panic_hook(dir.path());

        let result = std::panic::catch_unwind(|| {
            panic!("planted panic for the log test, API_KEY=sk-live-0123456789abcdef0123456789");
        });
        assert!(result.is_err());

        let contents = fs::read_to_string(dir.path().join("codepack-panics.log")).unwrap();
        assert!(contents.contains("PANIC"));
        assert!(contents.contains("planted panic for the log test"));
        assert!(
            contents.contains("log_sink.rs"),
            "the panic's own source location should be recorded: {contents}"
        );
        // The redaction contract applies here too — a secret in a panic message is
        // exactly the class of leak invariant I3 forbids.
        assert!(!contents.contains("sk-live-0123456789abcdef0123456789"));
        assert!(contents.contains("<REDACTED>"));
    }
}
