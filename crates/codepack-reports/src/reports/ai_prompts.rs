//! `AI_PROMPTS/CUSTOM_PROMPT.md`, ported from legacy
//! `services/prompt_builder.py::build_custom_prompt`/`write_custom_prompt` and the
//! folder-writing half of `reports/insights/ai_prompts.py::write_ai_prompt_files`.
//!
//! Legacy's own `AI_PROMPTS/` folder is populated by **two independent call sites**,
//! confirmed by reading both (`docs/__arch__/codepack-main.zip`):
//!
//! 1. `orchestrator.py` calls `write_ai_prompt_files`, which only creates the
//!    directory — its `PROMPTS: dict[str, str] = {}` is empty in production, so this
//!    call writes zero files. BLUEPRINT §A.7's description of ready-made
//!    review/refactor/security/bug-hunt prompt files describes an aspirational
//!    library that was never actually wired to any output file.
//! 2. `exporter.py`'s own step 6 separately calls `write_custom_prompt` with
//!    `self.config.prompt_goals`, writing exactly one file:
//!    `AI_PROMPTS/CUSTOM_PROMPT.md`.
//!
//! The net legacy result is therefore an `AI_PROMPTS/` folder containing exactly one
//! file. This job reproduces that real, thin feature — not the aspirational
//! multi-prompt library — as a single write, combining both call sites' effects
//! (`mkdir` + the one real file) since this crate has no reason to keep them as two
//! separate no-op-vs-real steps.

use std::path::Path;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::plugin::ReportJob;
use crate::profile;

pub const JOB: ReportJob = ReportJob {
    filename: "AI_PROMPTS",
    profiles: profile::AI_PROMPTS_FOLDER,
    description: "AI_PROMPTS/CUSTOM_PROMPT.md — a ready-to-paste review prompt driven by Config.prompt_goals.",
    run: write_ai_prompt_files,
};

/// Legacy `PROMPT_GOALS` (`services/prompt_builder.py`), ported verbatim including
/// its Russian descriptions — this is report *content*, not agent-facing
/// infrastructure (`.ai/project/10-project-map.md`'s language policy governs
/// documents/code, not artifact prose), and legacy's own text is Russian here.
const PROMPT_GOALS: &[(&str, &str)] = &[
    (
        "bug_hunt",
        "Найти вероятные ошибки, сломанные крайние случаи и логические несоответствия.",
    ),
    (
        "security_review",
        "Проверить безопасность, обработку секретов и опасные паттерны.",
    ),
    (
        "architecture_review",
        "Оценить архитектуру, слои, модульность и связность.",
    ),
    (
        "refactor_plan",
        "Составить безопасный пошаговый план рефакторинга.",
    ),
    (
        "write_tests",
        "Выявить недостающие тесты и предложить практичный тест-план.",
    ),
    (
        "codex_task",
        "Подготовить задачу для Codex с понятными критериями реализации.",
    ),
];

const DEFAULT_PROMPT_GOALS: &[&str] = &["architecture_review", "bug_hunt", "write_tests"];

fn goal_description(key: &str) -> Option<&'static str> {
    PROMPT_GOALS
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, description)| *description)
}

/// Legacy `normalise_prompt_goals`: keeps only recognised, de-duplicated keys in the
/// caller's order; an empty or entirely-invalid input falls back to the same three
/// defaults as `codepack_core::config::Config::default().prompt_goals`.
pub(crate) fn normalise_prompt_goals(goals: &[String]) -> Vec<&'static str> {
    let mut cleaned: Vec<&'static str> = Vec::new();
    for goal in goals {
        if let Some((key, _)) = PROMPT_GOALS.iter().find(|(key, _)| *key == goal.trim())
            && !cleaned.contains(key)
        {
            cleaned.push(key);
        }
    }
    if cleaned.is_empty() {
        DEFAULT_PROMPT_GOALS.to_vec()
    } else {
        cleaned
    }
}

