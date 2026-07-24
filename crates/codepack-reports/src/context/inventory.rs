//! [`Inventory`]: the aggregate file/byte/language summary built once per run,
//! ported from legacy `utils/inventory.py::collect_basic_inventory` — except it is
//! built from [`codepack_scanner::ExportPlan`]'s already-planned `included_files`
//! list, never from a fresh directory walk (this crate's scope boundary, `lib.rs`).
//!
//! `total_dirs` is a documented, narrower stand-in for legacy's `iter_project_dirs`:
//! legacy walks the copied tree and counts every directory, including empty ones.
//! `ExportPlan` carries no directory list at all (only files plus `skipped_dirs`, the
//! dirs pruned by ignore rules), so this counts the distinct directories that contain
//! at least one *included* file. An empty directory therefore never contributes here
//! — a real, accepted parity gap given the architectural boundary (S7 does not
//! re-walk), not an oversight.

use std::collections::BTreeSet;

use codepack_scanner::ExportPlan;

use crate::paths::{extension_key, file_name_of};

/// `LANGUAGE_BY_EXTENSION`, ported verbatim from legacy `constants.py`.
const LANGUAGE_BY_EXTENSION: &[(&str, &str)] = &[
    ("py", "Python"),
    ("pyw", "Python"),
    ("pyi", "Python"),
    ("js", "JavaScript"),
    ("jsx", "JavaScript / React"),
    ("mjs", "JavaScript"),
    ("cjs", "JavaScript"),
    ("ts", "TypeScript"),
    ("tsx", "TypeScript / React"),
    ("css", "CSS"),
    ("scss", "SCSS"),
    ("sass", "Sass"),
    ("less", "Less"),
    ("html", "HTML"),
    ("htm", "HTML"),
    ("vue", "Vue"),
    ("svelte", "Svelte"),
    ("astro", "Astro"),
    ("go", "Go"),
    ("rs", "Rust"),
    ("java", "Java"),
    ("kt", "Kotlin"),
    ("kts", "Kotlin"),
    ("cs", "C#"),
    ("cpp", "C++"),
    ("cxx", "C++"),
    ("cc", "C++"),
    ("c", "C"),
    ("h", "C/C++ Header"),
    ("hpp", "C++ Header"),
    ("rb", "Ruby"),
    ("php", "PHP"),
    ("swift", "Swift"),
    ("dart", "Dart"),
    ("sql", "SQL"),
    ("sh", "Shell"),
    ("bash", "Shell"),
    ("zsh", "Shell"),
    ("fish", "Shell"),
    ("ps1", "PowerShell"),
    ("bat", "Batch"),
    ("cmd", "Batch"),
    ("json", "JSON"),
    ("json5", "JSON5"),
    ("yaml", "YAML"),
    ("yml", "YAML"),
    ("toml", "TOML"),
    ("xml", "XML"),
    ("md", "Markdown"),
    ("markdown", "Markdown"),
    ("rst", "reStructuredText"),
    ("dockerfile", "Dockerfile"),
];

