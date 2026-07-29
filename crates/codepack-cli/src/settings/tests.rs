//! Tests for the parent module. Split out when the file passed the ~600-line
//! limit in `.ai/project/12-domain-rules.md`; the code itself is unchanged, and
//! `use super::*` still reaches exactly what it reached inline.

use super::*;
use codepack_core::profiles::UserProfilesFile;

fn empty_project() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn tokens(raw: &str) -> u64 {
    match parse_budget(raw).unwrap() {
        BudgetSpec::Tokens(value) => value,
        BudgetSpec::Model(name) => panic!("`{raw}` should be a number, parsed as model {name}"),
    }
}

#[test]
fn budget_accepts_the_units_blueprint_uses() {
    assert_eq!(tokens("200000"), 200_000);
    assert_eq!(tokens("200k"), 200_000);
    assert_eq!(tokens("200K"), 200_000);
    assert_eq!(tokens("1M"), 1_000_000);
    assert_eq!(tokens(" 8k "), 8_000);
}

#[test]
fn a_non_numeric_budget_is_taken_as_a_model_name() {
    assert_eq!(
        parse_budget("Claude").unwrap(),
        BudgetSpec::Model("Claude".to_string())
    );
    // Ends in `M`, but does not start with a digit — the suffix alone cannot decide.
    assert_eq!(
        parse_budget("Gemini 1.5 Pro (1M)").unwrap(),
        BudgetSpec::Model("Gemini 1.5 Pro (1M)".to_string())
    );
}

#[test]
fn a_model_name_resolves_through_the_table() {
    let limits = ModelContextLimits::default();

    // Exact.
    assert_eq!(
        resolve_budget(&BudgetSpec::Model("Claude (200K)".into()), &limits).unwrap(),
        200_000
    );
    // Case-insensitive exact.
    assert_eq!(
        resolve_budget(&BudgetSpec::Model("claude (200k)".into()), &limits).unwrap(),
        200_000
    );
    // Unambiguous substring — the spelling a person actually types.
    assert_eq!(
        resolve_budget(&BudgetSpec::Model("claude".into()), &limits).unwrap(),
        200_000
    );
    assert_eq!(
        resolve_budget(&BudgetSpec::Model("gemini".into()), &limits).unwrap(),
        1_000_000
    );
}

#[test]
fn an_ambiguous_model_names_the_candidates_instead_of_guessing() {
    // Two entries contain `gpt-4`; picking one would silently hand back a context
    // window the user did not ask for.
    let error = resolve_budget(
        &BudgetSpec::Model("gpt-4".into()),
        &ModelContextLimits::default(),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("more than one"), "{error}");
    assert!(error.contains("GPT-4o (128K)"), "{error}");
    assert!(error.contains("GPT-4 Turbo (128K)"), "{error}");
}

#[test]
fn an_unknown_model_lists_what_is_available() {
    let error = resolve_budget(
        &BudgetSpec::Model("llama".into()),
        &ModelContextLimits::default(),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("unknown model"), "{error}");
    assert!(error.contains("Claude (200K)"), "{error}");
}

#[test]
fn a_numeric_budget_never_consults_the_table() {
    // Including zero, which means "no budget" and must keep meaning that.
    assert_eq!(
        resolve_budget(&BudgetSpec::Tokens(0), &ModelContextLimits::default()).unwrap(),
        0
    );
    assert_eq!(
        resolve_budget(&BudgetSpec::Tokens(1234), &ModelContextLimits::default()).unwrap(),
        1234
    );
}

#[test]
fn resolving_by_model_is_recorded_in_the_trace() {
    let dir = empty_project();
    let overrides = Overrides {
        budget: Some(BudgetSpec::Model("claude".into())),
        ..Overrides::default()
    };
    let (config, trace) = resolve(
        Config::default(),
        dir.path(),
        &overrides,
        &UserProfilesFile::default(),
        &ModelContextLimits::default(),
    )
    .unwrap();

    assert_eq!(config.token_budget, 200_000);
    assert_eq!(trace.budget_model.as_deref(), Some("claude"));
}

/// The point of routing through `ModelContextLimits` rather than a hard-coded
/// match: a model the binary has never heard of works without a rebuild.
#[test]
fn a_model_added_by_the_override_file_resolves() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("model_limits.json");
    std::fs::write(&file, r#"{"Brand New 5 (2M)": 2000000}"#).unwrap();
    let limits = ModelContextLimits::load_or_default(&file).unwrap();

    assert_eq!(
        resolve_budget(&BudgetSpec::Model("brand new 5".into()), &limits).unwrap(),
        2_000_000
    );
}

#[test]
fn a_numeric_budget_leaves_the_model_field_empty() {
    let dir = empty_project();
    let overrides = Overrides {
        budget: Some(BudgetSpec::Tokens(50_000)),
        ..Overrides::default()
    };
    let (config, trace) = resolve(
        Config::default(),
        dir.path(),
        &overrides,
        &UserProfilesFile::default(),
        &ModelContextLimits::default(),
    )
    .unwrap();

    assert_eq!(config.token_budget, 50_000);
    assert_eq!(trace.budget_model, None);
}

#[test]
fn the_pr_review_preset_is_reachable_from_the_flag() {
    let dir = empty_project();
    let overrides = Overrides {
        preset: Some("pr review".to_string()),
        ..Overrides::default()
    };
    let (config, trace) = resolve(
        Config::default(),
        dir.path(),
        &overrides,
        &UserProfilesFile::default(),
        &ModelContextLimits::default(),
    )
    .unwrap();

    assert_eq!(trace.preset.as_deref(), Some("PR Review"));
    assert_eq!(config.diff_export_mode, "uncommitted");
    assert_eq!(config.export_profile, "ai_review");
    assert!(config.include_git_patch);
    assert!(config.redact_secrets);
}

