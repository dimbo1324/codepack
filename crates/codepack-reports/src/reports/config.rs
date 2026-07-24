//! `09_config.txt`, ported from legacy
//! `reports/insights/config_report.py::write_config_report`/`find_config_files`.
//!
//! [`find_config_files`] is the **canonical** copy: legacy's own `project_profile.py`
//! imports `find_config_files` from `config_report.py` rather than duplicating it, and
//! this module is now that same shared source of truth for
//! [`crate::project_profile::build_project_profile`] — see that module's call site.
//! Earlier (Group G) `project_profile.rs` carried a documented, self-contained copy of
//! this exact logic because this module did not exist yet; that copy is now removed in
//! favor of calling here. Unlike that removed copy, this function does **not**
//! truncate its result — legacy's `find_config_files` never truncates either; only
//! `project_profile.py`'s `important_config_files` field truncates to 100 at its own
//! assignment site, which `build_project_profile` still does locally.

use std::collections::HashMap;
use std::path::Path;

use codepack_tokens::format_bytes;

use crate::context::{ReportContext, redact_line};
use crate::error::ReportError;
use crate::paths::file_name_of;
use crate::plugin::ReportJob;
use crate::profile;

pub const JOB: ReportJob = ReportJob {
    filename: "09_config.txt",
    profiles: profile::CONFIG_TXT,
    description: "Detected configuration files and a capability checklist.",
    run: write_config_report,
};

/// Legacy `CONFIG_FILES` (`constants.py`), name-matched case-insensitively.
pub const CONFIG_FILE_NAMES: &[&str] = &[
    "package.json",
    "pnpm-lock.yaml",
    "package-lock.json",
    "yarn.lock",
    "bun.lockb",
    "tsconfig.json",
    "jsconfig.json",
    "vite.config.ts",
    "vite.config.js",
    "next.config.js",
    "next.config.mjs",
    "next.config.ts",
    "eslint.config.js",
    "eslint.config.mjs",
    ".eslintrc",
    ".eslintrc.js",
    ".eslintrc.json",
    ".prettierrc",
    ".prettierrc.json",
    "prettier.config.js",
    "tailwind.config.js",
    "tailwind.config.ts",
    "postcss.config.js",
    "components.json",
    "pyproject.toml",
    "requirements.txt",
    "requirements-dev.txt",
    "poetry.lock",
    "Pipfile",
    "Pipfile.lock",
    "go.mod",
    "go.sum",
    "Cargo.toml",
    "Cargo.lock",
    "Dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    ".env.example",
    ".gitignore",
    "README.md",
    "LICENSE",
];

/// Legacy `config_report.py::find_config_files`, scoped to
/// [`crate::context::Inventory`] (already-included files) rather than a fresh walk —
/// never truncated, sorted case-insensitively and deduplicated, matching legacy's own
/// `sorted(set(known), key=lambda p: str(p).lower())`.
pub fn find_config_files(ctx: &ReportContext<'_>) -> Vec<String> {
    let known_lower: std::collections::HashSet<String> = CONFIG_FILE_NAMES
        .iter()
        .map(|name| name.to_lowercase())
        .collect();

    let mut found: Vec<String> = ctx
        .inventory
        .files
        .iter()
        .filter(|file| {
            let name = file_name_of(&file.relative_path);
            let lower_name = name.to_lowercase();
            let forward_slash_path = file.relative_path.replace('\\', "/");
            CONFIG_FILE_NAMES.contains(&name)
                || known_lower.contains(&lower_name)
                || (forward_slash_path.starts_with(".github/workflows/")
                    && (lower_name.ends_with(".yml") || lower_name.ends_with(".yaml")))
                || lower_name.starts_with("dockerfile")
                || lower_name.starts_with(".env")
        })
        .map(|file| file.relative_path.clone())
        .collect();

    found.sort_by_key(|path| path.to_lowercase());
    found.dedup();
    found
}

struct Capability {
    label: &'static str,
    present: bool,
}

