//! Turning "what the user asked for" into a single [`Config`].
//!
//! Four sources contribute, and they are applied in this order, each overriding the
//! last:
//!
//! 1. **Built-in defaults** — `Config::default()`.
//! 2. **Global settings** — the user's own file, per machine and per user.
//! 3. **`.codepack.toml`** — the project's committed opinion, shared by the team.
//! 4. **Command-line flags** — this invocation.
//!
//! The order is the conventional one and each step is justified by how far the source
//! travels: a flag applies to one run, a project file to one repository, the global
//! file to one machine. The narrower scope wins.
//!
//! `--preset` sits between (3) and (4): a preset is a named bundle of flags, so it must
//! not override a flag the user typed explicitly on the same command line.

use std::path::Path;

use codepack_core::config::{AiPreset, Config, ai_presets};
use codepack_tokens::ModelContextLimits;

use crate::error::{CliError, Result};
use codepack_core::config::{ProjectConfig, ProjectConfigError};

/// Where each setting ended up coming from, for `--json` consumers and for the human
/// summary. Without this a user cannot tell why an export used `safe` mode.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub(crate) struct ResolutionTrace {
    /// Absolute path of the project config that contributed, if any.
    pub project_config: Option<String>,
    /// Name of the AI preset applied, if any.
    pub preset: Option<String>,
    /// Name of the user-defined export profile applied, if any.
    pub profile: Option<String>,
    /// The model `--budget` was resolved through, when it named one rather than a
    /// number. Reported because "budget: 200000" alone does not tell a reader that the
    /// figure came from a table entry that an override file could have changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_model: Option<String>,
}

/// The flag values that can override configuration, already parsed.
#[derive(Debug, Clone, Default)]
pub(crate) struct Overrides {
    pub preset: Option<String>,
    pub profile: Option<String>,
    pub safe_mode: Option<String>,
    pub diff: Option<String>,
    pub budget: Option<BudgetSpec>,
}

/// Resolves the four layers into one [`Config`], reporting what contributed.
pub(crate) fn resolve(
    base: Config,
    project_root: &Path,
    overrides: &Overrides,
    user_profiles: &codepack_core::profiles::UserProfilesFile,
    model_limits: &ModelContextLimits,
) -> Result<(Config, ResolutionTrace)> {
    let mut config = base;
    let mut trace = ResolutionTrace::default();

    if let Some((path, project)) = ProjectConfig::load(project_root).map_err(to_cli_error)? {
        project.apply_to(&mut config);
        trace.project_config = Some(path.display().to_string());
    }

    // A profile is resolved before a preset so that an explicit `--preset` still wins:
    // both set overlapping fields, and the preset is the more specific request.
    if let Some(name) = &overrides.profile {
        // Validated here rather than left to `apply_custom_profile`, which falls back to
        // `full` for an unknown key — legacy behaviour that is right for a GUI combobox
        // and wrong for a flag. `--profile minimul` would otherwise *widen* the export to
        // `full` while the trace still claimed the typed name had been applied. Same
        // reasoning as `--preset`: a typo must not silently produce a different export.
        if !is_known_profile(name, user_profiles) {
            let known = known_profile_names(user_profiles).join(", ");
            return Err(CliError::message(format!(
                "unknown profile `{name}`; available profiles: {known}"
            )));
        }
        config = codepack_core::profiles::apply_custom_profile(&config, user_profiles, name);
        trace.profile = Some(name.clone());
    }

    if let Some(name) = &overrides.preset {
        let preset = find_preset(name)?;
        apply_preset(&mut config, preset);
        trace.preset = Some(preset.name.to_string());
    }

    // Individual flags last: the user typed these on this command line, so nothing
    // should be able to override them.
    if let Some(safe_mode) = &overrides.safe_mode {
        config.safe_export_mode = safe_mode.clone();
    }
    if let Some(diff) = &overrides.diff {
        config.diff_export_mode = diff.clone();
    }
    if let Some(spec) = &overrides.budget {
        config.token_budget = resolve_budget(spec, model_limits)?;
        if let BudgetSpec::Model(name) = spec {
            trace.budget_model = Some(name.clone());
        }
    }

    Ok((config, trace))
}

/// Re-renders the shared parser's failure in this binary's own error idiom.
///
/// The diagnosis (which file, where, what) is produced once in `codepack-core`; the
/// CLI keeps its own variant because its rendering is deliberate — see the comment on
/// [`CliError::ProjectConfigSyntax`] about never echoing the file's text.
fn to_cli_error(error: ProjectConfigError) -> CliError {
    match error {
        ProjectConfigError::Read { path, source } => CliError::Read { path, source },
        ProjectConfigError::Syntax {
            path,
            span,
            message,
        } => CliError::ProjectConfigSyntax {
            path,
            span,
            message,
        },
    }
}

fn known_profile_names(user_profiles: &codepack_core::profiles::UserProfilesFile) -> Vec<String> {
    let mut names: Vec<String> = codepack_core::config::EXPORT_PROFILES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    names.extend(user_profiles.profiles.keys().cloned());
    names
}

fn is_known_profile(name: &str, user_profiles: &codepack_core::profiles::UserProfilesFile) -> bool {
    codepack_core::config::EXPORT_PROFILES.contains(&name)
        || user_profiles.profiles.contains_key(name)
}

