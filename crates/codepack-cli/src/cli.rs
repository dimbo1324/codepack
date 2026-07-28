//! The command surface, exactly as `ROADMAP.md` §3 specifies it for stage S10:
//! `export`, `preview`, `scan`, `history`, `doctor`, with `--preset`, `--profile`,
//! `--safe-mode`, `--diff`, `--budget`, `--out` and `--json`.
//!
//! `--json` is declared once, globally, rather than repeated per command: it changes
//! how output is rendered, not what a command does, and a user should not have to
//! remember which subcommands support it.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "codepack",
    version,
    about = "Turn a source folder into a clean, safe snapshot.",
    long_about = "Turn a source folder into a clean, safe snapshot: an archive plus \
                  reports fit to hand to an AI assistant and to a human.\n\n\
                  Analysis is entirely local; nothing is ever sent anywhere.\n\n\
                  Exit codes: 0 success, 1 error, 2 bad arguments, 3 critical secrets found."
)]
pub(crate) struct Cli {
    /// Emit machine-readable JSON on stdout instead of a human report.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Run the full export pipeline and write a bundle.
    Export(ExportArgs),
    /// Report what an export would include, without writing anything.
    Preview(PreviewArgs),
    /// Scan a project for secrets and risky code.
    Scan(ScanArgs),
    /// List previous export runs.
    History(HistoryArgs),
    /// Check the environment and report what is available.
    Doctor,
    /// Strip comments (tree-sitter) and reformat with a `PATH` tool into a separate
    /// destination folder. A standalone action, not part of `export`'s pipeline.
    Sanitize(SanitizeArgs),
    /// Print a shell completion script to stdout.
    Completions(CompletionsArgs),
}

#[derive(Debug, Args)]
pub(crate) struct CompletionsArgs {
    /// Which shell to generate a completion script for.
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

/// Settings shared by the commands that read a project.
#[derive(Debug, Args)]
pub(crate) struct ProjectArgs {
    /// Project directory. Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// AI preset to apply (see `codepack doctor` for the list).
    #[arg(long)]
    pub preset: Option<String>,

    /// Export profile: one of the five built in, or one of your own from
    /// `~/.project_exporter_profiles.json`.
    #[arg(long)]
    pub profile: Option<String>,

    /// How aggressively to exclude sensitive files.
    #[arg(long, value_enum)]
    pub safe_mode: Option<SafeMode>,

    /// Which files to consider: everything, or only what changed.
    #[arg(long, value_enum)]
    pub diff: Option<DiffMode>,

    /// Token budget; drops the least valuable files to fit. Accepts `200000`, `200k`,
    /// `1M`.
    #[arg(long, value_parser = crate::settings::parse_budget)]
    pub budget: Option<u64>,
}

#[derive(Debug, Args)]
pub(crate) struct ExportArgs {
    #[command(flatten)]
    pub project: ProjectArgs,

    /// Directory to write the bundle into. Defaults to the current directory.
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct PreviewArgs {
    #[command(flatten)]
    pub project: ProjectArgs,

    /// Also list the files that would be included.
    #[arg(long)]
    pub list_files: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ScanArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
}

#[derive(Debug, Args)]
pub(crate) struct SanitizeArgs {
    /// Source project directory to read from. Never written to.
    #[arg(long)]
    pub source: PathBuf,

    /// Destination directory the sterile copy is written into. Must not be the same as,
    /// or nested inside, `--source`.
    #[arg(long)]
    pub out: PathBuf,

    /// How aggressively to exclude sensitive files. Defaults to `safe`, matching every
    /// other command that reads a project.
    #[arg(long, value_enum)]
    pub safe_mode: Option<SafeMode>,
}

#[derive(Debug, Args)]
pub(crate) struct HistoryArgs {
    /// Show only runs for this project directory. Defaults to every project.
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// How many runs to show.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

/// Mirrors `codepack_core::config::valid_sets::SAFE_EXPORT_MODES`. Spelled out as an
/// enum so `clap` rejects a bad value with a usage error (exit code 2) and lists the
/// valid ones, rather than letting it reach `Config` normalization, which would quietly
/// fall back to a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum SafeMode {
    /// Exclude the most; safest to share.
    Safe,
    /// The default balance.
    Balanced,
    /// Exclude nothing on safety grounds.
    Full,
}

impl SafeMode {
    pub(crate) fn as_config_value(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Balanced => "balanced",
            Self::Full => "full",
        }
    }
}

/// Mirrors `codepack_core::config::valid_sets::DIFF_EXPORT_MODES`, for the same reason
/// as [`SafeMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum DiffMode {
    /// Every file the rules include.
    All,
    /// Only what changed since the last successful export.
    LastExport,
    /// Only what changed against a git ref.
    GitRef,
    /// Only what is uncommitted.
    Uncommitted,
}

