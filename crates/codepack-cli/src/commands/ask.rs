//! `codepack ask <bundle>` — the one command in this binary that uses the network.
//!
//! Stage S13's API path had a complete domain layer and no door from 2026-07-27 until
//! 2026-09-12. This is the door. Everything it decides is a decision about *whether* to
//! send; the sending itself is [`codepack_ai_api::ask`], which runs the guard, reads the
//! key and performs the exchange in a fixed order that a caller cannot rearrange.
//!
//! ## What the exit codes mean here
//!
//! The published contract is unchanged, and the interesting case falls out of it rather
//! than needing a new number: a send refused because the bundle carries critical
//! findings exits **3**, the same code `scan` uses for "the command worked and found
//! critical secrets". That is exactly what happened — the command worked, and what it
//! found is why nothing was sent. A transport failure, a missing key or a switched-off
//! integration exits **1**: those are failures to do the job, not findings.
//!
//! ## Why the plan is built twice on a real send
//!
//! `--dry-run` and the human-facing summary need the plan before anything leaves;
//! [`codepack_ai_api::ask`] builds its own plan internally and checks it there, because
//! a caller able to pass in a plan is a caller able to pass in a stale one. So a real
//! send reads `AI_CONTEXT/` twice. That is a directory of Markdown, and paying for one
//! extra read buys a guard no caller of this crate can skip — a trade this module makes
//! deliberately rather than by omission.
//!
//! ## What is not here
//!
//! No confirmation prompt. In a terminal, typing the command *is* the explicit user
//! action invariant I1 speaks of, and a prompt would only make the command unusable in
//! the scripts people actually reach for. `--dry-run` is how you look before you leap,
//! and `--override-critical` is still a separate flag rather than a `yes` to a question.

use serde::Serialize;

use codepack_ai_api::Refusal;

use crate::cli::AskArgs;
use crate::commands::bundle;
use crate::error::{CliError, Result};
use crate::exit::Outcome;
use crate::output::{self, Format};

/// A question a user has not typed yet. General on purpose: a specific default would be
/// wrong more often than it was right, and this one at least matches why people export
/// a bundle for a model in the first place.
const DEFAULT_QUESTION: &str =
    "Review this project and tell me what matters most: what it does, how it is \
     structured, and where the risks are.";

#[derive(Debug, Serialize)]
pub(crate) struct AskReport {
    pub bundle: String,
    /// The directory the context was read from — the same as `bundle` unless an archive
    /// had to be unpacked first.
    pub working_dir: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub extracted: bool,
    pub provider: String,
    pub model: String,
    pub question: String,
    pub context_files: usize,
    pub context_bytes: u64,
    pub estimated_tokens: u64,
    /// `null` means the bundle carries no scanner output — **not** that it is clean. The
    /// distinction is the whole reason this is an `Option` rather than a count with zero
    /// standing in for "unknown".
    pub critical_findings: Option<u64>,
    /// True when the estimate alone exceeds the model's advertised window.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub exceeds_context: bool,
    /// Absent on `--dry-run`, and absent when a guard refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<AnswerReport>,
    /// Why nothing was sent, when nothing was sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// True when this run only described what a send would do.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnswerReport {
    pub text: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    /// Set when the provider stopped for a reason worth telling the user about — the
    /// output cap, or a declined request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_early: Option<String>,
    /// Where the answer was appended inside the bundle.
    pub answer_file: String,
}

