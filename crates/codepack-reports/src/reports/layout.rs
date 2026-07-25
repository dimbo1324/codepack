//! Shared directory-derivation helper for the Group E reports that classify
//! directories by name convention (`15_architecture_report.md`,
//! `19_frontend_report.md`, `20_backend_report.md`, `11_routes_and_pages.txt`). Legacy's
//! own `iter_project_dirs` walks the copied tree directly; this crate never re-walks
//! (scope boundary, `lib.rs`), so every ancestor directory of an already-included file
//! is derived from [`crate::context::Inventory`] instead — an empty directory
//! therefore never appears here, the same accepted parity gap already documented on
//! `Inventory.total_dirs`.

use std::collections::BTreeSet;

use crate::context::Inventory;

/// Width of the `=`/`-` rule that separates sections in a plain-text report.
///
/// Legacy fixed this at 100 columns and every `.txt` report it produced used it, so it
/// is part of what those artifacts look like rather than a styling choice. Previously
/// written as a bare `100` at each of the fifteen places a rule is drawn.
pub(crate) const SECTION_RULE_WIDTH: usize = 100;

/// Draws a section rule of [`SECTION_RULE_WIDTH`] columns using `character`.
pub(crate) fn section_rule(character: char) -> String {
    character.to_string().repeat(SECTION_RULE_WIDTH)
}

/// Legacy `SOURCE_CODE_EXTENSIONS` (`constants.py`): the extensions treated as source
/// code by every report that distinguishes code from data or documentation
/// (`08_code_metrics.txt`, `17_code_quality_report.md`,
/// `23_refactoring_opportunities.md`).
pub(crate) const SOURCE_CODE_EXTENSIONS: &[&str] = &[
    "astro", "c", "cc", "cpp", "cs", "css", "cxx", "dart", "go", "h", "hpp", "html", "htm", "java",
    "js", "jsx", "kt", "kts", "less", "mjs", "php", "py", "pyi", "pyw", "rb", "rs", "sass", "scss",
    "sh", "sql", "svelte", "ts", "tsx", "vue",
];

pub(crate) fn all_directories(inventory: &Inventory) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();
    for file in &inventory.files {
        let segments: Vec<&str> = file.relative_path.split('\\').collect();
        if segments.len() < 2 {
            continue;
        }
        let mut current = String::new();
        for segment in &segments[..segments.len() - 1] {
            if current.is_empty() {
                current = (*segment).to_string();
            } else {
                current.push('\\');
                current.push_str(segment);
            }
            dirs.insert(current.clone());
        }
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::InventoryFile;

    #[test]
    fn derives_every_ancestor_directory() {
        let inventory = Inventory {
            files: vec![InventoryFile {
                relative_path: "src\\components\\Button.tsx".to_string(),
                size: 10,
                extension: "tsx".to_string(),
                language: None,
            }],
            ..Default::default()
        };
        let dirs = all_directories(&inventory);
        assert!(dirs.contains("src"));
        assert!(dirs.contains("src\\components"));
    }
}
