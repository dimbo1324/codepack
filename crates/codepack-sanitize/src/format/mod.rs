//! External-formatter layer: reformats already-stripped source with whatever the
//! table below names, if and only if it is found on `PATH` (never vendored — owner
//! decision, `docs/__arch__/open-questions.md`, 2026-07-28).
//!
//! A missing binary, a failed spawn, or a non-zero exit is never a hard error for the
//! run: [`format_source`] returns `None` in every one of those cases and the caller
//! writes the stripped-but-unformatted content instead. The whole point of this feature
//! must never depend on the user's machine happening to have a specific formatter
//! installed.
//!
//! Java, C#, PHP and Ruby are deliberately **not** in the table. Every mainstream
//! formatter for those four (`google-java-format`, `dotnet format`, `php-cs-fixer`,
//! `rubocop`) operates on project/file paths, not a clean stdin-in/stdout-out mode this
//! crate could invoke safely without writing a temp file — and this task's own
//! instructions rule out an in-place-only call against a temp file unless its safety is
//! certain. `FileOutcome::StrippedOnlyNoFormatterFound` for those four is the honest
//! answer, not a gap.

mod path_lookup;

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use codepack_core::CancellationToken;
use path_lookup::find_on_path;

use crate::language::Language;

/// How long a formatter subprocess may run before it is treated as hung and killed.
/// Real formatters finish in well under a second on any file this pipeline hands them;
/// this is deliberately generous headroom for a slow disk or a cold filesystem cache, not
/// a value anyone tuned (audit 2026-09-07, P-1).
const FORMATTER_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the watchdog re-checks the cancellation token and the deadline while a
/// formatter subprocess is running — the cadence `05-CONCURRENCY-AND-PERFORMANCE.txt`
/// itself suggests, frequent enough that "Cancel" in the UI feels immediate.
const WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// One candidate formatter invocation: a `PATH` binary name, the label recorded in the
/// report on success, and the arguments that make it read `source` from stdin and write
/// formatted output to stdout. Owned (rather than `&'static str` throughout) because
/// Prettier and clang-format need the file's own name embedded in an argument.
struct Candidate {
    binary: &'static str,
    label: &'static str,
    args: Vec<String>,
}

/// The per-language table (BLUEPRINT-adjacent decision record, 2026-07-28): each entry
/// is tried in order, the first one found on `PATH` and exiting successfully wins.
/// Python has two entries, matching the `ruff format` then `black` fallback the task
/// specifies explicitly.
fn candidates_for(language: Language, file_name: &str) -> Vec<Candidate> {
    match language {
        Language::Rust => vec![Candidate {
            binary: "rustfmt",
            label: "rustfmt",
            args: vec!["--emit".to_string(), "stdout".to_string()],
        }],
        Language::JavaScript | Language::TypeScript | Language::Tsx => vec![Candidate {
            binary: "prettier",
            label: "prettier",
            args: vec!["--stdin-filepath".to_string(), file_name.to_string()],
        }],
        Language::Python => vec![
            Candidate {
                binary: "ruff",
                label: "ruff format",
                args: vec!["format".to_string(), "-".to_string()],
            },
            Candidate {
                binary: "black",
                label: "black",
                args: vec!["-q".to_string(), "-".to_string()],
            },
        ],
        Language::Go => vec![Candidate {
            binary: "gofmt",
            label: "gofmt",
            args: Vec::new(),
        }],
        Language::C | Language::Cpp => vec![Candidate {
            binary: "clang-format",
            label: "clang-format",
            args: vec![format!("-assume-filename={file_name}")],
        }],
        Language::Shell => vec![Candidate {
            binary: "shfmt",
            label: "shfmt",
            args: Vec::new(),
        }],
        // ktlint (github.com/ktlint/ktlint, MIT) reads `--stdin` and writes the formatted
        // content to stdout under `--format`/`-F` (`KtlintCommandLine.format`, verified
        // against its source 2026-07-28 rather than assumed) — the same stdin-in/
        // stdout-out shape as every other entry in this table. `--stdin-path` carries the
        // real file name so ktlint tells a `.kts` script apart from plain `.kt`.
        Language::Kotlin => vec![Candidate {
            binary: "ktlint",
            label: "ktlint",
            args: vec![
                "--stdin".to_string(),
                "--format".to_string(),
                format!("--stdin-path={file_name}"),
            ],
        }],
        Language::Java | Language::CSharp | Language::Php | Language::Ruby | Language::Makefile => {
            Vec::new()
        }
    }
}