pub(crate) fn run(args: &AskArgs, format: Format) -> Result<Outcome> {
    // No project directory, so no `.codepack.toml` layer to resolve — global settings
    // and explicit flags, exactly as `handoff` does it.
    let app_paths = codepack_core::AppPaths::resolve()?;
    let config = codepack_core::config::load(&app_paths);

    let provider_id = args
        .provider
        .clone()
        .unwrap_or_else(|| config.normalized_ai_api_provider().to_string());
    let provider = codepack_ai_api::providers::resolve(&provider_id).map_err(|error| {
        let known: Vec<&str> = codepack_ai_api::providers::all()
            .iter()
            .map(|provider| provider.id())
            .collect();
        CliError::message(format!("{error}. Available: {}", known.join(", ")))
    })?;

    let model = resolve_model(args, &config, provider.as_ref())?;
    let question = resolve_question(args.question.as_deref(), &config.ai_api_question);

    let opened = bundle::open_bundle(&args.bundle)?;
    let plan = codepack_ai_api::plan::build_plan(&opened.directory, provider.as_ref(), &model)
        .map_err(|error| CliError::message(error.to_string()))?;

    let mut report = AskReport {
        bundle: args.bundle.display().to_string(),
        working_dir: opened.directory.display().to_string(),
        extracted: opened.extracted,
        provider: plan.provider.clone(),
        model: plan.model.clone(),
        question: question.clone(),
        context_files: plan.context_files,
        context_bytes: plan.context_bytes,
        estimated_tokens: plan.estimated_tokens,
        critical_findings: plan.critical_findings,
        exceeds_context: plan.exceeds_context(),
        answer: None,
        refused: None,
        dry_run: args.dry_run,
    };

    if args.dry_run {
        emit(&report, format)?;
        return Ok(Outcome::Success);
    }

    // The guard, before anything leaves. Run here so the report can say *why* nothing
    // was sent; `codepack_ai_api::ask` runs it again on its own, which is the version
    // that actually gates the network call.
    if let Err(refusal) = plan.check(config.ai_api_enabled, args.override_critical) {
        report.refused = Some(refusal.to_string());
        emit(&report, format)?;
        return Ok(outcome_for(&refusal));
    }

    let answer = codepack_ai_api::ask(
        &opened.directory,
        &provider_id,
        &model,
        &question,
        config.ai_api_enabled,
        args.override_critical,
    )
    .map_err(|error| CliError::message(error.to_string()))?;

    report.answer = Some(AnswerReport {
        text: answer.text,
        model: answer.model,
        input_tokens: answer.input_tokens,
        output_tokens: answer.output_tokens,
        stopped_early: answer.stopped_early,
        answer_file: opened
            .directory
            .join(codepack_ai_api::plan::ANSWER_FILE)
            .display()
            .to_string(),
    });
    emit(&report, format)?;
    Ok(Outcome::Success)
}

/// The question to ask: the flag, then the stored one, then a general-purpose default.
///
/// Blank is not a choice at either level. Somebody who clears the setting, or passes
/// `-q ""` to see what happens, gets the default rather than an empty question — a
/// provider asked nothing answers about nothing, at full token price.
fn resolve_question(flag: Option<&str>, stored: &str) -> String {
    for candidate in [flag.unwrap_or_default(), stored] {
        if !candidate.trim().is_empty() {
            return candidate.to_string();
        }
    }
    DEFAULT_QUESTION.to_string()
}

/// The model to ask: the flag, then the setting, then the provider's most capable known
/// model.
///
/// The last step is why `Config::ai_api_model` may be empty: `codepack-core` names no
/// vendor's models, so "the best one available" can only be resolved where the provider
/// is known. A provider that advertises no models at all is a build error rather than a
/// user error, and says so.
fn resolve_model(
    args: &AskArgs,
    config: &codepack_core::config::Config,
    provider: &dyn codepack_ai_api::AiProvider,
) -> Result<String> {
    if let Some(model) = args.model.clone().filter(|id| !id.trim().is_empty()) {
        return Ok(model);
    }
    if !config.ai_api_model.trim().is_empty() {
        return Ok(config.ai_api_model.clone());
    }
    provider
        .known_models()
        .first()
        .map(|model| model.id.to_string())
        .ok_or_else(|| {
            CliError::message(format!(
                "provider {} advertises no models, so --model is required",
                provider.id()
            ))
        })
}

/// A refusal's exit code.
///
/// `CriticalFindings` is the case worth being careful about: it is not a failure, it is
/// this product doing its job, and code 3 already means "worked, found critical
/// secrets". Reporting 1 there would tell a pipeline the command broke.
fn outcome_for(refusal: &Refusal) -> Outcome {
    match refusal {
        Refusal::CriticalFindings { .. } => Outcome::CriticalSecretsFound,
        Refusal::Disabled | Refusal::EmptyContext => Outcome::Incomplete,
    }
}

fn emit(report: &AskReport, format: Format) -> Result<()> {
    if format.is_json() {
        output::emit_json("ask", report)
    } else {
        print_human(report);
        Ok(())
    }
}

