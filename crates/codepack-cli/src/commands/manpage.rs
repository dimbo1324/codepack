//! `codepack manpage` — a roff man page on stdout, for the packaging step to capture.
//!
//! Hidden from `--help` (see [`crate::cli::Command::Manpage`]): this is not a command a
//! person runs, it is how `cargo xtask package` produces `usr/share/man/man1/
//! codepack.1.gz` for the Linux packages (audit 2026-09-07, L-1/L-10). Built from the
//! same [`Cli::command`] the help text and `codepack completions` already use, so it
//! cannot drift from what `--help` actually says.

use clap::CommandFactory;

use crate::cli::Cli;
use crate::error::{CliError, Result};
use crate::exit::Outcome;

pub(crate) fn run() -> Result<Outcome> {
    let command = Cli::command();
    clap_mangen::Man::new(command)
        .render(&mut std::io::stdout())
        .map_err(|source| CliError::message(format!("could not render the man page: {source}")))?;
    Ok(Outcome::Success)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_without_panicking_and_names_the_binary() {
        let mut buffer = Vec::new();
        let command = Cli::command();
        clap_mangen::Man::new(command).render(&mut buffer).unwrap();
        let rendered = String::from_utf8(buffer).unwrap();
        assert!(rendered.contains("codepack"));
    }
}
