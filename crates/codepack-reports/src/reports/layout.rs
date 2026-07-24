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

/// Legacy `SOURCE_CODE_EXTENSIONS` (`constants.py`), shared by the Group E reports
/// that need it (`17_code_quality_report.md`, `23_refactoring_opportunities.md`).
/// `08_code_metrics.txt` (Group A) carries its own private copy of this same
/// verbatim-ported list from an earlier pass; both copies are independently faithful
/// to the same fixed legacy constant, so the small duplication is left alone rather
/// than risk an unrelated edit to already-tested Group A code for this pass's sake.
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