/// Only values that *claim* to be numbers are rejected here. A word like `lots` is
/// now a model name, and is rejected later — by name, with the table listed — which
/// is a better message than a parser could give.
#[test]
fn budget_rejects_nonsense_instead_of_guessing() {
    assert!(parse_budget("").is_err());
    assert!(parse_budget("12kb").is_err());
    assert!(parse_budget("-5").is_err());
    assert!(parse_budget("99999999999999999999M").is_err());
}

#[test]
fn a_flag_beats_the_project_file() {
    let dir = empty_project();
    std::fs::write(
        dir.path()
            .join(codepack_core::config::PROJECT_CONFIG_FILE_NAME),
        "safe_export_mode = \"full\"\n",
    )
    .unwrap();

    let overrides = Overrides {
        safe_mode: Some("safe".to_string()),
        ..Overrides::default()
    };
    let (config, trace) = resolve(
        Config::default(),
        dir.path(),
        &overrides,
        &UserProfilesFile::default(),
        &ModelContextLimits::default(),
    )
    .unwrap();

    assert_eq!(config.safe_export_mode, "safe");
    assert!(trace.project_config.is_some());
}

#[test]
fn the_project_file_beats_the_global_settings_it_was_given() {
    let dir = empty_project();
    std::fs::write(
        dir.path()
            .join(codepack_core::config::PROJECT_CONFIG_FILE_NAME),
        "safe_export_mode = \"safe\"\n",
    )
    .unwrap();

    let global = Config {
        safe_export_mode: "full".to_string(),
        ..Config::default()
    };
    let (config, _) = resolve(
        global,
        dir.path(),
        &Overrides::default(),
        &UserProfilesFile::default(),
        &ModelContextLimits::default(),
    )
    .unwrap();

    assert_eq!(config.safe_export_mode, "safe");
}

#[test]
fn a_preset_beats_a_profile_because_it_is_the_more_specific_request() {
    let dir = empty_project();
    let preset = ai_presets()[0].name.to_string();
    let overrides = Overrides {
        preset: Some(preset.clone()),
        profile: Some("minimal".to_string()),
        ..Overrides::default()
    };

    let (config, trace) = resolve(
        Config::default(),
        dir.path(),
        &overrides,
        &UserProfilesFile::default(),
        &ModelContextLimits::default(),
    )
    .unwrap();

    assert_eq!(trace.preset.as_deref(), Some(preset.as_str()));
    assert_eq!(trace.profile.as_deref(), Some("minimal"));
    assert_eq!(config.export_profile, ai_presets()[0].export_profile);
}

#[test]
fn an_explicit_safe_mode_flag_still_wins_over_a_preset() {
    let dir = empty_project();
    let overrides = Overrides {
        preset: Some(ai_presets()[0].name.to_string()),
        safe_mode: Some("full".to_string()),
        ..Overrides::default()
    };
    let (config, _) = resolve(
        Config::default(),
        dir.path(),
        &overrides,
        &UserProfilesFile::default(),
        &ModelContextLimits::default(),
    )
    .unwrap();
    assert_eq!(config.safe_export_mode, "full");
}

#[test]
fn an_unknown_profile_is_rejected_rather_than_silently_widening_the_export() {
    // `apply_custom_profile` falls back to `full` for an unknown key. Reaching that
    // fallback from a flag would mean `--profile minimul` exports *more* than the
    // user asked for, while the trace reported the name they typed.
    let dir = empty_project();
    let overrides = Overrides {
        profile: Some("minimul".to_string()),
        ..Overrides::default()
    };
    let error = resolve(
        Config::default(),
        dir.path(),
        &overrides,
        &UserProfilesFile::default(),
        &ModelContextLimits::default(),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("minimul"));
    assert!(error.contains("available profiles"));
}

#[test]
fn a_user_defined_profile_is_accepted() {
    let dir = empty_project();
    let mut profiles = UserProfilesFile::default();
    profiles.profiles.insert(
        "team".to_string(),
        codepack_core::profiles::UserProfile {
            safe_export_mode: Some("safe".to_string()),
            ..Default::default()
        },
    );

    let overrides = Overrides {
        profile: Some("team".to_string()),
        ..Overrides::default()
    };
    let (config, trace) = resolve(
        Config::default(),
        dir.path(),
        &overrides,
        &profiles,
        &ModelContextLimits::default(),
    )
    .unwrap();
    assert_eq!(config.safe_export_mode, "safe");
    assert_eq!(trace.profile.as_deref(), Some("team"));
}

#[test]
fn an_unknown_preset_is_rejected_and_the_message_lists_the_real_ones() {
    let error = find_preset("clade").unwrap_err().to_string();
    assert!(error.contains("clade"));
    for preset in ai_presets() {
        assert!(
            error.contains(preset.name),
            "the message should list `{}`: {error}",
            preset.name
        );
    }
}

#[test]
fn preset_lookup_is_case_insensitive() {
    let name = ai_presets()[0].name;
    assert_eq!(find_preset(&name.to_uppercase()).unwrap().name, name);
}

#[test]
fn nothing_configured_anywhere_yields_the_defaults_untouched() {
    let dir = empty_project();
    let (config, trace) = resolve(
        Config::default(),
        dir.path(),
        &Overrides::default(),
        &UserProfilesFile::default(),
        &ModelContextLimits::default(),
    )
    .unwrap();
    assert_eq!(config, Config::default());
    assert_eq!(trace, ResolutionTrace::default());
}