/// Legacy `build_custom_prompt`, verbatim Russian template.
pub(crate) fn build_custom_prompt(project_name: &str, goals: &[String]) -> String {
    let selected = normalise_prompt_goals(goals);
    let goal_lines: String = selected
        .iter()
        .filter_map(|key| goal_description(key))
        .map(|description| format!("- {description}\n"))
        .collect();

    format!(
        "# Промпт для анализа проекта: {project_name}\n\n\
Ты работаешь с экспортом проекта. Сначала изучи структуру, manifest, профиль проекта, \
план экспорта и отчёты безопасности. Затем выполни задачи ниже.\n\n\
## Цели\n\n\
{goal_lines}\n\
## Формат ответа\n\n\
- Сначала перечисли критичные проблемы с файлами и причинами.\n\
- Затем предложи небольшие улучшения с низким риском внедрения.\n\
- Отдельно отметь тесты, которые нужно добавить или обновить.\n\
- Не переписывай проект целиком без необходимости.\n"
    )
}

fn project_name_of(ctx: &ReportContext<'_>) -> String {
    ctx.staging_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn write_ai_prompt_files(ctx: &ReportContext<'_>, output_dir: &Path) -> Result<(), ReportError> {
    std::fs::create_dir_all(output_dir).map_err(|source| ReportError::Write {
        path: output_dir.to_path_buf(),
        source,
    })?;

    let project_name = project_name_of(ctx);
    let content = build_custom_prompt(&project_name, &ctx.config.prompt_goals);
    let output_file = output_dir.join("CUSTOM_PROMPT.md");
    std::fs::write(&output_file, content).map_err(|source| ReportError::Write {
        path: output_file,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fixture;

    #[test]
    fn writes_only_custom_prompt_md_matching_legacys_real_thin_feature() {
        let fixture = Fixture::new(|_root| {});
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_dir = out_dir.path().join(JOB.filename);

        write_ai_prompt_files(&ctx, &output_dir).unwrap();

        let entries: Vec<_> = std::fs::read_dir(&output_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, vec!["CUSTOM_PROMPT.md".to_string()]);
    }

    #[test]
    fn custom_prompt_content_reflects_configured_goals() {
        let fixture = Fixture::new(|_root| {});
        let mut ctx = fixture.context("full");
        let mut config = fixture.config.clone();
        config.prompt_goals = vec!["security_review".to_string()];
        ctx.config = &config;
        let out_dir = tempfile::tempdir().unwrap();
        let output_dir = out_dir.path().join(JOB.filename);

        write_ai_prompt_files(&ctx, &output_dir).unwrap();

        let content = std::fs::read_to_string(output_dir.join("CUSTOM_PROMPT.md")).unwrap();
        assert!(content.contains("Проверить безопасность"));
        assert!(!content.contains("Оценить архитектуру"));
    }

    #[test]
    fn custom_prompt_falls_back_to_defaults_when_goals_are_empty_or_invalid() {
        let fixture = Fixture::new(|_root| {});
        let mut ctx = fixture.context("full");
        let mut config = fixture.config.clone();
        config.prompt_goals = vec!["not_a_real_goal".to_string()];
        ctx.config = &config;
        let out_dir = tempfile::tempdir().unwrap();
        let output_dir = out_dir.path().join(JOB.filename);

        write_ai_prompt_files(&ctx, &output_dir).unwrap();

        let content = std::fs::read_to_string(output_dir.join("CUSTOM_PROMPT.md")).unwrap();
        assert!(content.contains("Оценить архитектуру"));
        assert!(content.contains("Найти вероятные ошибки"));
        assert!(content.contains("Выявить недостающие тесты"));
    }

    #[test]
    fn normalise_prompt_goals_deduplicates_and_falls_back() {
        let goals = vec![
            "bug_hunt".to_string(),
            "bug_hunt".to_string(),
            "unknown".to_string(),
        ];
        assert_eq!(normalise_prompt_goals(&goals), vec!["bug_hunt"]);
        assert_eq!(
            normalise_prompt_goals(&[]),
            vec!["architecture_review", "bug_hunt", "write_tests"]
        );
    }
}
