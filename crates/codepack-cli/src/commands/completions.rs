//! `codepack completions <shell>` — a completion script on stdout, nothing else.
//!
//! No `--json` form: a shell completion script is not a report, it is a shell script
//! that gets piped straight into `source`/a completions directory. Wrapping it in the
//! JSON envelope would break every install instruction a user could copy from a shell's
//! own documentation for `clap`-based tools.

use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::{Cli, CompletionsArgs};
use crate::exit::Outcome;

pub(crate) fn run(args: &CompletionsArgs) -> Outcome {
    let mut command = Cli::command();
    let name = command.get_name().to_string();
    generate(args.shell, &mut command, name, &mut std::io::stdout());
    Outcome::Success
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap_complete::Shell;

    #[test]
    fn every_supported_shell_generates_without_panicking() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ] {
            let outcome = run(&CompletionsArgs { shell });
            assert_eq!(outcome, Outcome::Success);
        }
    }
}
