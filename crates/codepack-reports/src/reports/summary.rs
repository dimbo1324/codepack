//! `01_summary.txt`, ported from legacy
//! `reports/insights/summary.py::write_project_summary_report`.

use std::path::Path;

use codepack_tokens::format_bytes;

use crate::context::{ReportContext, detect_stack};
use crate::error::ReportError;
use crate::paths::{file_name_of, looks_like_test_path};
use crate::plugin::ReportJob;
use crate::profile;

pub const JOB: ReportJob = ReportJob {
    filename: "01_summary.txt",
    profiles: profile::SUMMARY_TXT,
    description: "High-level overview: stack detection, language counts, biggest files.",
    run: write_summary_report,
};

fn write_summary_report(ctx: &ReportContext<'_>, output_file: &Path) -> Result<(), ReportError> {
    let inventory = ctx.inventory;
    let stack = detect_stack(&ctx.staging_root, inventory);

    let readmes = inventory
        .files
        .iter()
        .filter(|file| {
            file_name_of(&file.relative_path)
                .to_lowercase()
                .starts_with("readme")
        })
        .count();
    let licenses = inventory
        .files
        .iter()
        .filter(|file| {
            file_name_of(&file.relative_path)
                .to_lowercase()
                .starts_with("license")
        })
        .count();
    let env_files = inventory
        .files
        .iter()
        .filter(|file| {
            file_name_of(&file.relative_path)
                .to_lowercase()
                .starts_with(".env")
        })
        .count();
    let test_files = inventory
        .files
        .iter()
        .filter(|file| looks_like_test_path(&file.relative_path))
        .count();
    let docker_files = inventory
        .files
        .iter()
        .filter(|file| {
            file_name_of(&file.relative_path)
                .to_lowercase()
                .starts_with("dockerfile")
        })
        .count();
    let compose_files = inventory
        .files
        .iter()
        .filter(|file| {
            matches!(
                file_name_of(&file.relative_path).to_lowercase().as_str(),
                "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml"
            )
        })
        .count();
    let ci_files = inventory
        .files
        .iter()
        .filter(|file| {
            let forward = file.relative_path.replace('\\', "/");
            forward.starts_with(".github/workflows/")
                && (forward.ends_with(".yml") || forward.ends_with(".yaml"))
        })
        .count();

    let mut sorted_by_size: Vec<&crate::context::InventoryFile> = inventory.files.iter().collect();
    sorted_by_size.sort_by_key(|file| std::cmp::Reverse(file.size));

    let mut out = String::new();
    out.push_str("=== Project Summary ===\n");
    out.push_str(&format!("Generated: {}\n", ctx.plan.generated_at));
    out.push_str(&"=".repeat(100));
    out.push_str("\n\n");

    out.push_str(&format!(
        "{:<32}: {}\n",
        "Source root",
        ctx.source_root.display()
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "Copied project root",
        ctx.staging_root.display()
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "Project name",
        ctx.staging_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "Total files",
        inventory.files.len()
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "Total folders", inventory.total_dirs
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "Total copied size",
        format_bytes(inventory.total_size)
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "README present",
        if readmes > 0 { "yes" } else { "no" }
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "LICENSE present",
        if licenses > 0 { "yes" } else { "no" }
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "Tests detected",
        if test_files > 0 {
            format!("yes ({test_files} files)")
        } else {
            "no".to_string()
        }
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "Docker detected",
        if docker_files > 0 || compose_files > 0 {
            "yes"
        } else {
            "no"
        }
    ));
    out.push_str(&format!(
        "{:<32}: {}\n",
        "CI/CD detected",
        if ci_files > 0 {
            format!("yes ({ci_files} GitHub Actions workflows)")
        } else {
            "no".to_string()
        }
    ));
    out.push_str(&format!("{:<32}: {}\n", ".env-like files", env_files));

    out.push_str("\n--- Detected stack ---\n");
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
        out.push_str(&format!("{group}: {joined}\n"));
    }

    out.push_str("\n--- Detected languages by file count ---\n");
    if inventory.by_language.is_empty() {
        out.push_str("No known language extensions detected.\n");
    } else {
        for stat in inventory.by_language.iter().take(30) {
            out.push_str(&format!(
                "{:<28} {:>8} files   {:>12}\n",
                stat.language,
                stat.count,
                format_bytes(stat.total_size)
            ));
        }
    }

    out.push_str("\n--- Largest files ---\n");
    for file in sorted_by_size.iter().take(15) {
        out.push_str(&format!(
            "{:>12}  {}\n",
            format_bytes(file.size),
            file.relative_path
        ));
    }

    out.push_str("\n--- Useful next checks ---\n");
    if readmes == 0 {
        out.push_str("- Add or update README with setup/run instructions.\n");
    }
    if licenses == 0 {
        out.push_str("- Add LICENSE if this project will be shared externally.\n");
    }
    if env_files > 0 {
        out.push_str("- Review .env-like files before sharing the export.\n");
    }
    if test_files == 0 {
        out.push_str("- No obvious test files found; consider adding smoke/unit tests.\n");
    }
    if ci_files == 0 {
        out.push_str("- No GitHub Actions workflow detected; consider adding CI for checks.\n");
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
    fn writes_stack_language_and_largest_files_sections() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("main.py"), "print('hi')\n").unwrap();
            std::fs::write(root.join("README.md"), "# hi\n").unwrap();
            std::fs::write(
                root.join("package.json"),
                r#"{"dependencies": {"react": "18.0.0"}}"#,
            )
            .unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_summary_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.starts_with("=== Project Summary ==="));
        assert!(content.contains("README present"));
        assert!(content.contains("frontend: React"));
        assert!(content.contains("--- Largest files ---"));
        assert!(content.contains("Python"));
    }

    #[test]
    fn reports_no_readme_and_no_tests_hints_when_absent() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("main.py"), "print('hi')\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_summary_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("Add or update README"));
        assert!(content.contains("No obvious test files found"));
    }
}
