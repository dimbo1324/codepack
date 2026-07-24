//! `15_architecture_report.md`, ported from legacy
//! `reports/insights/architecture.py::write_architecture_report`. Maps directories to
//! conceptual layers by folder-name convention and lists entrypoints; unlike
//! `16_key_files_report.md`/`23_refactoring_opportunities.md`, this report does **not**
//! consume [`crate::graph::collect`] — legacy's own `architecture.py` never imports
//! `dependency_graph.py` either (`graph.rs`'s module doc explains the full reuse
//! pattern).

use std::path::Path;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::paths::file_name_of;
use crate::plugin::ReportJob;
use crate::profile;
use crate::reports::layout::all_directories;

pub const JOB: ReportJob = ReportJob {
    filename: "15_architecture_report.md",
    profiles: profile::ARCHITECTURE_REPORT_MD,
    description: "Layered architecture map (folder-name heuristics) and extension points.",
    run: write_architecture_report,
};

const LAYER_HINTS: &[(&str, &[&str])] = &[
    (
        "UI / presentation",
        &[
            "ui",
            "views",
            "pages",
            "components",
            "widgets",
            "layouts",
            "screens",
        ],
    ),
    (
        "Application / orchestration",
        &[
            "services",
            "application",
            "usecases",
            "use_cases",
            "handlers",
            "controllers",
        ],
    ),
    (
        "Domain / business logic",
        &["domain", "core", "models", "entities", "business"],
    ),
    (
        "Data access / persistence",
        &[
            "repositories",
            "repository",
            "dao",
            "db",
            "database",
            "migrations",
            "storage",
        ],
    ),
    (
        "API / transport",
        &["api", "routes", "routers", "endpoints", "http", "server"],
    ),
    (
        "Configuration",
        &["config", "settings", "infra", "infrastructure"],
    ),
    ("Tests", &["tests", "test", "spec", "__tests__"]),
    ("Utilities", &["utils", "helpers", "lib", "shared"]),
];

const ENTRYPOINT_NAMES: &[&str] = &[
    "main.py",
    "__main__.py",
    "app.py",
    "server.py",
    "manage.py",
    "index.ts",
    "index.tsx",
    "main.ts",
    "main.tsx",
    "main.go",
];

