//! `codepack completions <shell>` — a completion script on stdout, nothing else.
//!
//! No `--json` form: a shell completion script is not a report, it is a shell script
//! that gets piped straight into `source`/a completions directory. Wrapping it in the
//! JSON envelope would break every install instruction a user could copy from a shell's
//! own documentation for `clap`-based tools.

use clap::CommandFactory;
use clap_complete::{Shell, generate};

use crate::cli::{Cli, CompletionsArgs};
use crate::exit::Outcome;

/// Generates `shell`'s completion script for the whole `Cli` command tree into `out`.
///
/// Split out of [`run`] (audit 2026-09-07, C-5) so a test can capture the script into a
/// `Vec<u8>` and assert something about its content, rather than calling `run` as-is —
/// which writes straight to `std::io::stdout()` because that is its job as a command —
/// and flooding the gate's own captured test log with several thousand lines of
/// generated shell script on every run.
pub(crate) fn write_completions(shell: Shell, out: &mut dyn std::io::Write) {
    let mut command = Cli::command();
    let name = command.get_name().to_string();
    generate(shell, &mut command, name, out);
}

pub(crate) fn run(args: &CompletionsArgs) -> Outcome {
    write_completions(args.shell, &mut std::io::stdout());
    Outcome::Success
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Audit 2026-09-07, C-5: the previous version of this test called `run` directly,
    /// which both flooded the gate log with the generated script and asserted nothing
    /// past "did not panic" — an empty script, a truncated one, or one generated for the
    /// wrong command tree would all have passed. Capturing lets it assert the script is
    /// non-empty and actually describes *this* command tree.
    #[test]
    fn every_supported_shell_generates_a_real_script_for_this_command_tree() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ] {
            let mut buffer = Vec::new();
            write_completions(shell, &mut buffer);
            let script = String::from_utf8(buffer).unwrap();

            assert!(!script.is_empty(), "{shell:?} produced an empty script");
            assert!(
                script.contains("codepack"),
                "{shell:?} script never names the command: {script}"
            );
            assert!(
                script.contains("sanitize"),
                "{shell:?} script is missing a real subcommand, not just an empty \
                 top-level shell: {script}"
            );
        }
    }
}