fn print_human(report: &AskReport) {
    output::line(format!("Provider: {}", report.provider));
    output::line(format!("Model:    {}", report.model));
    output::line(format!(
        "Context:  {} file(s), {}, about {} tokens",
        report.context_files,
        codepack_tokens::format_bytes(report.context_bytes),
        report.estimated_tokens
    ));
    match report.critical_findings {
        // Not collapsed into one arm with a count: "nothing checked" and "checked,
        // nothing found" are different facts, and a reader deserves the difference.
        None => output::line(
            "Scanner:  not verified — this bundle carries no security scan, so nothing \
             has checked it for secrets",
        ),
        Some(0) => output::line("Scanner:  checked, no critical findings"),
        Some(count) => output::line(format!("Scanner:  {count} critical finding(s)")),
    }
    if report.exceeds_context {
        output::line(
            "Warning:  the estimate alone exceeds this model's context window; the \
             provider may truncate or refuse",
        );
    }
    output::line("");

    if report.dry_run {
        output::line("Dry run: nothing was sent.");
        return;
    }
    if let Some(reason) = &report.refused {
        output::line(format!("Refused: {reason}"));
        return;
    }
    if let Some(answer) = &report.answer {
        if let Some(stopped) = &answer.stopped_early {
            output::line(format!("(the provider stopped early: {stopped})"));
            output::line("");
        }
        output::line(&answer.text);
        output::line("");
        output::line(format!("Appended to {}", answer.answer_file));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_question_falls_through_to_the_default_at_either_level() {
        assert_eq!(resolve_question(Some("why?"), "stored"), "why?");
        assert_eq!(resolve_question(Some("   "), "stored"), "stored");
        assert_eq!(resolve_question(None, "stored"), "stored");
        assert_eq!(resolve_question(None, ""), DEFAULT_QUESTION);
        assert_eq!(resolve_question(Some(""), "  "), DEFAULT_QUESTION);
    }

    #[test]
    fn a_critical_finding_exits_with_the_findings_code_not_the_failure_code() {
        // Code 3 means "the command worked and here is what it found", which is exactly
        // what a refusal over critical findings is. A pipeline reading 1 would conclude
        // codepack itself broke.
        assert_eq!(
            outcome_for(&Refusal::CriticalFindings { count: 2 }).code(),
            crate::exit::CRITICAL_SECRETS
        );
    }

    #[test]
    fn a_switched_off_integration_is_a_failure_to_do_the_job() {
        assert_eq!(
            outcome_for(&Refusal::Disabled).code(),
            crate::exit::FAILURE
        );
        assert_eq!(
            outcome_for(&Refusal::EmptyContext).code(),
            crate::exit::FAILURE
        );
    }

    #[test]
    fn an_unverified_bundle_is_reported_as_unverified_rather_than_as_clean() {
        // `None` serializing as `null` is the contract a `--json` consumer reads. If it
        // came out as `0`, every consumer would treat an unscanned bundle as a clean one.
        let report = AskReport {
            bundle: "b".into(),
            working_dir: "b".into(),
            extracted: false,
            provider: "anthropic".into(),
            model: "m".into(),
            question: "q".into(),
            context_files: 1,
            context_bytes: 10,
            estimated_tokens: 3,
            critical_findings: None,
            exceeds_context: false,
            answer: None,
            refused: None,
            dry_run: true,
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["critical_findings"], serde_json::Value::Null);
    }

    #[test]
    fn the_report_has_no_field_a_key_could_land_in() {
        // Every field of the serialized report, checked against a list, so adding one
        // that could carry a credential — a resolved key, an auth header, a raw request
        // — fails here rather than in somebody's CI log.
        let report = AskReport {
            bundle: "b".into(),
            working_dir: "b".into(),
            extracted: true,
            provider: "anthropic".into(),
            model: "m".into(),
            question: "q".into(),
            context_files: 1,
            context_bytes: 10,
            estimated_tokens: 3,
            critical_findings: Some(0),
            exceeds_context: true,
            answer: None,
            refused: Some("r".into()),
            dry_run: false,
        };
        let json = serde_json::to_value(&report).unwrap();
        let mut keys: Vec<&str> = json.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "bundle",
                "context_bytes",
                "context_files",
                "critical_findings",
                "estimated_tokens",
                "exceeds_context",
                "extracted",
                "model",
                "provider",
                "question",
                "refused",
                "working_dir",
            ]
        );
    }
}
