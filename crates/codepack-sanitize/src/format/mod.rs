//! External-formatter layer: reformats already-stripped source with whatever the
//! table below names, if and only if it is found on `PATH` (never vendored — owner
//! decision, `docs/decisions/open-questions.md`, 2026-07-28).
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

use std::io::Write;
use std::process::{Command, Stdio};

use path_lookup::find_on_path;

use crate::language::Language;

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
) -> Option<(String, String)> {
    for candidate in candidates_for(language, file_name) {
        let Some(binary_path) = find_on_path(candidate.binary) else {
            continue;
        };
        if let Some(formatted) = run_stdin(&binary_path, &candidate.args, source) {
            return Some((formatted, candidate.label.to_string()));
        }
    }
    None
}

/// Runs one formatter over `source` via stdin/stdout. Any failure at any stage (spawn,
/// write, wait, non-zero exit, non-UTF-8 output) is folded into `None` — the caller
/// falls back to the next candidate or to "no formatter found", never propagates an
/// error up and fails the whole run.
fn run_stdin(binary_path: &std::path::Path, args: &[String], source: &str) -> Option<String> {
    let mut child = Command::new(binary_path)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    // Scoped so the stdin handle is dropped (closing the pipe) before
    // `wait_with_output` reads stdout — every formatter here waits for EOF on stdin
    // before it writes anything back, and an open write end would deadlock the pipe.
    {
        let mut stdin = child.stdin.take()?;
        stdin.write_all(source.as_bytes()).ok()?;
    }

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustfmt_formats_rust_when_present_on_path() {
        // `rustfmt` ships with every toolchain this workspace's `rust-toolchain.toml`
        // pins, so this is a real integration test, not a mock.
        let (formatted, label) =
            format_source(Language::Rust, "main.rs", "fn main( ) {println!(\"hi\");}")
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
        let (formatted, label) = format_source(Language::Python, "a.py", "x=1\ny  =2\n")
            .expect("ruff was just confirmed to be on PATH");
        assert_eq!(label, "ruff format");
        assert_eq!(formatted, "x = 1\ny = 2\n");
    }

    #[test]
    fn a_language_with_no_table_entry_never_finds_a_formatter() {
        assert!(format_source(Language::Java, "A.java", "class A {}").is_none());
        assert!(format_source(Language::CSharp, "A.cs", "class A {}").is_none());
        assert!(format_source(Language::Php, "a.php", "<?php\n").is_none());
        assert!(format_source(Language::Ruby, "a.rb", "puts 1").is_none());
        assert!(format_source(Language::Makefile, "Makefile", "all:\n\techo hi\n").is_none());
    }

    #[test]
    fn a_binary_that_cannot_exist_falls_through_to_none() {
        // Exercises the "not found" path directly rather than depending on the
        // developer machine lacking every possible formatter.
        assert!(find_on_path("codepack-sanitize-definitely-not-a-real-tool-9f3b").is_none());
    }
}