fn group_dirs_by_layer(
    directories: &std::collections::BTreeSet<String>,
) -> Vec<(&'static str, Vec<String>)> {
    let mut groups: Vec<(&'static str, Vec<String>)> = LAYER_HINTS
        .iter()
        .map(|(layer, _)| (*layer, Vec::new()))
        .collect();
    for directory in directories {
        let lower_parts: std::collections::HashSet<String> = directory
            .split('\\')
            .map(|part| part.to_lowercase())
            .collect();
        for (index, (_, hints)) in LAYER_HINTS.iter().enumerate() {
            if hints.iter().any(|hint| lower_parts.contains(*hint)) {
                groups[index].1.push(directory.clone());
                break;
            }
        }
    }
    for (_, dirs) in &mut groups {
        dirs.sort_by_key(|value| value.to_lowercase());
    }
    groups
}

fn find_entrypoints(ctx: &ReportContext<'_>) -> Vec<String> {
    let mut result: Vec<String> = ctx
        .inventory
        .files
        .iter()
        .filter(|file| {
            let name = file_name_of(&file.relative_path).to_lowercase();
            ENTRYPOINT_NAMES.contains(&name.as_str())
                || name.starts_with("vite.config")
                || name.starts_with("next.config")
        })
        .map(|file| file.relative_path.clone())
        .collect();
    result.sort_by_key(|value| value.to_lowercase());
    result
}

fn top_level_dirs(ctx: &ReportContext<'_>) -> Vec<String> {
    let mut top: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for file in &ctx.inventory.files {
        if let Some(index) = file.relative_path.find('\\') {
            top.insert(file.relative_path[..index].to_string());
        }
    }
    let mut result: Vec<String> = top.into_iter().collect();
    result.sort_by_key(|value| value.to_lowercase());
    result
}

fn write_architecture_report(
    ctx: &ReportContext<'_>,
    output_file: &Path,
) -> Result<(), ReportError> {
    let directories = all_directories(ctx.inventory);
    let layer_dirs = group_dirs_by_layer(&directories);
    let entrypoints = find_entrypoints(ctx);
    let stack = crate::context::detect_stack(&ctx.staging_root, ctx.inventory);

    let project_name = ctx
        .staging_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str(&format!("# Architecture Report: {project_name}\n\n"));
    out.push_str(&format!("Generated: {}\n\n", ctx.plan.generated_at));
    out.push_str(
        "This report is a static architecture map based on file/folder conventions and detected tooling.\n\n",
    );

    out.push_str("## Detected technology groups\n\n");
    for (group, values) in [
        ("frontend", &stack.frontend),
        ("backend", &stack.backend),
        ("tools", &stack.tools),
        ("testing", &stack.testing),
        ("styling", &stack.styling),
        ("infrastructure", &stack.infrastructure),
        ("package_managers", &stack.package_managers),
    ] {
        let joined = if values.is_empty() {
            "not detected".to_string()
        } else {
            values.join(", ")
        };
        out.push_str(&format!("- **{group}**: {joined}\n"));
    }

    out.push_str("\n## Top-level structure\n\n");
    let top_dirs = top_level_dirs(ctx);
    if top_dirs.is_empty() {
        out.push_str("- No top-level folders detected.\n");
    } else {
        for directory in top_dirs.iter().take(80) {
            out.push_str(&format!("- `{directory}`\n"));
        }
    }

    out.push_str("\n## Layer map\n\n");
    for (layer, dirs) in &layer_dirs {
        out.push_str(&format!("### {layer}\n\n"));
        if dirs.is_empty() {
            out.push_str("- not detected\n");
        } else {
            for directory in dirs.iter().take(80) {
                out.push_str(&format!("- `{directory}`\n"));
            }
            if dirs.len() > 80 {
                out.push_str(&format!("- ... and {} more\n", dirs.len() - 80));
            }
        }
        out.push('\n');
    }

    out.push_str("## Entrypoints and bootstrap/config files\n\n");
    if entrypoints.is_empty() {
        out.push_str("- No obvious entrypoint detected.\n");
    } else {
        for path in entrypoints.iter().take(100) {
            out.push_str(&format!("- `{path}`\n"));
        }
    }

    out.push_str("\n## Extension points\n\n");
    out.push_str("- Add new report jobs under `codepack-reports/src/reports/` and register them in the report catalog.\n");
    out.push_str("- Add new copy/exclusion rules in `codepack-scanner`'s ignore-rule and stack-detection modules.\n");
    out.push_str("- Add new UI surfaces under `apps/desktop/ui`; keep long-running work behind the engine's cancellation-aware pipeline.\n");
    out.push_str("- Keep report writers pure: a `ReportContext` in, one report file out.\n");

    out.push_str("\n## Potential architectural risks\n\n");
    if layer_dirs.iter().all(|(_, dirs)| dirs.is_empty()) {
        out.push_str(
            "- No conventional layers detected; project may be very small or unconventionally organised.\n",
        );
    }
    out.push_str(
        "- Validate this report manually before making structural refactors; it is heuristic.\n",
    );

    std::fs::write(output_file, out).map_err(|source| ReportError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fixture;

    #[test]
    fn classifies_directories_into_layers_and_lists_entrypoints() {
        let fixture = Fixture::new(|root| {
            std::fs::create_dir_all(root.join("src").join("services")).unwrap();
            std::fs::create_dir_all(root.join("src").join("utils")).unwrap();
            std::fs::write(root.join("src").join("services").join("api.py"), "x = 1\n").unwrap();
            std::fs::write(root.join("src").join("utils").join("helper.py"), "y = 2\n").unwrap();
            std::fs::write(root.join("main.py"), "print(1)\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_architecture_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.starts_with("# Architecture Report:"));
        assert!(content.contains("### Application / orchestration"));
        assert!(content.contains("src\\services"));
        assert!(content.contains("### Utilities"));
        assert!(content.contains("src\\utils"));
        assert!(content.contains("## Entrypoints and bootstrap/config files"));
        assert!(content.contains("main.py"));
    }

    #[test]
    fn reports_no_layers_and_no_entrypoints_when_none_detected() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("README.md"), "# hi\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_architecture_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("- No obvious entrypoint detected."));
        assert!(content.contains("No conventional layers detected"));
    }
}