/// Attempts every candidate formatter for `language` in order, returning the formatted
/// content and the label of whichever one succeeded. `file_name` is the file's own name
/// (not its full path), used only to build `--stdin-filepath`/`-assume-filename=`-style
/// arguments some formatters need to pick their mode from the extension.
pub(crate) fn format_source(
    language: Language,
    file_name: &str,
    source: &str,
    cancel: &CancellationToken,
) -> Option<(String, String)> {
    for candidate in candidates_for(language, file_name) {
        let Some(binary_path) = find_on_path(candidate.binary) else {
            continue;
        };
        let mut command = Command::new(&binary_path);
        command.args(&candidate.args);
        if let Some(formatted) = run_command_with_stdin(command, source, cancel, FORMATTER_TIMEOUT)
        {
            return Some((formatted, candidate.label.to_string()));
        }
    }
    None
}

/// Runs `command` with `source` written to its stdin and its stdout collected, killing it
/// rather than waiting on it forever if it runs longer than `timeout` or `cancel` trips
/// first. Any failure at any stage (spawn, wait, a killed or non-zero exit, non-UTF-8
/// output) is folded into `None` — the caller falls back to the next candidate or to "no
/// formatter found", never propagates an error up and fails the whole run.
///
/// Split from the single caller that builds `command` (audit 2026-09-07, T-7) so a test
/// can drive the watchdog itself against a stand-in process, without a real formatter
/// binary on `PATH` and without a platform-specific `sleep`/`timeout` command.
///
/// Two independent defects, audit 2026-09-07 P-1, both closed by the same rebuild:
///
/// (a) **Pipe deadlock.** The OS pipe buffer is finite (historically 64 KiB on Linux and
/// on Windows named pipes). Any formatter that starts writing stdout before it has read
/// all of stdin — which streaming filters routinely do — would previously deadlock once
/// `source` exceeded that buffer: the child blocks writing to a full stdout, the parent
/// blocks writing to a full stdin, forever. Stdin is written and stdout is read on their
/// own threads here, so both pipes are serviced concurrently regardless of the child's
/// own read/write order — the only scheme that is correct without assuming anything
/// about a program this crate did not write.
///
/// (b) **No timeout, no cancellation.** `wait_with_output()` alone waits as long as the
/// child does, and the cancellation token was never consulted while it ran: a formatter
/// hung for any reason (deadlocked on its own bug, blocked on a network call some
/// formatters make for their config, just wedged) hung the calling thread — one of
/// `rayon`'s worker threads — forever, and "Cancel" in the UI did nothing until the
/// current file finished. The watchdog loop below polls `try_wait` rather than blocking
/// on it, so both the deadline and `cancel` are checked every
/// [`WATCHDOG_POLL_INTERVAL`].
fn run_command_with_stdin(
    mut command: Command,
    source: &str,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Option<String> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let mut stdin = child.stdin.take()?;
    let mut stdout = child.stdout.take()?;

    let owned_source = source.to_owned();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(owned_source.as_bytes());
        // Dropping `stdin` here closes the write end — the EOF every formatter in this
        // table waits for before it writes anything back.
    });
    let (stdout_tx, stdout_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut collected = Vec::new();
        let _ = stdout.read_to_end(&mut collected);
        let _ = stdout_tx.send(collected);
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait().ok()? {
            Some(status) => break status,
            None => {
                if cancel.is_cancelled() || Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = writer.join();
                    return None;
                }
                std::thread::sleep(WATCHDOG_POLL_INTERVAL);
            }
        }
    };

    let _ = writer.join();
    if !status.success() {
        return None;
    }
    String::from_utf8(stdout_rx.recv().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_cancel() -> CancellationToken {
        CancellationToken::new()
    }

    /// Larger than any realistic OS pipe buffer (historically 64 KiB on Linux and on
    /// Windows named pipes) — big enough that writing it in one call blocks unless
    /// something is draining the pipe concurrently.
    const STUB_FLOOD_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;

    #[test]
    fn rustfmt_formats_rust_when_present_on_path() {
        // `rustfmt` ships with every toolchain this workspace's `rust-toolchain.toml`
        // pins, so this is a real integration test, not a mock.
        let (formatted, label) = format_source(
            Language::Rust,
            "main.rs",
            "fn main( ) {println!(\"hi\");}",
            &no_cancel(),
        )
        .expect("rustfmt must be on PATH in this workspace's toolchain");
        assert_eq!(label, "rustfmt");
        assert!(formatted.contains("fn main() {"));
    }

    #[test]
    fn go_formats_when_gofmt_is_on_path() {
        if find_on_path("gofmt").is_none() {
            return;
        }
        let (formatted, label) = format_source(
            Language::Go,
            "main.go",
            "package main\nfunc main(){println(\"hi\")}\n",
            &no_cancel(),
        )
        .expect("gofmt was just confirmed to be on PATH");
        assert_eq!(label, "gofmt");
        assert!(formatted.contains("func main()"));
    }

    #[test]
    fn python_prefers_ruff_over_black_when_both_are_present() {
        if find_on_path("ruff").is_none() {
            return;
        }
        let (formatted, label) =
            format_source(Language::Python, "a.py", "x=1\ny  =2\n", &no_cancel())
                .expect("ruff was just confirmed to be on PATH");
        assert_eq!(label, "ruff format");
        assert_eq!(formatted, "x = 1\ny = 2\n");
    }

    #[test]
    fn kotlin_formats_when_ktlint_is_on_path() {
        if find_on_path("ktlint").is_none() {
            return;
        }
        let (formatted, label) = format_source(
            Language::Kotlin,
            "Main.kt",
            "fun main( ){\nval x=1\n}\n",
            &no_cancel(),
        )
        .expect("ktlint was just confirmed to be on PATH");
        assert_eq!(label, "ktlint");
        assert!(formatted.contains("fun main()"));
    }

    #[test]
    fn a_language_with_no_table_entry_never_finds_a_formatter() {
        assert!(format_source(Language::Java, "A.java", "class A {}", &no_cancel()).is_none());
        assert!(format_source(Language::CSharp, "A.cs", "class A {}", &no_cancel()).is_none());
        assert!(format_source(Language::Php, "a.php", "<?php\n", &no_cancel()).is_none());
        assert!(format_source(Language::Ruby, "a.rb", "puts 1", &no_cancel()).is_none());
        assert!(
            format_source(
                Language::Makefile,
                "Makefile",
                "all:\n\techo hi\n",
                &no_cancel()
            )
            .is_none()
        );
    }

    #[test]
    fn a_binary_that_cannot_exist_falls_through_to_none() {
        // Exercises the "not found" path directly rather than depending on the
        // developer machine lacking every possible formatter.
        assert!(find_on_path("codepack-sanitize-definitely-not-a-real-tool-9f3b").is_none());
    }

    // --- watchdog: a hung or misbehaving formatter (audit 2026-09-07, P-1/T-7) --------
    //
    // Standing in for a real formatter without any platform-specific `sleep`/`timeout`
    // command: this test binary re-invokes itself, selecting one of the two hidden
    // tests below by exact name, exactly the "more reliable" alternative
    // `04-TESTS.txt` names over a shell utility. Each hidden test is a no-op success
    // unless its own environment variable is set, so a normal `cargo test` run treats
    // them like any other passing, instant test.

    /// Not a real test on its own: a stand-in for a formatter that hangs forever,
    /// reachable only by the test below re-invoking this binary with
    /// `CODEPACK_SANITIZE_STUB_HANG` set.
    #[test]
    fn formatter_stub_hangs_forever() {
        if std::env::var_os("CODEPACK_SANITIZE_STUB_HANG").is_none() {
            return;
        }
        std::thread::sleep(Duration::from_secs(3600));
    }

    /// Not a real test on its own: a stand-in for a formatter that starts writing
    /// stdout — more than any realistic OS pipe buffer — without ever reading stdin,
    /// exactly the shape P-1(a) describes a streaming formatter taking. Reachable only
    /// by the test below re-invoking this binary with `CODEPACK_SANITIZE_STUB_FLOOD`
    /// set.
    #[test]
    fn formatter_stub_floods_stdout_without_reading_stdin() {
        if std::env::var_os("CODEPACK_SANITIZE_STUB_FLOOD").is_none() {
            return;
        }
        let payload = vec![b'x'; STUB_FLOOD_PAYLOAD_BYTES];
        let _ = std::io::stdout().write_all(&payload);
    }

    /// Builds the `Command` that re-invokes this test binary as a stand-in formatter,
    /// running only `hidden_test_name` (via `cargo test`'s own `--exact` filter) with
    /// `env_var` set so that hidden test actually performs its failure mode instead of
    /// being the no-op it is under a normal test run.
    fn stub_formatter_command(hidden_test_name: &str, env_var: &str) -> Command {
        let mut command = Command::new(
            std::env::current_exe().expect("the test binary's own path must be resolvable"),
        );
        // `FILTER` is positional; `--exact` and `--nocapture` are flags that follow it —
        // `--exact <name>` (flag-takes-value order) silently matches nothing instead of
        // erroring, which would make this stub a no-op rather than a build failure.
        command
            .args([hidden_test_name, "--exact", "--nocapture"])
            .env(env_var, "1");
        command
    }

    #[test]
    fn a_hung_formatter_is_killed_within_the_timeout_rather_than_waited_on_forever() {
        let command = stub_formatter_command(
            "format::tests::formatter_stub_hangs_forever",
            "CODEPACK_SANITIZE_STUB_HANG",
        );
        let short_timeout = Duration::from_millis(200);
        let start = Instant::now();

        let result = run_command_with_stdin(command, "source", &no_cancel(), short_timeout);

        assert!(
            result.is_none(),
            "a hung formatter must not be treated as success"
        );
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "the watchdog should have killed the child close to its {short_timeout:?} \
             timeout, not left it running: took {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn cancelling_kills_a_running_formatter_before_its_own_timeout() {
        let command = stub_formatter_command(
            "format::tests::formatter_stub_hangs_forever",
            "CODEPACK_SANITIZE_STUB_HANG",
        );
        let cancel = CancellationToken::new();
        cancel.cancel();
        let start = Instant::now();

        // A long timeout that would fail the test if cancellation were not actually
        // checked — the only way this returns quickly is the watchdog noticing
        // `cancel.is_cancelled()` on its very first poll.
        let result = run_command_with_stdin(command, "source", &cancel, Duration::from_secs(60));

        assert!(result.is_none());
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "cancellation should stop the child immediately, not wait for its timeout: \
             took {:?}",
            start.elapsed()
        );
    }

    /// The deadlock P-1(a) describes: a formatter that starts writing more than a pipe
    /// buffer's worth of stdout before it has read any of stdin. On the old
    /// write-then-read scheme this test would hang forever; here it must complete well
    /// within the timeout, because stdin and stdout are serviced on their own threads.
    #[test]
    fn a_formatter_that_floods_stdout_without_reading_stdin_does_not_deadlock() {
        let command = stub_formatter_command(
            "format::tests::formatter_stub_floods_stdout_without_reading_stdin",
            "CODEPACK_SANITIZE_STUB_FLOOD",
        );
        // Larger than the stub's own payload, so a correct run reads it all rather than
        // succeeding on a truncated read.
        let source = "unread stdin, irrelevant to this stub";
        let start = Instant::now();

        let result = run_command_with_stdin(command, source, &no_cancel(), Duration::from_secs(10));

        // Not an exact-length comparison: the stub runs under `cargo test`'s own
        // harness re-invoked as a child, and `--nocapture` lets *its* banner and summary
        // lines (`running 1 test`, `test result: ok. …`) onto the same stdout around the
        // payload. None of that harness text contains an `x`, so counting the payload's
        // own byte is what survives being wrapped in someone else's harness output.
        let flooded_byte_count = result
            .as_deref()
            .map(|output| output.bytes().filter(|&byte| byte == b'x').count())
            .unwrap_or(0);
        assert_eq!(
            flooded_byte_count, STUB_FLOOD_PAYLOAD_BYTES,
            "the full flood payload should have been read back: {result:?}"
        );
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "concurrent stdin/stdout handling should finish quickly, not deadlock: took \
             {:?}",
            start.elapsed()
        );
    }
}
