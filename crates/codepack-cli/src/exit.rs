//! Process exit codes — a contract, not an implementation detail.
//!
//! `ROADMAP.md` §3 fixes these four values for stage S10. A CI pipeline branches on
//! them, so changing a number later breaks other people's scripts exactly the way
//! changing an artifact format does.

/// The run did what was asked.
pub(crate) const SUCCESS: i32 = 0;
/// Something failed: I/O, a corrupt configuration, a storage error.
pub(crate) const FAILURE: i32 = 1;
/// The arguments could not be understood. Emitted by `clap` itself, declared here so
/// the whole contract is readable in one place.
pub(crate) const BAD_ARGUMENTS: i32 = 2;
/// The command completed, and found `critical`-severity secrets.
pub(crate) const CRITICAL_SECRETS: i32 = 3;

/// What a command decided, before it becomes a process exit code.
///
/// Commands return this rather than calling `std::process::exit` themselves: a command
/// that exits directly cannot be tested without spawning a process, and cannot be
/// composed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Success,
    /// Finished, but `critical` findings were present.
    CriticalSecretsFound,
}

impl Outcome {
    pub(crate) fn code(self) -> i32 {
        match self {
            Self::Success => SUCCESS,
            Self::CriticalSecretsFound => CRITICAL_SECRETS,
        }
    }
}

/// Resolves a completed command into an exit code.
///
/// **A real failure always outranks `CRITICAL_SECRETS`.** Code 3 means "the command
/// worked and here is what it found"; if the command did not work, saying 3 would tell
/// a pipeline the scan result is trustworthy when it is not. This ordering is the one
/// thing about these four codes that is easy to get subtly wrong, so it lives in one
/// function with a test rather than being repeated at each call site.
pub(crate) fn resolve(result: &crate::error::Result<Outcome>) -> i32 {
    match result {
        Ok(outcome) => outcome.code(),
        Err(_) => FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CliError;

    #[test]
    fn the_four_documented_codes_are_what_roadmap_fixes_them_at() {
        assert_eq!(
            (SUCCESS, FAILURE, BAD_ARGUMENTS, CRITICAL_SECRETS),
            (0, 1, 2, 3)
        );
    }

    #[test]
    fn a_failure_outranks_a_critical_finding() {
        // Reporting 3 for a run that errored would tell a pipeline "the scan completed
        // and found secrets" when in fact the scan did not complete.
        let failed: crate::error::Result<Outcome> = Err(CliError::Message("boom".into()));
        assert_eq!(resolve(&failed), FAILURE);
    }

    #[test]
    fn a_clean_run_and_a_finding_run_are_distinguishable() {
        assert_eq!(resolve(&Ok(Outcome::Success)), SUCCESS);
        assert_eq!(
            resolve(&Ok(Outcome::CriticalSecretsFound)),
            CRITICAL_SECRETS
        );
    }
}
