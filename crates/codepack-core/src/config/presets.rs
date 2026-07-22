//! `AI_PRESETS`, ported verbatim from legacy `constants.py`. Data only — applying a
//! preset to a live `Config` (surfacing it in the UI, wiring it into a report) is S7's
//! job, not core's.

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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_exactly_five_presets_in_legacy_order() {
        let names: Vec<&str> = ai_presets().iter().map(|preset| preset.name).collect();
        assert_eq!(
            names,
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