fn language_for_extension(extension: &str) -> Option<&'static str> {
    LANGUAGE_BY_EXTENSION
        .iter()
        .find(|(ext, _)| *ext == extension)
        .map(|(_, language)| *language)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryFile {
    pub relative_path: String,
    pub size: u64,
    pub extension: String,
    pub language: Option<&'static str>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtensionStat {
    pub extension: String,
    pub count: usize,
    pub total_size: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LanguageStat {
    pub language: String,
    pub count: usize,
    pub total_size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Inventory {
    pub files: Vec<InventoryFile>,
    pub total_size: u64,
    pub total_dirs: usize,
    /// Descending by file count, ties broken by extension name (deterministic;
    /// legacy's `Counter.most_common()` tie-break is insertion order, which Rust's
    /// `ExportPlan.included_files` order does not guarantee to reproduce identically).
    pub by_extension: Vec<ExtensionStat>,
    pub by_language: Vec<LanguageStat>,
}

impl Inventory {
    pub fn from_plan(plan: &ExportPlan) -> Self {
        let mut files = Vec::with_capacity(plan.included_files.len());
        let mut total_size = 0u64;
        let mut extension_counts: std::collections::HashMap<String, ExtensionStat> =
            std::collections::HashMap::new();
        let mut language_counts: std::collections::HashMap<String, LanguageStat> =
            std::collections::HashMap::new();
        let mut dirs: BTreeSet<String> = BTreeSet::new();

        for planned in &plan.included_files {
            let extension = extension_key(&planned.relative_path);
            let language = language_for_extension(&extension);

            total_size += planned.size;

            let stat = extension_counts
                .entry(extension.clone())
                .or_insert_with(|| ExtensionStat {
                    extension: extension.clone(),
                    count: 0,
                    total_size: 0,
                });
            stat.count += 1;
            stat.total_size += planned.size;

            if let Some(language) = language {
                let stat = language_counts
                    .entry(language.to_string())
                    .or_insert_with(|| LanguageStat {
                        language: language.to_string(),
                        count: 0,
                        total_size: 0,
                    });
                stat.count += 1;
                stat.total_size += planned.size;
            }

            if let Some(index) = planned.relative_path.rfind('\\') {
                dirs.insert(planned.relative_path[..index].to_string());
            }

            files.push(InventoryFile {
                relative_path: planned.relative_path.clone(),
                size: planned.size,
                extension,
                language,
            });
        }

        let mut by_extension: Vec<ExtensionStat> = extension_counts.into_values().collect();
        by_extension.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.extension.cmp(&b.extension))
        });

        let mut by_language: Vec<LanguageStat> = language_counts.into_values().collect();
        by_language.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.language.cmp(&b.language))
        });

        Self {
            files,
            total_size,
            total_dirs: dirs.len(),
            by_extension,
            by_language,
        }
    }
}

/// True when any included file's final path segment matches `predicate` (case
/// already lowered by the caller where relevant).
pub fn any_file_name(inventory: &Inventory, mut predicate: impl FnMut(&str) -> bool) -> bool {
    inventory
        .files
        .iter()
        .any(|file| predicate(&file_name_of(&file.relative_path).to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_core::CancellationToken;
    use codepack_scanner::{ExportIgnoreRules, ScanOptions, build_export_plan};

    fn build_plan(dir: &std::path::Path) -> ExportPlan {
        build_export_plan(
            dir,
            &ScanOptions::default(),
            &ExportIgnoreRules::default(),
            &codepack_scanner::no_safety_classification,
            &CancellationToken::new(),
        )
        .unwrap()
    }

    #[test]
    fn aggregates_extension_and_language_stats() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.py"), "print('hi')\n").unwrap();
        std::fs::write(dir.path().join("second.py"), "x = 1\n").unwrap();
        std::fs::write(dir.path().join("README.md"), "# hi\n").unwrap();

        let plan = build_plan(dir.path());
        let inventory = Inventory::from_plan(&plan);

        assert_eq!(inventory.files.len(), 3);
        assert_eq!(inventory.total_size, plan.summary.estimated_included_bytes);

        let python_ext = inventory
            .by_extension
            .iter()
            .find(|stat| stat.extension == "py")
            .unwrap();
        assert_eq!(python_ext.count, 2);

        let python_lang = inventory
            .by_language
            .iter()
            .find(|stat| stat.language == "Python")
            .unwrap();
        assert_eq!(python_lang.count, 2);
    }

    #[test]
    fn counts_directories_containing_included_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/utils")).unwrap();
        std::fs::write(dir.path().join("src/main.py"), "x = 1\n").unwrap();
        std::fs::write(dir.path().join("src/utils/helper.py"), "y = 2\n").unwrap();

        let plan = build_plan(dir.path());
        let inventory = Inventory::from_plan(&plan);

        assert_eq!(inventory.total_dirs, 2);
    }
}