fn capability_checklist(configs: &[String]) -> Vec<Capability> {
    let names: std::collections::HashSet<String> = configs
        .iter()
        .map(|path| file_name_of(path).to_lowercase())
        .collect();
    let starts_with_any = |prefix: &str| names.iter().any(|name| name.starts_with(prefix));
    let contains_any = |needle: &str| names.iter().any(|name| name.contains(needle));

    vec![
        Capability {
            label: "TypeScript",
            present: names.contains("tsconfig.json"),
        },
        Capability {
            label: "Vite",
            present: starts_with_any("vite.config"),
        },
        Capability {
            label: "ESLint",
            present: contains_any("eslint") || starts_with_any(".eslintrc"),
        },
        Capability {
            label: "Prettier",
            present: contains_any("prettier") || starts_with_any(".prettierrc"),
        },
        Capability {
            label: "Tailwind CSS",
            present: starts_with_any("tailwind.config"),
        },
        Capability {
            label: "PostCSS",
            present: starts_with_any("postcss.config"),
        },
        Capability {
            label: "Docker",
            present: starts_with_any("dockerfile"),
        },
        Capability {
            label: "Docker Compose",
            present: names.iter().any(|name| {
                matches!(
                    name.as_str(),
                    "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml"
                )
            }),
        },
        Capability {
            label: "GitHub Actions",
            present: configs
                .iter()
                .any(|path| path.replace('\\', "/").starts_with(".github/workflows/")),
        },
        Capability {
            label: "Python pyproject",
            present: names.contains("pyproject.toml"),
        },
        Capability {
            label: "Go modules",
            present: names.contains("go.mod"),
        },
        Capability {
            label: "Rust Cargo",
            present: names.contains("cargo.toml"),
        },
        Capability {
            label: "Environment examples",
            present: starts_with_any(".env"),
        },
    ]
}

fn write_config_report(ctx: &ReportContext<'_>, output_file: &Path) -> Result<(), ReportError> {
    let configs = find_config_files(ctx);
    let sizes: HashMap<&str, u64> = ctx
        .inventory
        .files
        .iter()
        .map(|file| (file.relative_path.as_str(), file.size))
        .collect();

    let mut out = String::new();
    out.push_str("=== Configuration Report ===\n");
    out.push_str(&format!("Generated: {}\n", ctx.plan.generated_at));
    out.push_str(&"=".repeat(100));
    out.push_str("\n\n");

    out.push_str("--- Capability checklist ---\n");
    for capability in capability_checklist(&configs) {
        out.push_str(&format!(
            "{:<24} {}\n",
            capability.label,
            if capability.present { "yes" } else { "no" }
        ));
    }

    out.push_str("\n--- Detected configuration files ---\n");
    if configs.is_empty() {
        out.push_str("None detected.\n");
    } else {
        for path in &configs {
            let size = sizes
                .get(path.as_str())
                .map(|size| format_bytes(*size))
                .unwrap_or_else(|| "unknown".to_string());
            out.push_str(&redact_line(&format!("{size:>12}  {path}\n")));
        }
    }

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
    fn detects_known_config_files_and_capabilities() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("package.json"), "{}").unwrap();
            std::fs::write(root.join("tsconfig.json"), "{}").unwrap();
            std::fs::write(root.join("tailwind.config.js"), "module.exports = {}").unwrap();
            std::fs::write(root.join("Dockerfile"), "FROM scratch\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_config_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        let has_capability = |label: &str, expected: &str| {
            let padded = format!("{label:<24}");
            content
                .lines()
                .any(|line| line.starts_with(&padded) && line.trim_end().ends_with(expected))
        };
        assert!(has_capability("TypeScript", "yes"));
        assert!(has_capability("Tailwind CSS", "yes"));
        assert!(has_capability("Docker", "yes"));
        assert!(has_capability("ESLint", "no"));
        assert!(content.contains("package.json"));
        assert!(content.contains("tsconfig.json"));
    }

    #[test]
    fn reports_none_detected_when_no_config_files_exist() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("main.py"), "x = 1\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_config_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("--- Detected configuration files ---\nNone detected."));
    }

    #[test]
    fn find_config_files_is_never_truncated_here() {
        let fixture = Fixture::new(|root| {
            for i in 0..150 {
                std::fs::write(root.join(format!(".env.{i}")), "x=1\n").unwrap();
            }
        });
        let ctx = fixture.context("full");
        let configs = find_config_files(&ctx);
        assert_eq!(configs.len(), 150);
    }
}