/// Looks a preset up by name, case-insensitively, listing the valid names on failure.
///
/// Rejecting an unknown preset rather than falling back to the default is deliberate:
/// `--preset clade` (a typo) silently producing a default export is precisely the kind
/// of quiet wrong answer a CI pipeline would never notice.
pub(crate) fn find_preset(name: &str) -> Result<&'static AiPreset> {
    let wanted = name.trim().to_lowercase();
    ai_presets()
        .iter()
        .find(|preset| preset.name.to_lowercase() == wanted)
        .ok_or_else(|| {
            let known = ai_presets()
                .iter()
                .map(|preset| preset.name)
                .collect::<Vec<_>>()
                .join(", ");
            CliError::message(format!(
                "unknown preset `{name}`; available presets: {known}"
            ))
        })
}

fn apply_preset(config: &mut Config, preset: &AiPreset) {
    config.export_profile = preset.export_profile.to_string();
    config.safe_export_mode = preset.safe_export_mode.to_string();
    config.redact_secrets = preset.redact_secrets;
    config.include_git_patch = preset.include_git_patch;
    config.diff_export_mode = preset.diff_export_mode.to_string();
    config.text_file_size_limit_enabled = preset.text_file_size_limit_enabled;
    if let Some(max_text_file_mb) = preset.max_text_file_mb {
        config.max_text_file_mb = max_text_file_mb;
    }
}

/// What `--budget` was given: a number of tokens, or the name of a model to look the
/// number up from.
///
/// Kept unresolved until [`resolve`] because the model table can be overridden by a
/// file whose location comes from `AppPaths`, and clap's `value_parser` runs before any
/// of that exists. Resolving there would also misreport an unknown model as exit 2
/// ("the arguments could not be understood") when the arguments were understood
/// perfectly and the model simply is not in the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BudgetSpec {
    Tokens(u64),
    Model(String),
}

/// Parses `--budget`, accepting `200000`, `200k`, `1M`, or a model name.
///
/// BLUEPRINT §B.5's own example is `--budget 200k`, and context windows are quoted in
/// those units everywhere, so requiring a bare integer would make the documented
/// invocation fail. Anything that is not a number is taken as a model name and checked
/// later — this stage only decides *which kind* of value it is.
pub(crate) fn parse_budget(raw: &str) -> std::result::Result<BudgetSpec, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("budget must not be empty".to_string());
    }

    let (digits, multiplier) = match text.chars().last() {
        Some('k' | 'K') => (&text[..text.len() - 1], 1_000),
        Some('m' | 'M') => (&text[..text.len() - 1], 1_000_000),
        _ => (text, 1),
    };

    // A leading digit or sign is what distinguishes "a number, possibly with a unit"
    // from "a model name" — and `Gemini 1.5 Pro (1M)` ends in `M`, so the suffix alone
    // cannot decide it. A sign counts as numeric intent so that `-5` stays a usage
    // error instead of becoming a model nobody will ever have.
    if !digits
        .trim()
        .starts_with(|character: char| character.is_ascii_digit() || matches!(character, '+' | '-'))
    {
        return Ok(BudgetSpec::Model(text.to_string()));
    }

    let value: u64 = digits.trim().parse().map_err(|_| {
        format!("`{raw}` is not a token budget; use a number, a number with k/M, or a model name")
    })?;
    value
        .checked_mul(multiplier)
        .map(BudgetSpec::Tokens)
        .ok_or_else(|| format!("`{raw}` is too large to be a token budget"))
}

/// Turns a [`BudgetSpec`] into a token count, consulting the model table when needed.
///
/// Matching is deliberately forgiving, because the table holds legacy *display* names —
/// `Claude (200K)`, `GPT-4o (128K)` — and demanding one of those verbatim, with its
/// capitals and parentheses, through a shell that treats them specially would make the
/// feature technically present and practically unusable.
///
/// Forgiving matching needs a stated rule for collisions, so: exact, then
/// case-insensitive exact, then case-insensitive substring **only when exactly one
/// model matches**. `GPT-4` matches two entries and is an error naming both — guessing
/// there would silently pick a context window the user did not ask for, and a budget
/// that is wrong in the generous direction quietly ships more than intended.
pub(crate) fn resolve_budget(spec: &BudgetSpec, limits: &ModelContextLimits) -> Result<u64> {
    let name = match spec {
        BudgetSpec::Tokens(value) => return Ok(*value),
        BudgetSpec::Model(name) => name,
    };

    if let Some(limit) = limits.get(name) {
        return Ok(limit);
    }

    let wanted = name.to_lowercase();
    if let Some((_, limit)) = limits
        .iter()
        .find(|(candidate, _)| candidate.to_lowercase() == wanted)
    {
        return Ok(limit);
    }

    let matches: Vec<(&str, u64)> = limits
        .iter()
        .filter(|(candidate, _)| candidate.to_lowercase().contains(&wanted))
        .collect();

    match matches.as_slice() {
        [(_, limit)] => Ok(*limit),
        [] => Err(CliError::message(format!(
            "unknown model `{name}`; available models: {}",
            model_names(limits)
        ))),
        several => Err(CliError::message(format!(
            "`{name}` matches more than one model: {}. Name one of them exactly.",
            several
                .iter()
                .map(|(candidate, _)| *candidate)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn model_names(limits: &ModelContextLimits) -> String {
    limits
        .iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests;
