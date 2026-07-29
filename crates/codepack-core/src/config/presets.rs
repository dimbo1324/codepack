//! `AI_PRESETS`. The first five entries are ported verbatim from legacy
//! `constants.py`; anything after them is this project's own addition, kept after the
//! legacy block so the boundary stays visible (and pinned by the tests below). Data
//! only — applying a preset to a live `Config` (surfacing it in the UI, wiring it into
//! a report) is S7's job, not core's.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub export_profile: &'static str,
    pub safe_export_mode: &'static str,
    pub redact_secrets: bool,
    pub include_git_patch: bool,
    pub diff_export_mode: &'static str,
    pub text_file_size_limit_enabled: bool,
    /// `None` when the legacy preset dict did not override `max_text_file_mb`.
    pub max_text_file_mb: Option<u32>,
}

pub fn ai_presets() -> &'static [AiPreset] {
    &[
        AiPreset {
            name: "Claude Code",
            description: "Полный контекст: архитектура, Git-история, все AI-отчёты. Оптимально для Claude.",
            export_profile: "ai_review",
            safe_export_mode: "safe",
            redact_secrets: true,
            include_git_patch: false,
            diff_export_mode: "all",
            text_file_size_limit_enabled: false,
            max_text_file_mb: None,
        },
        AiPreset {
            name: "ChatGPT",
            description: "Компактный обзор без Git-патча. Подходит для GPT-4 с ограниченным контекстным окном.",
            export_profile: "quick",
            safe_export_mode: "safe",
            redact_secrets: true,
            include_git_patch: false,
            diff_export_mode: "all",
            text_file_size_limit_enabled: true,
            max_text_file_mb: Some(1),
        },
        AiPreset {
            name: "Code Review",
            description: "Полный снимок кода с Git-патчем. Идеально для детального ревью.",
            export_profile: "ai_review",
            safe_export_mode: "balanced",
            redact_secrets: true,
            include_git_patch: true,
            diff_export_mode: "all",
            text_file_size_limit_enabled: false,
            max_text_file_mb: None,
        },
        AiPreset {
            name: "Security Audit",
            description: "Акцент на безопасности: конфигурация, зависимости и анализ рисков.",
            export_profile: "security",
            safe_export_mode: "safe",
            redact_secrets: true,
            include_git_patch: false,
            diff_export_mode: "all",
            text_file_size_limit_enabled: false,
            max_text_file_mb: None,
        },
        AiPreset {
            name: "Онбординг",
            description: "Краткий обзор для быстрого введения нового разработчика в проект.",
            export_profile: "minimal",
            safe_export_mode: "balanced",
            redact_secrets: true,
            include_git_patch: false,
            diff_export_mode: "all",
            text_file_size_limit_enabled: true,
            max_text_file_mb: Some(2),
        },
        // Added 2026-07-29, beyond the five legacy entries above. It differs from
        // `Code Review` in *scope*, not in safety: the same audience and the same
        // reports, narrowed to what is actually being proposed rather than the whole
        // project. Composed entirely of existing pieces — `ai_review` is already the
        // profile that gates `REVIEW_CHECKLIST.md` in, and `uncommitted` is already a
        // diff mode — so this adds a combination, not a mechanism.
        AiPreset {
            name: "PR Review",
            description: "Только незакоммиченные изменения с Git-патчем и отчётами для ревью. Для обсуждения одного пул-реквеста, а не всего проекта.",
            export_profile: "ai_review",
            safe_export_mode: "balanced",
            redact_secrets: true,
            include_git_patch: true,
            diff_export_mode: "uncommitted",
            text_file_size_limit_enabled: false,
            max_text_file_mb: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The five legacy entries, in legacy order, still first and still unchanged —
    /// followed by the additions this project has made since. Written as two
    /// assertions rather than one list so the boundary between "ported" and "ours"
    /// stays visible: a future entry appended here must not be mistaken for something
    /// legacy shipped.
    #[test]
    fn the_five_legacy_presets_come_first_in_legacy_order() {
        let names: Vec<&str> = ai_presets().iter().map(|preset| preset.name).collect();
        assert_eq!(
            names[..5],
            [
                "Claude Code",
                "ChatGPT",
                "Code Review",
                "Security Audit",
                "Онбординг"
            ]
        );
    }

    #[test]
    fn additions_beyond_legacy_are_listed_explicitly() {
        let names: Vec<&str> = ai_presets().iter().map(|preset| preset.name).collect();
        assert_eq!(names[5..], ["PR Review"]);
    }

    #[test]
    fn pr_review_narrows_scope_without_loosening_safety() {
        let preset = ai_presets()
            .iter()
            .find(|preset| preset.name == "PR Review")
            .expect("the preset asserted above to exist");
        let code_review = ai_presets()
            .iter()
            .find(|preset| preset.name == "Code Review")
            .expect("a legacy preset asserted above to exist");

        // The whole point: same reports, same safety, narrower scope.
        assert_eq!(preset.diff_export_mode, "uncommitted");
        assert_eq!(preset.export_profile, "ai_review");
        assert!(preset.include_git_patch, "a reviewer needs the patch");
        assert!(preset.redact_secrets);
        assert_eq!(
            preset.safe_export_mode, code_review.safe_export_mode,
            "differing in safety from its sibling would be an unannounced decision"
        );
    }

    #[test]
    fn only_chatgpt_and_onboarding_override_max_text_file_mb() {
        for preset in ai_presets() {
            let expects_override = matches!(preset.name, "ChatGPT" | "Онбординг");
            assert_eq!(
                preset.max_text_file_mb.is_some(),
                expects_override,
                "{}",
                preset.name
            );
        }
    }
}