impl DiffMode {
    pub(crate) fn as_config_value(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::LastExport => "last_export",
            Self::GitRef => "git_ref",
            Self::Uncommitted => "uncommitted",
        }
    }
}

impl ProjectArgs {
    /// Collects the flags into the shape [`crate::settings::resolve`] consumes.
    pub(crate) fn overrides(&self) -> crate::settings::Overrides {
        crate::settings::Overrides {
            preset: self.preset.clone(),
            profile: self.profile.clone(),
            safe_mode: self
                .safe_mode
                .map(|mode| mode.as_config_value().to_string()),
            diff: self.diff.map(|mode| mode.as_config_value().to_string()),
            budget: self.budget,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_command_definition_is_internally_consistent() {
        // clap's own audit: catches duplicate flags, bad defaults and broken value
        // parsers at test time rather than at a user's first invocation.
        Cli::command().debug_assert();
    }

    #[test]
    fn every_command_roadmap_names_exists() {
        for argv in [
            vec!["codepack", "export", "."],
            vec!["codepack", "preview", "."],
            vec!["codepack", "scan", "."],
            vec!["codepack", "history"],
            vec!["codepack", "doctor"],
            vec!["codepack", "sanitize", "--source", ".", "--out", "../out"],
        ] {
            assert!(
                Cli::try_parse_from(&argv).is_ok(),
                "failed to parse {argv:?}"
            );
        }
    }

    #[test]
    fn json_is_accepted_on_every_command_not_just_some() {
        for argv in [
            vec!["codepack", "--json", "export", "."],
            vec!["codepack", "export", ".", "--json"],
            vec!["codepack", "doctor", "--json"],
            vec!["codepack", "history", "--json"],
            vec![
                "codepack", "sanitize", "--source", ".", "--out", "../out", "--json",
            ],
        ] {
            assert!(
                Cli::try_parse_from(&argv).is_ok(),
                "failed to parse {argv:?}"
            );
        }
    }

    #[test]
    fn the_project_path_defaults_to_the_current_directory() {
        let cli = Cli::try_parse_from(["codepack", "preview"]).unwrap();
        match cli.command {
            Command::Preview(args) => assert_eq!(args.project.path, PathBuf::from(".")),
            other => panic!("expected preview, got {other:?}"),
        }
    }

    #[test]
    fn budget_units_are_parsed_by_the_argument_parser_itself() {
        let cli = Cli::try_parse_from(["codepack", "export", ".", "--budget", "200k"]).unwrap();
        match cli.command {
            Command::Export(args) => assert_eq!(args.project.budget, Some(200_000)),
            other => panic!("expected export, got {other:?}"),
        }
    }

    #[test]
    fn an_invalid_enum_value_is_a_usage_error_not_a_silent_default() {
        let error = Cli::try_parse_from(["codepack", "export", ".", "--safe-mode", "paranoid"])
            .unwrap_err();
        assert_eq!(error.exit_code(), crate::exit::BAD_ARGUMENTS);
    }

    #[test]
    fn an_invalid_budget_is_a_usage_error_too() {
        let error =
            Cli::try_parse_from(["codepack", "export", ".", "--budget", "lots"]).unwrap_err();
        assert_eq!(error.exit_code(), crate::exit::BAD_ARGUMENTS);
    }

    #[test]
    fn enum_values_map_to_exactly_what_config_expects() {
        use codepack_core::config::{DIFF_EXPORT_MODES, SAFE_EXPORT_MODES};

        for mode in [SafeMode::Safe, SafeMode::Balanced, SafeMode::Full] {
            assert!(
                SAFE_EXPORT_MODES.contains(&mode.as_config_value()),
                "{mode:?} maps to a value Config does not accept"
            );
        }
        for mode in [
            DiffMode::All,
            DiffMode::LastExport,
            DiffMode::GitRef,
            DiffMode::Uncommitted,
        ] {
            assert!(
                DIFF_EXPORT_MODES.contains(&mode.as_config_value()),
                "{mode:?} maps to a value Config does not accept"
            );
        }
    }
}
