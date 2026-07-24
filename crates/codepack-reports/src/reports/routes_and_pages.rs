//! `11_routes_and_pages.txt`, ported from legacy
//! `reports/insights/routes_report.py::write_routes_and_pages_report`. A heuristic,
//! framework-agnostic map based purely on folder/file naming conventions — no import
//! graph is involved (unlike `16_key_files_report.md`/`23_refactoring_opportunities.md`,
//! legacy's own `routes_report.py` never imports `dependency_graph.py`).

use std::path::Path;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::paths::file_name_of;
use crate::plugin::ReportJob;
use crate::profile;
use crate::reports::layout::all_directories;

pub const JOB: ReportJob = ReportJob {
    filename: "11_routes_and_pages.txt",
    profiles: profile::ROUTES_AND_PAGES_TXT,
    description: "Heuristic frontend routes/pages/UI map based on folder and file names.",
    run: write_routes_and_pages_report,
};

const DIR_LIST_LIMIT: usize = 100;
const FILE_LIST_LIMIT: usize = 300;
const UI_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "vue", "svelte", "astro"];
const INTERESTING_DIR_KEYS: &[&str] = &[
    "pages",
    "routes",
    "app",
    "features",
    "components",
    "widgets",
    "layouts",
];

fn dir_key(directory: &str) -> Option<&'static str> {
    let lower_parts: Vec<String> = directory
        .split('\\')
        .map(|part| part.to_lowercase())
        .collect();
    INTERESTING_DIR_KEYS
        .iter()
        .find(|key| lower_parts.iter().any(|part| part == *key))
        .copied()
}

fn write_routes_and_pages_report(
    ctx: &ReportContext<'_>,
    output_file: &Path,
) -> Result<(), ReportError> {
    let directories = all_directories(ctx.inventory);
    let mut by_key: std::collections::BTreeMap<&'static str, Vec<String>> =
        std::collections::BTreeMap::new();
    for key in INTERESTING_DIR_KEYS {
        by_key.insert(key, Vec::new());
    }
    for directory in &directories {
        if let Some(key) = dir_key(directory) {
            // `by_key` was just pre-populated with every `INTERESTING_DIR_KEYS` entry
            // above, and `dir_key` only ever returns a member of that same slice.
            by_key
                .get_mut(key)
                .expect("by_key was pre-populated with every INTERESTING_DIR_KEYS entry")
                .push(directory.clone());
        }
    }
    for entries in by_key.values_mut() {
        entries.sort_by_key(|value| value.to_lowercase());
    }

    let mut route_like: Vec<&str> = Vec::new();
    let mut page_like: Vec<&str> = Vec::new();
    let mut component_like: Vec<&str> = Vec::new();

    for file in &ctx.inventory.files {
        if !UI_EXTENSIONS.contains(&file.extension.as_str()) {
            continue;
        }
        let lower_parts: Vec<String> = file
            .relative_path
            .split('\\')
            .map(|part| part.to_lowercase())
            .collect();
        let name_lower = file_name_of(&file.relative_path).to_lowercase();

        if lower_parts.iter().any(|part| part == "routes")
            || name_lower.contains("router")
            || name_lower.contains("route")
        {
            route_like.push(&file.relative_path);
        }
        if lower_parts.iter().any(|part| part == "components")
            || is_pascal_case_component_name(&file.extension, &file.relative_path)
        {
            component_like.push(&file.relative_path);
        }
        if lower_parts.iter().any(|part| part == "pages") || name_lower.contains("page") {
            page_like.push(&file.relative_path);
        }
    }
    route_like.sort_by_key(|value| value.to_lowercase());
    page_like.sort_by_key(|value| value.to_lowercase());
    component_like.sort_by_key(|value| value.to_lowercase());

    let mut out = String::new();
    out.push_str("=== Frontend Routes / Pages / UI Map ===\n");
    out.push_str(&format!("Generated: {}\n", ctx.plan.generated_at));
    out.push_str("This is a heuristic map based on folder and file names.\n");
    out.push_str(&"=".repeat(100));
    out.push_str("\n\n");

    out.push_str("--- Important UI directories ---\n");
    for key in INTERESTING_DIR_KEYS {
        out.push_str(&format!("\n{key}:\n"));
        let dirs = &by_key[key];
        if dirs.is_empty() {
            out.push_str("- not detected\n");
        } else {
            for directory in dirs.iter().take(DIR_LIST_LIMIT) {
                out.push_str(&format!("- {directory}\n"));
            }
        }
    }

    for (title, paths) in [
        ("Route-like files", &route_like),
        ("Page-like files", &page_like),
        ("Component-like files", &component_like),
    ] {
        out.push_str(&format!("\n--- {title} ---\n"));
        if paths.is_empty() {
            out.push_str("None detected.\n");
        } else {
            for path in paths.iter().take(FILE_LIST_LIMIT) {
                out.push_str(path);
                out.push('\n');
            }
            if paths.len() > FILE_LIST_LIMIT {
                out.push_str(&format!("... and {} more\n", paths.len() - FILE_LIST_LIMIT));
            }
        }
    }

    std::fs::write(output_file, out).map_err(|source| ReportError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

/// Legacy `re.match(r"^[A-Z].*\.(tsx|jsx|vue|svelte|astro)$", path.name)`: a
/// PascalCase-named file with one of those extensions (not `.ts`/`.js`, which lack a
/// component-template shape).
fn is_pascal_case_component_name(extension: &str, relative_path: &str) -> bool {
    let name = file_name_of(relative_path);
    matches!(extension, "tsx" | "jsx" | "vue" | "svelte" | "astro")
        && name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fixture;

    #[test]
    fn lists_important_directories_and_route_page_component_files() {
        let fixture = Fixture::new(|root| {
            std::fs::create_dir_all(root.join("src").join("pages")).unwrap();
            std::fs::create_dir_all(root.join("src").join("components")).unwrap();
            std::fs::write(
                root.join("src").join("pages").join("Home.tsx"),
                "export default function Home() {}\n",
            )
            .unwrap();
            std::fs::write(
                root.join("src").join("components").join("Button.tsx"),
                "export default function Button() {}\n",
            )
            .unwrap();
            std::fs::write(
                root.join("src").join("router.ts"),
                "export const routes = [];\n",
            )
            .unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_routes_and_pages_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.starts_with("=== Frontend Routes / Pages / UI Map ==="));
        assert!(content.contains("pages:\n- src\\pages"));
        assert!(content.contains("components:\n- src\\components"));
        assert!(content.contains("--- Route-like files ---\nsrc\\router.ts"));
        assert!(content.contains("--- Page-like files ---\nsrc\\pages\\Home.tsx"));
        assert!(content.contains("--- Component-like files ---"));
        assert!(content.contains("src\\components\\Button.tsx"));
    }

    #[test]
    fn reports_not_detected_and_none_detected_when_project_has_no_ui_code() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("main.py"), "x = 1\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_routes_and_pages_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("pages:\n- not detected"));
        assert!(content.contains("--- Route-like files ---\nNone detected."));
    }
}
