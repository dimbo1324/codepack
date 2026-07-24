//! `24_architecture_map.md`, ported from legacy
//! `reports/insights/architecture_map.py::write_architecture_map_report`. Classifies
//! every file into one coarse layer (unlike `15_architecture_report.md`, which
//! classifies *directories* by name convention and lists entrypoints separately —
//! legacy keeps these two reports genuinely distinct, and so does this port).
//!
//! Legacy's own `_layer_for_path` checks `path.parts` against a full **absolute**
//! filesystem path (never `.relative_to(root)` first) — almost certainly an
//! unintentional legacy quirk rather than a deliberate design choice, since every
//! sibling module in this same report family relativizes first. This crate's
//! [`crate::context::Inventory`] never retains an absolute path at all (only
//! `relative_path`), so reproducing the quirk bit-for-bit is not possible here; this
//! port classifies against the relative path's own segments instead — the clearly
//! intended behavior, and the only one this crate's types can express.

use std::path::Path;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::paths::file_name_of;
use crate::plugin::ReportJob;
use crate::profile;

pub const JOB: ReportJob = ReportJob {
    filename: "24_architecture_map.md",
    profiles: profile::ARCHITECTURE_MAP_MD,
    description: "Per-file architectural layer map, plus a Mermaid dependency-direction diagram.",
    run: write_architecture_map_report,
};

const PER_LAYER_LIMIT: usize = 40;

fn layer_for_file(relative_path: &str, extension: &str) -> &'static str {
    let parts: std::collections::HashSet<String> = relative_path
        .split('\\')
        .map(|part| part.to_lowercase())
        .collect();
    let name = file_name_of(relative_path).to_lowercase();

    if matches!(
        name.as_str(),
        "main.py" | "__main__.py" | "app.py" | "manage.py"
    ) || name.starts_with("vite.config")
    {
        return "entrypoints";
    }
    if parts.contains("ui")
        || parts.contains("components")
        || parts.contains("pages")
        || matches!(
            extension,
            "html" | "css" | "scss" | "tsx" | "jsx" | "vue" | "svelte"
        )
    {
        return "interface";
    }
    if parts.contains("services") || parts.contains("service") || parts.contains("usecases") {
        return "business_services";
    }
    if parts.contains("reports") || parts.contains("exporters") {
        return "report_generation";
    }
    if parts.contains("models") || parts.contains("schemas") || parts.contains("entities") {
        return "data_models";
    }
    if parts.contains("utils") || parts.contains("helpers") || parts.contains("lib") {
        return "utilities";
    }
    if parts.contains("tests") || name.contains("test") {
        return "tests";
    }
    if matches!(
        extension,
        "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf"
    ) {
        return "configuration";
    }
    if matches!(extension, "md" | "rst" | "adoc" | "txt") {
        return "documentation";
    }
    "other"
}

fn write_architecture_map_report(
    ctx: &ReportContext<'_>,
    output_file: &Path,
) -> Result<(), ReportError> {
    let mut layers: std::collections::BTreeMap<&'static str, Vec<&str>> =
        std::collections::BTreeMap::new();
    for file in &ctx.inventory.files {
        let layer = layer_for_file(&file.relative_path, &file.extension);
        layers
            .entry(layer)
            .or_default()
            .push(file.relative_path.as_str());
    }
    for paths in layers.values_mut() {
        paths.sort_by_key(|path| path.to_lowercase());
    }

    let mut out = String::new();
    out.push_str("# Architecture Map\n\n");
    out.push_str(&format!("Generated: {}\n\n", ctx.plan.generated_at));
    out.push_str("## Layer summary\n\n");
    for (layer, paths) in &layers {
        out.push_str(&format!("### {layer}\n\n"));
        for path in paths.iter().take(PER_LAYER_LIMIT) {
            out.push_str(&format!("- `{path}`\n"));
        }
        if paths.len() > PER_LAYER_LIMIT {
            out.push_str(&format!(
                "- ... and {} more\n",
                paths.len() - PER_LAYER_LIMIT
            ));
        }
        out.push('\n');
    }

    out.push_str("## Suggested dependency direction\n\n");
    out.push_str("```mermaid\n");
    out.push_str("flowchart TD\n");
    out.push_str("  entrypoints[Entrypoints] --> interface[Interface / CLI / UI]\n");
    out.push_str("  interface --> business_services[Business services]\n");
    out.push_str("  business_services --> report_generation[Report generation]\n");
    out.push_str("  business_services --> data_models[Data models]\n");
    out.push_str("  report_generation --> utilities[Utilities]\n");
    out.push_str("  data_models --> utilities\n");
    out.push_str("  tests[Tests] --> business_services\n");
    out.push_str("  tests --> report_generation\n");
    out.push_str("```\n\n");
    out.push_str("## Review notes\n\n");
    out.push_str("- Keep UI modules thin; long-running work should stay in services.\n");
    out.push_str("- Report-generation modules should not depend on a GUI toolkit.\n");
    out.push_str("- Utility modules should stay dependency-light and deterministic.\n");

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
    fn classifies_entrypoint_service_and_utility_files_into_layers() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("main.py"), "print(1)\n").unwrap();
            std::fs::create_dir_all(root.join("services")).unwrap();
            std::fs::write(root.join("services").join("user_service.py"), "x = 1\n").unwrap();
            std::fs::create_dir_all(root.join("utils")).unwrap();
            std::fs::write(root.join("utils").join("helper.py"), "y = 2\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_architecture_map_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.starts_with("# Architecture Map"));
        assert!(content.contains("### entrypoints\n\n- `main.py`"));
        assert!(content.contains("### business_services\n\n- `services\\user_service.py`"));
        assert!(content.contains("### utilities\n\n- `utils\\helper.py`"));
        assert!(content.contains("```mermaid"));
        assert!(content.contains("flowchart TD"));
    }

    #[test]
    fn classifies_markdown_as_documentation_and_json_as_configuration() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("README.md"), "# hi\n").unwrap();
            std::fs::write(root.join("tsconfig.json"), "{}").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_architecture_map_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("### documentation\n\n- `README.md`"));
        assert!(content.contains("### configuration\n\n- `tsconfig.json`"));
    }
}
